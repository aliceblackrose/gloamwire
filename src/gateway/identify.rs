use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::Instant};

use crate::http::SessionStartLimit;

use super::{GatewayCoordinationFuture, GatewayIdentifyCoordinator, ShardId};

const IDENTIFY_BUCKET_WINDOW: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub(crate) struct GatewayIdentifyLimiter {
    state: Arc<Mutex<IdentifyState>>,
}

#[derive(Debug)]
struct IdentifyState {
    total: u32,
    remaining: u32,
    reset_after: Duration,
    reset_at: Instant,
    next_bucket_allowed: Vec<Instant>,
}

impl GatewayIdentifyLimiter {
    pub(crate) fn new(limit: &SessionStartLimit) -> Self {
        let now = Instant::now();
        let max_concurrency = limit.max_concurrency.max(1) as usize;
        let reset_after = Duration::from_millis(limit.reset_after);

        Self {
            state: Arc::new(Mutex::new(IdentifyState {
                total: limit.total,
                remaining: limit.remaining,
                reset_after,
                reset_at: now + reset_after,
                next_bucket_allowed: vec![now; max_concurrency],
            })),
        }
    }

    async fn acquire(&self, shard_id: u32) {
        loop {
            let wait_until = {
                let mut state = self.state.lock().await;
                let now = Instant::now();

                if now >= state.reset_at {
                    state.remaining = state.total;
                    state.reset_at = now + state.reset_after;
                }

                if state.remaining == 0 {
                    Some(state.reset_at)
                } else {
                    let bucket = shard_id as usize % state.next_bucket_allowed.len();
                    let next_allowed = state.next_bucket_allowed[bucket];

                    if next_allowed > now {
                        Some(next_allowed)
                    } else {
                        state.remaining -= 1;
                        state.next_bucket_allowed[bucket] = now + IDENTIFY_BUCKET_WINDOW;

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
                None => return,
            }
        }
    }
}

impl GatewayIdentifyCoordinator for GatewayIdentifyLimiter {
    fn acquire_identify(&self, shard_id: ShardId) -> GatewayCoordinationFuture<'_, ()> {
        Box::pin(async move {
            self.acquire(shard_id.get()).await;
            Ok(())
        })
    }
}
