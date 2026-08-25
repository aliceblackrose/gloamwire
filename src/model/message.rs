use serde::{Deserialize, Serialize};

use super::{ChannelId, GuildId, MessageId, Reaction, User};

/// A Discord message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The message ID.
    pub id: MessageId,
    /// The channel containing the message.
    pub channel_id: ChannelId,
    /// The guild containing the message, when applicable.
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    /// The message author.
    pub author: User,
    /// Textual message content.
    pub content: String,
    /// Reactions currently present on the message.
    #[serde(default)]
    pub reactions: Vec<Reaction>,
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

#[cfg(test)]
mod tests {
    use super::Message;

    #[test]
    fn message_reactions_default_when_omitted() {
        let message: Message = serde_json::from_str(
            r#"{
                "id":"1",
                "channel_id":"2",
                "author":{"id":"3","username":"gloam","discriminator":"0"},
                "content":"hello"
            }"#,
        )
        .expect("message");

        assert!(message.reactions.is_empty());
    }
}
