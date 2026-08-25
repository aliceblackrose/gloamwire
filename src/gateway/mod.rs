//! Discord Gateway WebSocket support.

mod connection;
mod event;
mod intents;

pub use connection::{GatewayConfig, GatewayConnection};
pub use event::{DispatchEvent, GatewayEvent};
pub use intents::GatewayIntents;
