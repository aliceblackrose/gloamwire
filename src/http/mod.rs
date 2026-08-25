//! Discord HTTP API support.

mod client;
mod models;

pub use client::RestClient;
pub use models::{GatewayBot, SessionStartLimit};
