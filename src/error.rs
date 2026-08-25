use std::{fmt, time::Duration};

use reqwest::StatusCode;
use serde_json::Value;
use thiserror::Error;

use crate::gateway::GatewayCloseCode;

/// A result returned by Gloamwire operations.
pub type Result<T> = std::result::Result<T, Error>;

/// One field-level validation failure returned by Discord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordValidationError {
    /// Object keys and array indexes locating the invalid field.
    pub path: Vec<String>,
    /// Discord's machine-readable validation error code.
    pub code: String,
    /// Human-readable validation error message.
    pub message: String,
}

impl DiscordValidationError {
    /// Returns the validation path in dotted form, such as `activities.0.platform`.
    #[must_use]
    pub fn dotted_path(&self) -> String {
        self.path.join(".")
    }
}

/// Structured form-validation details returned by Discord's HTTP API.
///
/// Discord may add new nested error shapes over time. `raw_errors` therefore
/// retains the complete error tree while `validation_errors` provides a
/// convenient flattened view of every `_errors` entry found within it.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscordApiError {
    /// Discord's numeric API error code.
    pub code: i64,
    /// Top-level human-readable error message.
    pub message: String,
    /// Complete nested value from Discord's `errors` response field.
    pub raw_errors: Value,
    /// Flattened field-level validation errors.
    pub validation_errors: Vec<DiscordValidationError>,
}

impl DiscordApiError {
    pub(crate) fn new(code: i64, message: String, raw_errors: Value) -> Self {
        let mut validation_errors = Vec::new();
        let mut path = Vec::new();
        collect_validation_errors(&raw_errors, &mut path, &mut validation_errors);

        Self {
            code,
            message,
            raw_errors,
            validation_errors,
        }
    }
}

impl fmt::Display for DiscordApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} (Discord code {})", self.message, self.code)
    }
}

fn collect_validation_errors(
    value: &Value,
    path: &mut Vec<String>,
    validation_errors: &mut Vec<DiscordValidationError>,
) {
    match value {
        Value::Object(object) => {
            if let Some(Value::Array(errors)) = object.get("_errors") {
                for error in errors {
                    let Some(code) = error.get("code").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(message) = error.get("message").and_then(Value::as_str) else {
                        continue;
                    };

                    validation_errors.push(DiscordValidationError {
                        path: path.clone(),
                        code: code.to_owned(),
                        message: message.to_owned(),
                    });
                }
            }

            for (key, child) in object {
                if key == "_errors" {
                    continue;
                }

                path.push(key.clone());
                collect_validation_errors(child, path, validation_errors);
                path.pop();
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                path.push(index.to_string());
                collect_validation_errors(child, path, validation_errors);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Errors produced by the REST and Gateway clients.
#[derive(Debug, Error)]
pub enum Error {
    /// Discord returned structured form-validation errors.
    #[error("Discord API returned HTTP {status}: {error}")]
    DiscordApi {
        /// HTTP status returned by Discord.
        status: StatusCode,
        /// Structured Discord API validation error.
        error: DiscordApiError,
    },

    /// Discord returned an unsuccessful HTTP response without structured validation details.
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

    /// Reading a file-backed upload failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

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
        /// Serialized payload size.
        actual: usize,
        /// Maximum accepted payload size.
        limit: usize,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::DiscordApiError;

    #[test]
    fn flattens_nested_object_and_array_index_paths() {
        let error = DiscordApiError::new(
            50035,
            "Invalid Form Body".to_owned(),
            json!({
                "activities": {
                    "0": {
                        "platform": {
                            "_errors": [{
                                "code": "BASE_TYPE_CHOICES",
                                "message": "Value must be one of the allowed platforms."
                            }]
                        },
                        "type": {
                            "_errors": [{
                                "code": "BASE_TYPE_CHOICES",
                                "message": "Value must be one of the allowed types."
                            }]
                        }
                    }
                }
            }),
        );

        assert_eq!(error.validation_errors.len(), 2);
        assert_eq!(
            error.validation_errors[0].dotted_path(),
            "activities.0.platform"
        );
        assert_eq!(error.validation_errors[0].code, "BASE_TYPE_CHOICES");
        assert_eq!(
            error.validation_errors[1].dotted_path(),
            "activities.0.type"
        );
    }

    #[test]
    fn flattens_root_request_errors() {
        let raw = json!({
            "_errors": [{
                "code": "APPLICATION_COMMAND_TOO_LARGE",
                "message": "Command exceeds maximum size (8000)"
            }]
        });
        let error = DiscordApiError::new(50035, "Invalid Form Body".to_owned(), raw.clone());

        assert_eq!(error.raw_errors, raw);
        assert!(error.validation_errors[0].path.is_empty());
        assert_eq!(
            error.validation_errors[0].code,
            "APPLICATION_COMMAND_TOO_LARGE"
        );
    }

    #[test]
    fn ignores_unknown_error_nodes_without_losing_raw_tree() {
        let raw = json!({
            "future": {
                "unexpected": true,
                "_errors": [{"code": 123, "message": {"nested": true}}]
            }
        });
        let error = DiscordApiError::new(50035, "Invalid Form Body".to_owned(), raw.clone());

        assert_eq!(error.raw_errors, raw);
        assert!(error.validation_errors.is_empty());
    }
}
