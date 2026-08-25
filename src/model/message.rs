use serde::{Deserialize, Serialize};

use super::{Snowflake, User};

/// A Discord message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The message ID.
    pub id: Snowflake,
    /// The channel containing the message.
    pub channel_id: Snowflake,
    /// The guild containing the message, when applicable.
    #[serde(default)]
    pub guild_id: Option<Snowflake>,
    /// The message author.
    pub author: User,
    /// Textual message content.
    pub content: String,
}

/// Parameters accepted by Discord's Create Message endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CreateMessage {
    /// Textual message content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Whether Discord should synthesize a text-to-speech message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
}

impl CreateMessage {
    /// Creates message parameters containing plain text content.
    #[must_use]
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tts: None,
        }
    }
}
