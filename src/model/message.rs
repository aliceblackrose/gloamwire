use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{
    ApplicationId, Attachment, AttachmentId, Channel, ChannelId, ChannelType, Component, Embed,
    GuildId, MessageId, Poll, PollCreateRequest, Reaction, RoleId, StickerId, User, UserId,
    WebhookId,
};

/// Discord message type.
///
/// The numeric representation is retained so future message types remain
/// deserializable before Gloamwire adds dedicated behavior for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageType(pub u8);

impl MessageType {
    pub const DEFAULT: Self = Self(0);
    pub const REPLY: Self = Self(19);
    pub const CHAT_INPUT_COMMAND: Self = Self(20);
    pub const THREAD_STARTER_MESSAGE: Self = Self(21);
    pub const CONTEXT_MENU_COMMAND: Self = Self(23);
    pub const PURCHASE_NOTIFICATION: Self = Self(44);
    pub const POLL_RESULT: Self = Self(46);
}

bitflags! {
    /// Flags attached to a Discord message.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct MessageFlags: u64 {
        const CROSSPOSTED = 1 << 0;
        const IS_CROSSPOST = 1 << 1;
        const SUPPRESS_EMBEDS = 1 << 2;
        const SOURCE_MESSAGE_DELETED = 1 << 3;
        const URGENT = 1 << 4;
        const HAS_THREAD = 1 << 5;
        const EPHEMERAL = 1 << 6;
        const LOADING = 1 << 7;
        const FAILED_TO_MENTION_SOME_ROLES_IN_THREAD = 1 << 8;
        const SUPPRESS_NOTIFICATIONS = 1 << 12;
        const IS_VOICE_MESSAGE = 1 << 13;
        const HAS_SNAPSHOT = 1 << 14;
        const IS_COMPONENTS_V2 = 1 << 15;
    }
}

impl Serialize for MessageFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for MessageFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

/// Integer-or-string nonce accepted by Discord message endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageNonce {
    Integer(i64),
    String(String),
}

/// Message reference type used for replies and forwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageReferenceType(pub u8);

impl MessageReferenceType {
    pub const DEFAULT: Self = Self(0);
    pub const FORWARD: Self = Self(1);
}

/// A reference to another Discord message.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReference {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MessageReferenceType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_if_not_exists: Option<bool>,
}

/// A channel mention embedded in a crossposted message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMention {
    pub id: ChannelId,
    pub guild_id: GuildId,
    #[serde(rename = "type")]
    pub kind: ChannelType,
    pub name: String,
}

/// Controls which mentions Discord parses from outgoing message content.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedMentions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parse: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<UserId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replied_user: Option<bool>,
}

/// Attachment identifier used in create/edit requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttachmentRequestId {
    Existing(AttachmentId),
    Upload(u32),
}

/// Attachment metadata included in a message create/edit request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachmentRequest {
    pub id: AttachmentRequestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_spoiler: Option<bool>,
}

impl From<&Attachment> for AttachmentRequest {
    fn from(attachment: &Attachment) -> Self {
        Self {
            id: AttachmentRequestId::Existing(attachment.id),
            filename: Some(attachment.filename.clone()),
            title: attachment.title.clone(),
            description: attachment.description.clone(),
            duration_secs: attachment.duration_secs,
            waveform: attachment.waveform.clone(),
            is_spoiler: None,
        }
    }
}

/// Custom client-side theme shared through a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedClientTheme {
    #[serde(default)]
    pub colors: Vec<String>,
    pub gradient_angle: u16,
    pub base_mix: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_theme: Option<BaseTheme>,
}

/// Base mode used by a shared client theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BaseTheme(pub u8);

impl BaseTheme {
    pub const UNSET: Self = Self(0);
    pub const DARK: Self = Self(1);
    pub const LIGHT: Self = Self(2);
    pub const DARKER: Self = Self(3);
    pub const MIDNIGHT: Self = Self(4);
}

/// Call metadata attached to a private-channel message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCall {
    #[serde(default)]
    pub participants: Vec<UserId>,
    #[serde(default)]
    pub ended_timestamp: Option<String>,
}

/// A Discord message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub channel_id: ChannelId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub author: User,
    pub content: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub tts: bool,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub mention_roles: Vec<RoleId>,
    #[serde(default)]
    pub mention_channels: Vec<ChannelMention>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    #[serde(default)]
    pub nonce: Option<MessageNonce>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub webhook_id: Option<WebhookId>,
    #[serde(rename = "type", default)]
    pub kind: Option<MessageType>,
    #[serde(default)]
    pub application: Option<Value>,
    #[serde(default)]
    pub application_id: Option<ApplicationId>,
    #[serde(default)]
    pub flags: Option<MessageFlags>,
    #[serde(default)]
    pub message_reference: Option<MessageReference>,
    #[serde(default)]
    pub referenced_message: Option<Box<Message>>,
    #[serde(default)]
    pub interaction_metadata: Option<Value>,
    #[serde(default)]
    pub thread: Option<Channel>,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub sticker_items: Vec<Value>,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub role_subscription_data: Option<Value>,
    #[serde(default)]
    pub resolved: Option<Value>,
    #[serde(default)]
    pub poll: Option<Poll>,
    #[serde(default)]
    pub call: Option<MessageCall>,
    #[serde(default)]
    pub shared_client_theme: Option<SharedClientTheme>,
}

/// Parameters accepted by Discord's Create Message endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CreateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<MessageNonce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sticker_ids: Vec<StickerId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_nonce: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<PollCreateRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_client_theme: Option<SharedClientTheme>,
}

impl CreateMessage {
    /// Creates message parameters containing plain text content.
    #[must_use]
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

/// Parameters accepted by Discord's Edit Message endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct EditMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<AttachmentRequest>>,
}

/// One message currently pinned in a channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessagePin {
    pub pinned_at: String,
    pub message: Message,
}

/// Response from Discord's current Get Channel Pins endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelPins {
    #[serde(default)]
    pub items: Vec<MessagePin>,
    pub has_more: bool,
}

/// JSON body for Discord's Bulk Delete Messages endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BulkDeleteMessages {
    pub messages: Vec<MessageId>,
}

#[cfg(test)]
mod tests {
    use super::{
        AttachmentRequest, AttachmentRequestId, BaseTheme, CreateMessage, Message, MessageFlags,
        MessageReferenceType, MessageType, SharedClientTheme,
    };
    use crate::model::{AttachmentId, ComponentType, PollLayoutType};

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
        assert!(message.attachments.is_empty());
        assert!(message.embeds.is_empty());
        assert!(message.components.is_empty());
        assert!(message.poll.is_none());
    }

    #[test]
    fn parses_current_message_metadata() {
        let message: Message = serde_json::from_str(
            r#"{
                "id":"1",
                "channel_id":"2",
                "author":{"id":"3","username":"gloam","discriminator":"0"},
                "content":"hello",
                "timestamp":"2026-08-25T20:00:00+00:00",
                "tts":false,
                "mention_everyone":false,
                "mentions":[],
                "mention_roles":[],
                "attachments":[],
                "embeds":[],
                "pinned":false,
                "type":46,
                "flags":4,
                "message_reference":{"type":1,"message_id":"4","channel_id":"2"}
            }"#,
        )
        .expect("message");

        assert_eq!(message.kind, Some(MessageType::POLL_RESULT));
        assert!(
            message
                .flags
                .expect("flags")
                .contains(MessageFlags::SUPPRESS_EMBEDS)
        );
        assert_eq!(
            message.message_reference.expect("reference").kind,
            Some(MessageReferenceType::FORWARD)
        );
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

    #[test]
    fn attachment_request_ids_preserve_existing_and_upload_forms() {
        let existing = AttachmentRequest {
            id: AttachmentRequestId::Existing(AttachmentId::new(10)),
            filename: None,
            title: None,
            description: None,
            duration_secs: None,
            waveform: None,
            is_spoiler: None,
        };
        let upload = AttachmentRequest {
            id: AttachmentRequestId::Upload(0),
            ..existing.clone()
        };

        assert_eq!(serde_json::to_value(existing).expect("existing")["id"], "10");
        assert_eq!(serde_json::to_value(upload).expect("upload")["id"], 0);
    }

    #[test]
    fn serializes_extended_create_message() {
        let mut message = CreateMessage::content("hello");
        message.flags = Some(MessageFlags::SUPPRESS_NOTIFICATIONS);
        message.shared_client_theme = Some(SharedClientTheme {
            colors: vec!["5865F2".to_owned()],
            gradient_angle: 0,
            base_mix: 58,
            base_theme: Some(BaseTheme::DARK),
        });

        let value = serde_json::to_value(message).expect("create message");
        assert_eq!(value["flags"], 4096);
        assert_eq!(value["shared_client_theme"]["base_theme"], 1);
    }
}
