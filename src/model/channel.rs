use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    ApplicationId, ChannelId, EmojiId, GuildId, MessageId, PermissionOverwrite, Permissions,
    Snowflake, User, UserId,
};

/// Discord channel type.
///
/// This remains a numeric newtype so unknown future channel types deserialize
/// without breaking applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelType(pub u8);

impl ChannelType {
    pub const GUILD_TEXT: Self = Self(0);
    pub const DM: Self = Self(1);
    pub const GUILD_VOICE: Self = Self(2);
    pub const GROUP_DM: Self = Self(3);
    pub const GUILD_CATEGORY: Self = Self(4);
    pub const GUILD_ANNOUNCEMENT: Self = Self(5);
    pub const ANNOUNCEMENT_THREAD: Self = Self(10);
    pub const PUBLIC_THREAD: Self = Self(11);
    pub const PRIVATE_THREAD: Self = Self(12);
    pub const GUILD_STAGE_VOICE: Self = Self(13);
    pub const GUILD_DIRECTORY: Self = Self(14);
    pub const GUILD_FORUM: Self = Self(15);
    pub const GUILD_MEDIA: Self = Self(16);
}

bitflags! {
    /// Discord channel flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ChannelFlags: u64 {
        const PINNED = 1 << 1;
        const REQUIRE_TAG = 1 << 4;
        const HIDE_MEDIA_DOWNLOAD_OPTIONS = 1 << 15;
        const CHANNEL_OBFUSCATED = 1 << 17;
        const IS_SPOILER_CHANNEL = 1 << 21;
    }
}

impl Serialize for ChannelFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for ChannelFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

/// Thread-specific channel metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMetadata {
    pub archived: bool,
    pub auto_archive_duration: u32,
    pub archive_timestamp: String,
    pub locked: bool,
    #[serde(default)]
    pub invitable: Option<bool>,
    #[serde(default)]
    pub create_timestamp: Option<String>,
}

/// The default reaction configured for a forum or media channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultReaction {
    pub emoji_id: Option<EmojiId>,
    pub emoji_name: Option<String>,
}

/// A forum/media tag available on a channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForumTag {
    pub id: Snowflake,
    pub name: String,
    pub moderated: bool,
    pub emoji_id: Option<EmojiId>,
    pub emoji_name: Option<String>,
}

/// A Discord channel.
///
/// Most metadata fields are optional because Discord can send obfuscated
/// Gateway channels when the bot cannot view them. `id`, `type`, `position`,
/// and `parent_id` are the only fields Discord currently guarantees will not be
/// obfuscated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    #[serde(rename = "type")]
    pub kind: ChannelType,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub permission_overwrites: Vec<PermissionOverwrite>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub nsfw: Option<bool>,
    #[serde(default)]
    pub last_message_id: Option<MessageId>,
    #[serde(default)]
    pub bitrate: Option<u32>,
    #[serde(default)]
    pub user_limit: Option<u32>,
    #[serde(default)]
    pub rate_limit_per_user: Option<u32>,
    #[serde(default)]
    pub recipients: Vec<User>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub owner_id: Option<UserId>,
    #[serde(default)]
    pub application_id: Option<ApplicationId>,
    #[serde(default)]
    pub managed: Option<bool>,
    #[serde(default)]
    pub parent_id: Option<ChannelId>,
    #[serde(default)]
    pub last_pin_timestamp: Option<String>,
    #[serde(default)]
    pub rtc_region: Option<String>,
    #[serde(default)]
    pub video_quality_mode: Option<u8>,
    #[serde(default)]
    pub message_count: Option<u32>,
    #[serde(default)]
    pub member_count: Option<u32>,
    #[serde(default)]
    pub thread_metadata: Option<ThreadMetadata>,
    #[serde(default)]
    pub default_auto_archive_duration: Option<u32>,
    #[serde(default)]
    pub permissions: Option<Permissions>,
    #[serde(default)]
    pub app_permissions: Option<Permissions>,
    #[serde(default)]
    pub flags: ChannelFlags,
    #[serde(default)]
    pub total_message_sent: Option<u32>,
    #[serde(default)]
    pub available_tags: Vec<ForumTag>,
    #[serde(default)]
    pub applied_tags: Vec<Snowflake>,
    #[serde(default)]
    pub default_reaction_emoji: Option<DefaultReaction>,
    #[serde(default)]
    pub default_thread_rate_limit_per_user: Option<u32>,
    #[serde(default)]
    pub default_sort_order: Option<u8>,
    #[serde(default)]
    pub default_forum_layout: Option<u8>,
}

impl Channel {
    /// Returns whether Discord marked this Gateway channel as obfuscated.
    #[must_use]
    pub const fn is_obfuscated(&self) -> bool {
        self.flags.contains(ChannelFlags::CHANNEL_OBFUSCATED)
    }
}

#[cfg(test)]
mod tests {
    use super::{Channel, ChannelFlags};

    #[test]
    fn parses_obfuscated_channel_without_sensitive_metadata() {
        let json = r#"{
            "id":"123",
            "type":0,
            "position":1,
            "parent_id":null,
            "flags":131072,
            "permission_overwrites":[]
        }"#;

        let channel: Channel = serde_json::from_str(json).expect("obfuscated channel");
        assert!(channel.is_obfuscated());
        assert!(channel.name.is_none());
        assert!(channel.flags.contains(ChannelFlags::CHANNEL_OBFUSCATED));
    }
}
