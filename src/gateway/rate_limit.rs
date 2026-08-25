use std::{collections::VecDeque, time::Duration};

use tokio::{sync::Mutex, time::Instant};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_EVENTS: usize = 120;
const HEARTBEAT_RESERVE: usize = 5;
const NORMAL_EVENT_LIMIT: usize = MAX_EVENTS - HEARTBEAT_RESERVE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutboundPriority {
    Normal,
    Heartbeat,
}

#[derive(Debug, Default)]
pub(crate) struct GatewayRateLimiter {
    events: Mutex<VecDeque<Instant>>,
}

impl GatewayRateLimiter {
    /// Acquires an outbound slot, waiting when necessary.
    ///
    /// This is used only by connection-lifecycle traffic and heartbeats. Public
    /// send events use [`Self::try_acquire`] so waiting cannot prevent callers
    /// from polling the Gateway receive/heartbeat loop.
    pub(crate) async fn acquire(&self, priority: OutboundPriority) {
        loop {
            match self.try_acquire(priority).await {
                Some(retry_after) => tokio::time::sleep(retry_after).await,
                None => return,
            }
        }
    }

    /// Attempts to reserve an outbound slot without sleeping.
    ///
    /// Returns the duration until a slot is expected to become available when
    /// the per-connection window is full.
    pub(crate) async fn try_acquire(&self, priority: OutboundPriority) -> Option<Duration> {
        let mut events = self.events.lock().await;
        let now = Instant::now();

        while events
            .front()
            .is_some_and(|instant| now.duration_since(*instant) >= WINDOW)
        {
            events.pop_front();
        }

        let limit = match priority {
            OutboundPriority::Normal => NORMAL_EVENT_LIMIT,
            OutboundPriority::Heartbeat => MAX_EVENTS,
        };

        if events.len() < limit {
            events.push_back(now);
            return None;
        }

        events
            .front()
            .map(|oldest| (*oldest + WINDOW).saturating_duration_since(now))
    }
}

#[cfg(test)]
mod tests {
    use super::{GatewayRateLimiter, NORMAL_EVENT_LIMIT, OutboundPriority};

    #[tokio::test]
    async fn normal_sends_preserve_heartbeat_capacity() {
        let limiter = GatewayRateLimiter::default();

        for _ in 0..NORMAL_EVENT_LIMIT {
            assert!(limiter.try_acquire(OutboundPriority::Normal).await.is_none());
        }

        assert!(limiter.try_acquire(OutboundPriority::Normal).await.is_some());
        assert!(limiter.try_acquire(OutboundPriority::Heartbeat).await.is_none());
    }
}
