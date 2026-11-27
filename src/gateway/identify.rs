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
                    state.next_bucket_allowed.retain(|_, deadline| *deadline > now);
                    let bucket = shard_id % state.max_concurrency;
                    let next_allowed = state.next_bucket_allowed.get(&bucket).copied().unwrap_or(now);

                    if next_allowed > now {
                        Some(next_allowed)
                    } else {
                        state.remaining -= 1;
                        state.next_bucket_allowed.insert(bucket, now + IDENTIFY_BUCKET_WINDOW);

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
        Box::pin(async move {
            self.acquire(shard_id.get()).await
        })
    }
}
