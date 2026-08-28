use std::time::Duration;

use thiserror::Error;

use super::VoiceCloseCode;

/// Result type returned by Discord voice operations.
pub type VoiceResult<T> = std::result::Result<T, VoiceError>;

/// Errors produced by the Discord voice subsystem.
#[derive(Debug, Error)]
pub enum VoiceError {
    /// Opening, reading, or writing the UDP socket failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A Voice Gateway JSON payload could not be encoded or decoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// The Voice Gateway WebSocket transport failed.
    #[error(transparent)]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// Discord closed the Voice Gateway connection.
    #[error("Discord closed the Voice Gateway connection with {code:?}: {reason}")]
    Closed {
        /// Discord or WebSocket close code, when supplied.
        code: Option<VoiceCloseCode>,
        /// Close reason supplied by the peer.
        reason: String,
    },

    /// A Voice Gateway packet violated the expected protocol sequence.
    #[error("Voice Gateway protocol error: {0}")]
    Protocol(String),

    /// The previous Voice Gateway heartbeat was not acknowledged in time.
    #[error("Voice Gateway heartbeat was not acknowledged")]
    HeartbeatNotAcknowledged,

    /// Discord did not advertise a transport-encryption mode Gloamwire can use.
    #[error("Voice Gateway did not advertise a supported transport-encryption mode")]
    UnsupportedEncryptionMode,

    /// Discord's UDP IP-discovery response was malformed.
    #[error("invalid Discord voice UDP discovery response: {0}")]
    InvalidDiscoveryResponse(String),

    /// A Voice Gateway operation timed out.
    #[error("Voice Gateway operation timed out after {0:?}")]
    Timeout(Duration),
}
