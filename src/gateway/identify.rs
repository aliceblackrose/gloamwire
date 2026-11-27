use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::Instant};

use crate::http::SessionStartLimit;

use super::{GatewayCoordinationFuture, GatewayIdentifyCoordinator, ShardId};

const IDENTIFY_BUCKET_WINDOW: Duration = Duration::from_secs(5);
const SESSION_START_WINDOW: Duration = Duration::from_secs(86_400);

#[derive(Debug, Clone)]
pub(crate) struct GatewayIdentifyLimiter {
    state: Arc<Mutex<IdentifyState>>,
}

#[derive(Debug)]
struct IdentifyState {
    total: u32,
    remaining: u32,
    reset_at: Instant,
    max_concurrency: u32,
    next_bucket_allowed: HashMap<u32, Instant>,
}

impl GatewayIdentifyLimiter {
    pub(crate) fn new(limit: &SessionStartLimit) -> Self {
        let now = Instant::now();
        let max_concurrency = limit.max_concurrency.max(1);
        let reset_after = Duration::from_millis(limit.reset_after);

        Self {
            state: Arc::new(Mutex::new(IdentifyState {
                total: limit.total,
                remaining: limit.remaining.min(limit.total),
                reset_at: now + reset_after,
                max_concurrency,
                next_bucket_allowed: HashMap::new(),
            })),
        }
    }

    async fn acquire(&self, shard_id: u32) -> super::GatewayCoordinationResult<()> {
        loop {
            let wait_until = {
                let mut state = self.state.lock().await;
                let now = Instant::now();

                if state.total == 0 {
                    return Err(super::GatewayCoordinationError::new(
                        "Discord reported zero Identify session capacity",
                    ));
                }
                if now >= state.reset_at {
                    state.remaining = state.total;
                    // reset_after describes the current window, not future quota windows.
                    state.reset_at = now + SESSION_START_WINDOW;
                }

                if state.remaining == 0 {
                    Some(state.reset_at)
                } else {
                    state
                        .next_bucket_allowed
                        .retain(|_, deadline| *deadline > now);
                    let bucket = shard_id % state.max_concurrency;
                    let next_allowed = state
                        .next_bucket_allowed
                        .get(&bucket)
                        .copied()
                        .unwrap_or(now);

                    if next_allowed > now {
                        Some(next_allowed)
                    } else {
                        state.remaining -= 1;
                        state
                            .next_bucket_allowed
                            .insert(bucket, now + IDENTIFY_BUCKET_WINDOW);

                        #[cfg(feature = "tracing")]
                        tracing::trace!(
                            target: "gloamwire::gateway",
                            shard_id,
                            remaining = state.remaining,
                            "reserved Discord Gateway Identify session"
                        );

                        None
                    }
                }
            };

            match wait_until {
                Some(deadline) => {
                    #[cfg(feature = "tracing")]
                    {
                        let wait = deadline.saturating_duration_since(Instant::now());
                        tracing::debug!(
                            target: "gloamwire::gateway",
                            shard_id,
                            wait_ms = %wait.as_millis(),
                            "waiting for Discord Gateway Identify capacity"
                        );
                    }
                    tokio::time::sleep_until(deadline).await;
                }
                None => return Ok(()),
            }
        }
    }
}

impl GatewayIdentifyCoordinator for GatewayIdentifyLimiter {
    fn acquire_identify(&self, shard_id: ShardId) -> GatewayCoordinationFuture<'_, ()> {
        Box::pin(async move { self.acquire(shard_id.get()).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn first_reset_does_not_reuse_remaining_window_duration() {
        let limiter = GatewayIdentifyLimiter::new(&SessionStartLimit {
            total: 1,
            remaining: 0,
            reset_after: 0,
            max_concurrency: u32::MAX,
        });
        limiter.acquire(0).await.expect("first reset");
        let state = limiter.state.lock().await;
        assert_eq!(state.remaining, 0);
        assert!(
            state.reset_at.saturating_duration_since(Instant::now()) > Duration::from_secs(86_390)
        );
        assert_eq!(state.next_bucket_allowed.len(), 1);
    }

    #[tokio::test]
    async fn zero_total_quota_fails_without_busy_loop() {
        let limiter = GatewayIdentifyLimiter::new(&SessionStartLimit {
            total: 0,
            remaining: 0,
            reset_after: 0,
            max_concurrency: 1,
        });
        assert!(limiter.acquire(0).await.is_err());
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_identify_waiter_does_not_consume_quota() {
        let limiter = GatewayIdentifyLimiter::new(&SessionStartLimit {
            total: 10,
            remaining: 10,
            reset_after: 60_000,
            max_concurrency: 1,
        });
        limiter.acquire(0).await.unwrap();
        let waiting = limiter.clone();
        let task = tokio::spawn(async move { waiting.acquire(1).await });
        tokio::task::yield_now().await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(limiter.state.lock().await.remaining, 9);
    }
}
