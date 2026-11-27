use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Weak},
    time::Duration,
};

use reqwest::{StatusCode, header::HeaderMap};
use tokio::{
    sync::{Mutex, OwnedMutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Instant,
};

use super::route::Route;

const GLOBAL_REQUEST_LIMIT: usize = 50;
const GLOBAL_WINDOW: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub(crate) struct RateLimiter {
    state: Mutex<RateLimitState>,
    discovery: RwLock<()>,
    gates: Mutex<HashMap<RateLimitBucketKey, Weak<Mutex<()>>>>,
}

/// Holds a single-flight reservation until the HTTP attempt completes or is
/// cancelled. Unknown routes are discovered exclusively, since they may share
/// a bucket with a previously learned route.
pub(crate) struct Reservation<'a> {
    _read: Option<RwLockReadGuard<'a, ()>>,
    _write: Option<RwLockWriteGuard<'a, ()>>,
    _bucket: Option<OwnedMutexGuard<()>>,
}

#[derive(Debug, Default)]
struct RateLimitState {
    global_requests: VecDeque<Instant>,
    global_until: Option<Instant>,
    route_buckets: HashMap<String, String>,
    buckets: HashMap<RateLimitBucketKey, BucketState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RateLimitBucketKey {
    Discord {
        hash: String,
        major: String,
    },
    Provisional {
        route_identity: String,
        major: String,
    },
}

#[derive(Debug, Clone, Copy)]
struct BucketState {
    remaining: u32,
    reset_at: Instant,
}

impl RateLimiter {
    pub(crate) async fn reserve(&self, route: &Route) -> Reservation<'_> {
        let read = self.discovery.read().await;
        let key = {
            let mut state = self.state.lock().await;
            let now = Instant::now();
            if state.buckets.len() > 4096 {
                state.buckets.retain(|_, bucket| {
                    now.saturating_duration_since(bucket.reset_at) < Duration::from_secs(60)
                });
            }
            state
                .route_buckets
                .get(&route.identity())
                .map(|hash| bucket_key(hash, route.major()))
        };
        let reservation = if let Some(key) = key {
            let gate = {
                let mut gates = self.gates.lock().await;
                gates.retain(|_, gate| gate.strong_count() > 0);
                if let Some(gate) = gates.get(&key).and_then(Weak::upgrade) {
                    gate
                } else {
                    let gate = Arc::new(Mutex::new(()));
                    gates.insert(key, Arc::downgrade(&gate));
                    gate
                }
            };
            Reservation {
                _read: Some(read),
                _write: None,
                _bucket: Some(gate.lock_owned().await),
            }
        } else {
            drop(read);
            Reservation {
                _read: None,
                _write: Some(self.discovery.write().await),
                _bucket: None,
            }
        };
        self.acquire(route).await;
        reservation
    }

    async fn acquire(&self, route: &Route) {
        loop {
            let wait_until = {
                let mut state = self.state.lock().await;
                let now = Instant::now();

                while state
                    .global_requests
                    .front()
                    .is_some_and(|instant| now.duration_since(*instant) >= GLOBAL_WINDOW)
                {
                    state.global_requests.pop_front();
                }

                let mut wait_until = state.global_until.filter(|deadline| *deadline > now);

                if !route.global_exempt()
                    && state.global_requests.len() >= GLOBAL_REQUEST_LIMIT
                    && let Some(oldest) = state.global_requests.front()
                {
                    wait_until = later(wait_until, *oldest + GLOBAL_WINDOW);
                }

                let route_identity = route.identity();
                let bucket_key = state
                    .route_buckets
                    .get(&route_identity)
                    .map(|hash| bucket_key(hash, route.major()))
                    .unwrap_or_else(|| provisional_bucket_key(&route_identity, route.major()));

                if let Some(bucket) = state.buckets.get(&bucket_key)
                    && bucket.remaining == 0
                    && bucket.reset_at > now
                {
                    wait_until = later(wait_until, bucket.reset_at);
                }

                if wait_until.is_none() {
                    if !route.global_exempt() {
                        state.global_requests.push_back(now);
                    }
                    if let Some(bucket) = state.buckets.get_mut(&bucket_key)
                        && bucket.reset_at > now
                        && bucket.remaining > 0
                    {
                        bucket.remaining -= 1;
                    }
                }

                wait_until
            };

            match wait_until {
                Some(deadline) => {
                    #[cfg(feature = "tracing")]
                    {
                        let wait = deadline.saturating_duration_since(Instant::now());
                        tracing::debug!(
                            target: "gloamwire::http",
                            route = %route.identity(),
                            wait_ms = %wait.as_millis(),
                            "waiting for Discord REST rate-limit capacity"
                        );
                    }
                    tokio::time::sleep_until(deadline).await;
                }
                None => return,
            }
        }
    }

    pub(crate) async fn update(
        &self,
        route: &Route,
        status: StatusCode,
        headers: &HeaderMap,
        retry_after: Option<Duration>,
        global: bool,
    ) {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let route_identity = route.identity();
        let bucket_hash = header_str(headers, "x-ratelimit-bucket").map(str::to_owned);

        #[cfg(feature = "tracing")]
        tracing::trace!(
            target: "gloamwire::http",
            route = %route_identity,
            status = status.as_u16(),
            global,
            "updated Discord REST rate-limit state"
        );

        if let Some(hash) = &bucket_hash {
            state
                .route_buckets
                .insert(route_identity.clone(), hash.clone());
        }

        let bucket_key = bucket_hash
            .as_deref()
            .or_else(|| state.route_buckets.get(&route_identity).map(String::as_str))
            .map(|hash| bucket_key(hash, route.major()))
            .unwrap_or_else(|| provisional_bucket_key(&route_identity, route.major()));

        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = retry_after.unwrap_or(Duration::from_secs(1));
            let deadline = now + retry_after.min(Duration::from_secs(86_400));

            #[cfg(feature = "tracing")]
            tracing::debug!(
                target: "gloamwire::http",
                route = %route_identity,
                retry_after_ms = %retry_after.as_millis(),
                global,
                "Discord REST rate limit applied"
            );

            if global || header_str(headers, "x-ratelimit-global").is_some() {
                state.global_until = later(state.global_until, deadline);
            } else {
                let bucket = state.buckets.entry(bucket_key).or_insert(BucketState {
                    remaining: 0,
                    reset_at: deadline,
                });
                bucket.remaining = 0;
                bucket.reset_at = bucket.reset_at.max(deadline);
            }
            return;
        }

        let remaining = header_str(headers, "x-ratelimit-remaining")
            .and_then(|value| value.parse::<u32>().ok());
        let reset_after = header_str(headers, "x-ratelimit-reset-after")
            .and_then(|value| value.parse::<f64>().ok())
            .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
            .map(|duration| duration.min(Duration::from_secs(86_400)));

        if let (Some(remaining), Some(reset_after)) = (remaining, reset_after) {
            let reset_at = now + reset_after;
            let bucket = state.buckets.entry(bucket_key).or_insert(BucketState {
                remaining,
                reset_at,
            });
            if bucket.remaining == 0 && bucket.reset_at > now {
                // Preserve a live 429 cooldown. Single-flight reservations
                // prevent older successful responses from racing newer ones.
                bucket.reset_at = bucket.reset_at.max(reset_at);
            } else {
                *bucket = BucketState {
                    remaining,
                    reset_at,
                };
            }
        }
    }
}

fn later(current: Option<Instant>, candidate: Instant) -> Option<Instant> {
    Some(current.map_or(candidate, |current| current.max(candidate)))
}

fn bucket_key(hash: &str, major: &str) -> RateLimitBucketKey {
    RateLimitBucketKey::Discord {
        hash: hash.to_owned(),
        major: major.to_owned(),
    }
}

fn provisional_bucket_key(route_identity: &str, major: &str) -> RateLimitBucketKey {
    RateLimitBucketKey::Provisional {
        route_identity: route_identity.to_owned(),
        major: major.to_owned(),
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use proptest::prelude::*;
    use reqwest::{StatusCode, header::HeaderMap};

    use super::{RateLimiter, bucket_key, provisional_bucket_key};
    use crate::http::route::Route;

    #[tokio::test]
    async fn global_429_delays_following_request() {
        let limiter = RateLimiter::default();
        let route = Route::current_user();

        limiter
            .update(
                &route,
                StatusCode::TOO_MANY_REQUESTS,
                &HeaderMap::new(),
                Some(Duration::from_millis(20)),
                true,
            )
            .await;

        let started = tokio::time::Instant::now();
        limiter.acquire(&route).await;
        assert!(started.elapsed() >= Duration::from_millis(15));
    }

    #[tokio::test]
    async fn stale_response_and_headerless_429_do_not_restore_capacity() {
        let limiter = RateLimiter::default();
        let route = Route::current_user();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-bucket", "bucket".parse().unwrap());
        headers.insert("x-ratelimit-remaining", "1".parse().unwrap());
        headers.insert("x-ratelimit-reset-after", "10".parse().unwrap());
        limiter
            .update(&route, StatusCode::OK, &headers, None, false)
            .await;
        limiter
            .update(
                &route,
                StatusCode::TOO_MANY_REQUESTS,
                &HeaderMap::new(),
                Some(Duration::from_secs(10)),
                false,
            )
            .await;
        limiter
            .update(&route, StatusCode::OK, &headers, None, false)
            .await;
        let state = limiter.state.lock().await;
        assert_eq!(
            state.buckets[&bucket_key("bucket", route.major())].remaining,
            0
        );
    }

    #[tokio::test]
    async fn learned_bucket_aliases_are_single_flight() {
        let limiter = RateLimiter::default();
        let first = Route::current_user();
        let second = Route::gateway_bot();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-bucket", "shared".parse().unwrap());
        headers.insert("x-ratelimit-remaining", "5".parse().unwrap());
        headers.insert("x-ratelimit-reset-after", "10".parse().unwrap());
        for route in [&first, &second] {
            limiter
                .update(route, StatusCode::OK, &headers, None, false)
                .await;
        }
        let held = limiter.reserve(&first).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(10), limiter.reserve(&second))
                .await
                .is_err()
        );
        drop(held);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), limiter.reserve(&second))
                .await
                .is_ok()
        );
    }

    proptest! {
        #[test]
        fn discord_bucket_keys_preserve_component_boundaries(
            hash_a in any::<String>(),
            major_a in any::<String>(),
            hash_b in any::<String>(),
            major_b in any::<String>(),
        ) {
            let same_components = hash_a == hash_b && major_a == major_b;
            prop_assert_eq!(
                bucket_key(&hash_a, &major_a) == bucket_key(&hash_b, &major_b),
                same_components
            );
        }

        #[test]
        fn provisional_keys_never_alias_discord_bucket_keys(
            route_identity in any::<String>(),
            major in any::<String>(),
            hash in any::<String>(),
        ) {
            prop_assert_ne!(
                provisional_bucket_key(&route_identity, &major),
                bucket_key(&hash, &major)
            );
        }
    }
}
