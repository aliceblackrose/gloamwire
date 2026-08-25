//! Gloamwire is an asynchronous Discord Gateway and REST API library.
//!
//! The crate intentionally starts small: it provides a reusable HTTP client,
//! core Discord models, snowflake handling, Gateway intents, and a maintained
//! Gateway WebSocket connection without imposing a bot framework or command
//! system on downstream applications.

#![deny(unsafe_code)]

pub mod error;
pub mod gateway;
pub mod http;
pub mod model;

pub use error::{DiscordApiError, DiscordValidationError, Error, Result};
pub use http::{Pagination, RestClient, RestClientBuilder, UploadFile, UploadSource};
