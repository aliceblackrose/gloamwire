use std::time::Duration;

use reqwest::StatusCode;
use thiserror::Error;

use crate::gateway::GatewayCloseCode;

/// A result returned by Gloamwire operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the REST and Gateway clients.
#[derive(Debug, Error)]
pub enum Error {
    /// Discord returned an unsuccessful HTTP response.
    #[error("Discord API returned HTTP {status}: {message}")]
    HttpStatus {
        /// HTTP status returned by Discord.
        status: StatusCode,
        /// Discord's numeric API error code, when present.
        code: Option<i64>,
        /// Human-readable error message or response body.
        message: String,
    },

    /// The supplied bot token could not be represented as an HTTP header.
    #[error("the Discord bot token contains invalid header characters")]
    InvalidToken,

    /// An HTTP transport error occurred.
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// JSON serialization or deserialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// A WebSocket transport error occurred.
    #[error(transparent)]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// A managed asynchronous shard task failed.
    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),

    /// Discord closed the Gateway connection.
    #[error("Discord closed the Gateway connection with {code:?}: {reason}")]
    GatewayClosed {
        /// Discord or WebSocket close code, when one was supplied.
        code: Option<GatewayCloseCode>,
        /// Close reason supplied by the peer.
        reason: String,
    },

    /// A Gateway packet violated the expected protocol sequence.
    #[error("Gateway protocol error: {0}")]
    GatewayProtocol(String),

    /// Gateway transport decompression failed.
    #[error("Gateway compression error: {0}")]
    GatewayCompression(String),

    /// Gateway payload serialization or deserialization failed.
    #[error("Gateway encoding error: {0}")]
    GatewayEncoding(String),

    /// A typed Gateway send event violates a documented payload constraint.
    #[error("invalid Gateway send event: {0}")]
    InvalidGatewaySendEvent(String),

    /// A normal outbound Gateway send would exceed the current connection's rate limit.
    #[error("Gateway outbound rate limit reached; retry after {retry_after:?}")]
    GatewayOutboundRateLimited {
        /// Approximate time until a normal send slot is available.
        retry_after: Duration,
    },

    /// Discord did not acknowledge the previous heartbeat before the next one was due.
    #[error("Gateway heartbeat was not acknowledged")]
    HeartbeatNotAcknowledged,

    /// An outgoing Gateway payload exceeded Discord's size limit.
    #[error("Gateway payload is {actual} bytes; Discord allows at most {limit} bytes")]
    GatewayPayloadTooLarge {
        /// Serialized payload size in bytes.
        actual: usize,
        /// Maximum accepted payload size.
        limit: usize,
    },
}
