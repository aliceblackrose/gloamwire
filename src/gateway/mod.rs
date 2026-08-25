//! Discord Gateway WebSocket support.

mod close;
mod connection;
mod event;
mod intents;
mod session;

pub use close::{GatewayCloseCode, GatewayReconnectStrategy};
pub use connection::{GatewayConfig, GatewayConnection};
pub use event::{DispatchEvent, GatewayEvent};
pub use intents::GatewayIntents;
pub use session::GatewaySession;
