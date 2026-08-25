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
    pub(crate) async fn acquire(&self, priority: OutboundPriority) {
        loop {
            let wait_until = {
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
                    None
                } else {
                    events.front().map(|oldest| *oldest + WINDOW)
                }
            };

            match wait_until {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => return,
            }
        }
    }
}
