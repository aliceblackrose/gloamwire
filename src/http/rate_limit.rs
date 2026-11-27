use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};

use reqwest::{StatusCode, header::HeaderMap};
use tokio::{
    sync::{Mutex, OwnedMutexGuard},
    time::Instant,
};

use super::route::Route;

const GLOBAL_REQUEST_LIMIT: usize = 50;
const GLOBAL_WINDOW: Duration = Duration::from_secs(1);
const MAX_BUCKETS: usize = 4096;
const GATE_STRIPES: usize = 256;

#[derive(Debug, Default)]
pub(crate) struct RateLimiter {
    state: std::sync::Mutex<RateLimitState>,
    shared: Arc<SharedRateLimits>,
}

/// Shared across authentication contexts. Serialize attempts by major resource,
/// not by mutable server bucket hash; release the gate before any cooldown sleep.
/// This deliberately sacrifices concurrency within one major resource.
#[derive(Debug)]
pub(crate) struct SharedRateLimits {
    gates: Vec<Arc<Mutex<()>>>,
    cooldowns: std::sync::Mutex<SharedCooldowns>,
}

#[derive(Debug, Default)]
struct SharedCooldowns {
    entries: HashMap<String, Instant>,
    overflow_until: Option<Instant>,
}

impl Default for SharedRateLimits {
    fn default() -> Self {
        Self {
            gates: (0..GATE_STRIPES)
                .map(|_| Arc::new(Mutex::new(())))
                .collect(),
            cooldowns: std::sync::Mutex::default(),
        }
    }
}

impl SharedRateLimits {
    fn cooldown(&self, major: &str, now: Instant) -> Option<Instant> {
        let mut cooldowns = self.cooldowns.lock().unwrap_or_else(|e| e.into_inner());
        cooldowns.entries.retain(|_, deadline| *deadline > now);
        let overflow = cooldowns.overflow_until.filter(|deadline| *deadline > now);
        match cooldowns.entries.get(major).copied() {
            Some(deadline) => later(overflow, deadline),
            None => overflow,
        }
    }

    fn update(&self, major: &str, deadline: Instant) {
        let mut cooldowns = self.cooldowns.lock().unwrap_or_else(|e| e.into_inner());
        cooldowns.entries.retain(|_, until| *until > Instant::now());
        if cooldowns.entries.len() >= MAX_BUCKETS && !cooldowns.entries.contains_key(major) {
            cooldowns.overflow_until = later(cooldowns.overflow_until, deadline);
        } else {
            let until = cooldowns
                .entries
                .entry(major.to_owned())
                .or_insert(deadline);
            *until = (*until).max(deadline);
        }
    }

    fn gate(&self, major: &str) -> Arc<Mutex<()>> {
        let index = super::route::credential_fingerprint(major.as_bytes()) as usize % GATE_STRIPES;
        Arc::clone(&self.gates[index])
    }
}

#[derive(Debug, Default)]
struct RateLimitState {
    global_requests: VecDeque<Instant>,
    global_until: Option<Instant>,
    ambiguous_429s: usize,
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
    cooldown_until: Option<Instant>,
}

/// Cancellation-safe provisional admission barrier for an unclassified 429.
pub(crate) struct Ambiguous429<'a> {
    limiter: &'a RateLimiter,
    classified: bool,
}

impl Ambiguous429<'_> {
    pub(crate) fn classified(mut self) {
        self.classified = true;
    }
}

impl Drop for Ambiguous429<'_> {
    fn drop(&mut self) {
        let mut state = self.limiter.state.lock().unwrap_or_else(|e| e.into_inner());
        state.ambiguous_429s -= 1;
        if !self.classified {
            state.global_until = later(state.global_until, Instant::now() + Duration::from_secs(1));
        }
    }
}

impl RateLimiter {
    pub(crate) fn begin_ambiguous_429(&self) -> Ambiguous429<'_> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ambiguous_429s += 1;
        Ambiguous429 {
            limiter: self,
            classified: false,
        }
    }

    pub(crate) fn with_shared(shared: Arc<SharedRateLimits>) -> Self {
        Self {
            shared,
            ..Self::default()
        }
    }

    pub(crate) async fn reserve(&self, route: &Route) -> OwnedMutexGuard<()> {
        loop {
            let guard = self.shared.gate(route.resource_major()).lock_owned().await;
            match self.try_acquire(route) {
                None => return guard,
                Some(deadline) => {
                    drop(guard);
                    tokio::time::sleep_until(deadline).await;
                }
            }
        }
    }

    #[cfg(test)]
    async fn acquire(&self, route: &Route) {
        drop(self.reserve(route).await);
    }

    fn try_acquire(&self, route: &Route) -> Option<Instant> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        while state
            .global_requests
            .front()
            .is_some_and(|instant| now.duration_since(*instant) >= GLOBAL_WINDOW)
        {
            state.global_requests.pop_front();
        }

        if state.buckets.len() >= MAX_BUCKETS {
            state.buckets.retain(|_, bucket| {
                bucket.reset_at > now || bucket.cooldown_until.is_some_and(|until| until > now)
            });
        }

        let mut wait_until = state.global_until.filter(|deadline| *deadline > now);
        if state.ambiguous_429s > 0 {
            wait_until = later(wait_until, now + Duration::from_millis(50));
        }
        if let Some(deadline) = self.shared.cooldown(route.resource_major(), now) {
            wait_until = later(wait_until, deadline);
        }

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

        if let Some(bucket) = state.buckets.get(&bucket_key) {
            if bucket.remaining == 0 && bucket.reset_at > now {
                wait_until = later(wait_until, bucket.reset_at);
            }
            if let Some(deadline) = bucket.cooldown_until.filter(|deadline| *deadline > now) {
                wait_until = later(wait_until, deadline);
            }
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
    }

    /// A limiter with a live bucket/global deadline must not be evicted.
    pub(crate) fn has_live_cooldown(&self, now: Instant) -> bool {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.ambiguous_429s > 0
            || state
                .global_requests
                .back()
                .is_some_and(|time| now.saturating_duration_since(*time) < GLOBAL_WINDOW)
            || state.global_until.is_some_and(|until| until > now)
            || state.buckets.values().any(|bucket| {
                bucket.reset_at > now || bucket.cooldown_until.is_some_and(|until| until > now)
            })
    }

    pub(crate) fn update(
        &self,
        route: &Route,
        status: StatusCode,
        headers: &HeaderMap,
        retry_after: Option<Duration>,
        global: bool,
    ) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let route_identity = route.identity();
        let bucket_hash = header_str(headers, "x-ratelimit-bucket")
            .filter(|hash| hash.len() <= 128)
            .map(str::to_owned);

        #[cfg(feature = "tracing")]
        tracing::trace!(
            target: "gloamwire::http",
            route = %route_identity,
            status = status.as_u16(),
            global,
            "updated Discord REST rate-limit state"
        );

        let previous_key = state
            .route_buckets
            .get(&route_identity)
            .map(|hash| bucket_key(hash, route.major()))
            .unwrap_or_else(|| provisional_bucket_key(&route_identity, route.major()));
        if let Some(hash) = &bucket_hash {
            let next_key = bucket_key(hash, route.major());
            if previous_key != next_key
                && let Some(previous) = state.buckets.get(&previous_key).copied()
                && (previous.reset_at > now
                    || previous.cooldown_until.is_some_and(|until| until > now))
            {
                if state.buckets.len() >= MAX_BUCKETS && !state.buckets.contains_key(&next_key) {
                    let deadline = previous
                        .cooldown_until
                        .map_or(previous.reset_at, |until| until.max(previous.reset_at));
                    state.global_until = later(state.global_until, deadline);
                } else {
                    let next = state.buckets.entry(next_key).or_insert(previous);
                    next.remaining = next.remaining.min(previous.remaining);
                    next.reset_at = next.reset_at.max(previous.reset_at);
                    if let Some(until) = previous.cooldown_until {
                        next.cooldown_until = later(next.cooldown_until, until);
                    }
                }
            }
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

            if header_str(headers, "x-ratelimit-scope") == Some("shared") {
                self.shared.update(route.resource_major(), deadline);
            }
            if global
                || header_str(headers, "x-ratelimit-global") == Some("true")
                || header_str(headers, "x-ratelimit-scope") == Some("global")
            {
                state.global_until = later(state.global_until, deadline);
            } else if state.buckets.len() >= MAX_BUCKETS && !state.buckets.contains_key(&bucket_key)
            {
                state.global_until = later(state.global_until, deadline);
            } else {
                let bucket = state.buckets.entry(bucket_key).or_insert(BucketState {
                    remaining: 0,
                    reset_at: deadline,
                    cooldown_until: Some(deadline),
                });
                bucket.remaining = 0;
                bucket.reset_at = bucket.reset_at.max(deadline);
                bucket.cooldown_until = later(bucket.cooldown_until, deadline);
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
            if state.buckets.len() >= MAX_BUCKETS && !state.buckets.contains_key(&bucket_key) {
                state.global_until = later(state.global_until, reset_at);
                return;
            }
            let bucket = state.buckets.entry(bucket_key).or_insert(BucketState {
                remaining,
                reset_at,
                cooldown_until: None,
            });
            if bucket.cooldown_until.is_some_and(|until| until > now) {
                // A response must not erase a separately recorded 429 deadline.
                bucket.reset_at = bucket.reset_at.max(reset_at);
            } else {
                *bucket = BucketState {
                    remaining,
                    reset_at,
                    cooldown_until: None,
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

    #[test]
    fn bucket_remapping_preserves_cooldown_but_success_restores_ordinary_capacity() {
        let limiter = RateLimiter::default();
        let route = Route::current_user();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-bucket", "old".parse().unwrap());
        headers.insert("x-ratelimit-remaining", "1".parse().unwrap());
        headers.insert("x-ratelimit-reset-after", "10".parse().unwrap());
        limiter.update(&route, StatusCode::OK, &headers, None, false);
        assert!(limiter.try_acquire(&route).is_none());
        limiter.update(&route, StatusCode::OK, &headers, None, false);
        assert!(limiter.try_acquire(&route).is_none());
        limiter.update(
            &route,
            StatusCode::TOO_MANY_REQUESTS,
            &headers,
            Some(Duration::from_secs(10)),
            false,
        );
        headers.insert("x-ratelimit-bucket", "new".parse().unwrap());
        limiter.update(&route, StatusCode::OK, &headers, None, false);
        assert!(limiter.try_acquire(&route).is_some());
    }

    #[test]
    fn ambiguous_global_rate_limit_blocks_other_routes_until_classified() {
        let limiter = RateLimiter::default();
        let guard = limiter.begin_ambiguous_429();
        assert!(limiter.try_acquire(&Route::gateway_bot()).is_some());
        guard.classified();
        assert!(limiter.try_acquire(&Route::gateway_bot()).is_none());
        drop(limiter.begin_ambiguous_429());
        assert!(limiter.try_acquire(&Route::current_user()).is_some());
    }

    #[test]
    fn active_cooldown_prevents_limiter_eviction() {
        let limiter = RateLimiter::default();
        let route = Route::current_user();
        limiter.update(
            &route,
            StatusCode::TOO_MANY_REQUESTS,
            &HeaderMap::new(),
            Some(Duration::from_secs(30)),
            true,
        );
        let now = tokio::time::Instant::now();
        assert!(limiter.has_live_cooldown(now));
        assert!(!limiter.has_live_cooldown(now + Duration::from_secs(31)));
    }

    #[tokio::test]
    async fn global_scope_header_applies_before_body() {
        let limiter = RateLimiter::default();
        let route = Route::current_user();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-scope", "global".parse().unwrap());
        limiter.update(
            &route,
            StatusCode::TOO_MANY_REQUESTS,
            &headers,
            Some(Duration::from_secs(5)),
            false,
        );
        assert!(limiter.try_acquire(&Route::gateway_bot()).is_some());
        assert!(limiter.state.lock().unwrap().global_until.is_some());
    }

    #[tokio::test]
    async fn global_429_delays_following_request() {
        let limiter = RateLimiter::default();
        let route = Route::current_user();

        limiter.update(
            &route,
            StatusCode::TOO_MANY_REQUESTS,
            &HeaderMap::new(),
            Some(Duration::from_millis(20)),
            true,
        );

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
        limiter.update(&route, StatusCode::OK, &headers, None, false);
        limiter.update(
            &route,
            StatusCode::TOO_MANY_REQUESTS,
            &HeaderMap::new(),
            Some(Duration::from_secs(10)),
            false,
        );
        limiter.update(&route, StatusCode::OK, &headers, None, false);
        let state = limiter.state.lock().unwrap();
        assert_eq!(
            state.buckets[&bucket_key("bucket", route.major())].remaining,
            0
        );
    }

    #[tokio::test]
    async fn cooled_resource_does_not_block_other_majors() {
        let shared = std::sync::Arc::new(super::SharedRateLimits::default());
        let first = RateLimiter::with_shared(shared.clone());
        let second = RateLimiter::with_shared(shared);
        let a = Route::create_message(crate::model::ChannelId::new(1));
        let b = Route::create_message(crate::model::ChannelId::new(2));
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-scope", "shared".parse().unwrap());
        first.update(
            &a,
            StatusCode::TOO_MANY_REQUESTS,
            &headers,
            Some(Duration::from_secs(5)),
            false,
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(20), second.reserve(&a))
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), second.reserve(&b))
                .await
                .is_ok()
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
            limiter.update(route, StatusCode::OK, &headers, None, false);
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

#[cfg(test)]
mod bounded_state_tests {
    use super::*;

    #[test]
    fn shared_cooldowns_are_bounded_without_discarding_deadlines() {
        let shared = SharedRateLimits::default();
        let now = Instant::now();
        let deadline = now + Duration::from_secs(60);
        for index in 0..=MAX_BUCKETS {
            shared.update(&index.to_string(), deadline);
        }
        assert_eq!(shared.cooldowns.lock().unwrap().entries.len(), MAX_BUCKETS);
        assert_eq!(shared.cooldown("untracked", now), Some(deadline));
    }
}
