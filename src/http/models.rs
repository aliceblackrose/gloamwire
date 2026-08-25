use serde::Deserialize;

/// Information returned by Discord's Get Gateway Bot endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GatewayBot {
    /// WebSocket URL to use for Gateway connections.
    pub url: String,
    /// Recommended number of shards.
    pub shards: u32,
    /// Current identify-session limits for the application.
    pub session_start_limit: SessionStartLimit,
}

/// Gateway identify-session rate-limit information.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionStartLimit {
    /// Total identify calls allowed per reset window.
    pub total: u32,
    /// Identify calls remaining in the current window.
    pub remaining: u32,
    /// Milliseconds until the limit resets.
    pub reset_after: u64,
    /// Number of shards that may identify concurrently.
    pub max_concurrency: u32,
}
