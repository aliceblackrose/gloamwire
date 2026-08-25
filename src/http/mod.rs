//! Discord HTTP API support.

mod client;
mod models;
mod rate_limit;
mod response;
mod route;

pub use client::{RestClient, RestClientBuilder};
pub use models::{GatewayBot, SessionStartLimit};
pub use response::HttpResponse;
