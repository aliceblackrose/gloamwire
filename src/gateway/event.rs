use serde_json::Value;

/// A dispatch event delivered by Discord.
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchEvent {
    /// Dispatch event name, such as `READY` or `MESSAGE_CREATE`.
    pub name: String,
    /// Gateway sequence number associated with the dispatch.
    pub sequence: u64,
    /// Event-specific JSON payload.
    pub data: Value,
}

/// A meaningful event received from the Gateway connection.
#[derive(Debug, Clone, PartialEq)]
pub enum GatewayEvent {
    /// A normal Discord dispatch event.
    Dispatch(DispatchEvent),
    /// Discord acknowledged the most recent heartbeat.
    HeartbeatAck,
    /// Discord requested that the client reconnect and attempt to resume.
    Reconnect,
    /// Discord declared the current session invalid.
    InvalidSession {
        /// Whether the session may be resumable.
        resumable: bool,
    },
    /// An opcode not explicitly modeled by this version of Gloamwire.
    Unknown {
        /// Gateway opcode.
        opcode: u8,
        /// Raw data payload.
        data: Value,
    },
}
