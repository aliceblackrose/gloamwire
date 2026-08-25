//! Discord Gateway WebSocket support.

mod close;
mod connection;
mod event;
mod identify;
mod intents;
mod rate_limit;
mod session;
mod shard;

pub use close::{GatewayCloseCode, GatewayReconnectStrategy};
pub use connection::{GatewayConfig, GatewayConnection};
pub use event::{DispatchEvent, GatewayEvent};
pub use intents::GatewayIntents;
pub use session::GatewaySession;
pub use shard::{ShardCount, ShardEvent, ShardId, ShardManager, shard_for_guild};
