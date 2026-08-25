//! Discord HTTP API support.

mod client;
mod models;
mod rate_limit;
mod route;

pub use client::RestClient;
pub use models::{GatewayBot, SessionStartLimit};
