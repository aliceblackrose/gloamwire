use serde_json::Value;

/// A dispatch event delivered by Discord.
#[derive(Clone, PartialEq)]
pub struct DispatchEvent {
    /// Dispatch event name, such as `READY` or `MESSAGE_CREATE`.
    pub name: String,
    /// Gateway sequence number associated with the dispatch.
    pub sequence: u64,
    /// Event-specific JSON payload.
    pub data: Value,
}

impl std::fmt::Debug for DispatchEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchEvent")
            .field("name", &self.name)
            .field("sequence", &self.sequence)
            .field("data", &"[OMITTED]")
            .finish_non_exhaustive()
    }
}

/// A meaningful event received from the Gateway connection.
#[derive(Clone, PartialEq)]
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

impl std::fmt::Debug for GatewayEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dispatch(event) => f.debug_tuple("Dispatch").field(event).finish(),
            Self::HeartbeatAck => f.write_str("HeartbeatAck"),
            Self::Reconnect => f.write_str("Reconnect"),
            Self::InvalidSession { resumable } => f
                .debug_struct("InvalidSession")
                .field("resumable", resumable)
                .finish(),
            Self::Unknown { opcode, .. } => f
                .debug_struct("Unknown")
                .field("opcode", opcode)
                .finish_non_exhaustive(),
        }
    }
}

impl GatewayEvent {
    pub(crate) fn estimated_bytes(&self) -> usize {
        match self {
            Self::Dispatch(event) => {
                crate::queue_budget::json_bytes(&event.data).saturating_add(event.name.capacity())
            }
            Self::Unknown { data, .. } => crate::queue_budget::json_bytes(data),
            _ => 128,
        }
    }
}
