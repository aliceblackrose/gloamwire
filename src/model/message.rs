use serde::{Deserialize, Serialize};

use super::{ChannelId, Component, GuildId, MessageId, Poll, PollCreateRequest, Reaction, User};

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
    /// Message components, including Components V2 layouts and interactive controls.
    #[serde(default)]
    pub components: Vec<Component>,
    /// Poll attached to the message, when present.
    #[serde(default)]
    pub poll: Option<Poll>,
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
    /// Poll to create with the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<PollCreateRequest>,
}

impl CreateMessage {
    /// Creates message parameters containing plain text content.
    #[must_use]
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tts: None,
            poll: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Message;
    use crate::model::{ComponentType, PollLayoutType};

    #[test]
    fn message_collections_default_when_omitted() {
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
        assert!(message.components.is_empty());
        assert!(message.poll.is_none());
    }

    #[test]
    fn parses_components_v2_on_messages() {
        let message: Message = serde_json::from_str(
            r##"{
                "id":"1",
                "channel_id":"2",
                "author":{"id":"3","username":"gloam","discriminator":"0"},
                "content":"",
                "components":[{"type":10,"content":"# Hello"}]
            }"##,
        )
        .expect("message");

        assert_eq!(message.components[0].kind, ComponentType::TEXT_DISPLAY);
    }

    #[test]
    fn parses_poll_on_message() {
        let message: Message = serde_json::from_str(
            r#"{
                "id":"1",
                "channel_id":"2",
                "author":{"id":"3","username":"gloam","discriminator":"0"},
                "content":"",
                "poll":{
                    "question":{"text":"Ready?"},
                    "answers":[{"answer_id":1,"poll_media":{"text":"Yes"}}],
                    "expiry":null,
                    "allow_multiselect":false,
                    "layout_type":1
                }
            }"#,
        )
        .expect("poll message");

        assert_eq!(
            message.poll.expect("poll").layout_type,
            PollLayoutType::DEFAULT
        );
    }
}
