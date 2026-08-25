use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{ChannelId, ChannelType, GuildId, RoleColors, RoleId, User};

/// Discord invite type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InviteType(pub u8);

impl InviteType {
    pub const GUILD: Self = Self(0);
    pub const GROUP_DM: Self = Self(1);
    pub const FRIEND: Self = Self(2);
}

/// Target type for voice-channel invites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InviteTargetType(pub u8);

impl InviteTargetType {
    pub const STREAM: Self = Self(1);
    pub const EMBEDDED_APPLICATION: Self = Self(2);
}

bitflags! {
    /// Flags attached to a guild invite.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct InviteFlags: u64 {
        const IS_GUEST_INVITE = 1 << 0;
    }
}

impl Serialize for InviteFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for InviteFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

/// Partial guild object embedded in an invite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InviteGuild {
    pub id: GuildId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub splash: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub verification_level: Option<u8>,
    #[serde(default)]
    pub vanity_url_code: Option<String>,
    #[serde(default)]
    pub nsfw_level: Option<u8>,
    #[serde(default)]
    pub premium_subscription_count: Option<u32>,
}

/// Partial channel object embedded in an invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteChannel {
    pub id: ChannelId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<ChannelType>,
}

/// Partial role assigned when a user accepts an invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteRole {
    pub id: RoleId,
    pub name: String,
    pub position: i32,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub colors: Option<RoleColors>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub unicode_emoji: Option<String>,
}

/// A Discord invite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invite {
    #[serde(rename = "type")]
    pub kind: InviteType,
    pub code: String,
    #[serde(default)]
    pub guild: Option<InviteGuild>,
    #[serde(default)]
    pub channel: Option<InviteChannel>,
    #[serde(default)]
    pub inviter: Option<User>,
    #[serde(default)]
    pub target_type: Option<InviteTargetType>,
    #[serde(default)]
    pub target_user: Option<User>,
    /// Partial application payload for embedded-application invites.
    #[serde(default)]
    pub target_application: Option<Value>,
    #[serde(default)]
    pub approximate_presence_count: Option<u32>,
    #[serde(default)]
    pub approximate_member_count: Option<u32>,
    pub expires_at: Option<String>,
    /// Scheduled-event data remains lossless until the scheduled-event model slice lands.
    #[serde(default)]
    pub guild_scheduled_event: Option<Value>,
    #[serde(default)]
    pub flags: Option<InviteFlags>,
    #[serde(default)]
    pub roles: Vec<InviteRole>,
    #[serde(default)]
    pub uses: Option<u32>,
    #[serde(default)]
    pub max_uses: Option<u32>,
    #[serde(default)]
    pub max_age: Option<u32>,
    #[serde(default)]
    pub temporary: Option<bool>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Status of Discord's asynchronous invite target-user CSV processing job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InviteTargetUsersJobStatusType(pub u8);

impl InviteTargetUsersJobStatusType {
    pub const UNSPECIFIED: Self = Self(0);
    pub const PROCESSING: Self = Self(1);
    pub const COMPLETED: Self = Self(2);
    pub const FAILED: Self = Self(3);
}

/// Processing status for invite target users uploaded as CSV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteTargetUsersJobStatus {
    pub status: InviteTargetUsersJobStatusType,
    pub total_users: u64,
    pub processed_users: u64,
    pub created_at: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        Invite, InviteFlags, InviteTargetType, InviteTargetUsersJobStatus,
        InviteTargetUsersJobStatusType, InviteType,
    };

    #[test]
    fn parses_current_guild_invite() {
        let invite: Invite = serde_json::from_str(
            r#"{
                "type":0,
                "code":"0vCdhLbwjZZTWZLD",
                "guild":{
                    "id":"165176875973476352",
                    "name":"CS:GO Fraggers Only",
                    "features":["NEWS","DISCOVERABLE"],
                    "verification_level":2,
                    "nsfw_level":0,
                    "premium_subscription_count":5
                },
                "channel":{"id":"165176875973476352","name":"illuminati","type":0},
                "inviter":{"id":"115590097100865541","username":"speed","discriminator":"0"},
                "target_type":1,
                "target_user":{"id":"165176875973476353","username":"bob","discriminator":"0"},
                "expires_at":null,
                "flags":1,
                "roles":[{
                    "id":"42",
                    "name":"Guest",
                    "position":1,
                    "color":0,
                    "colors":{"primary_color":0,"secondary_color":null,"tertiary_color":null},
                    "icon":null,
                    "unicode_emoji":"👋"
                }]
            }"#,
        )
        .expect("invite");

        assert_eq!(invite.kind, InviteType::GUILD);
        assert_eq!(invite.target_type, Some(InviteTargetType::STREAM));
        assert!(
            invite
                .flags
                .expect("flags")
                .contains(InviteFlags::IS_GUEST_INVITE)
        );
        assert_eq!(invite.roles[0].id.get(), 42);
    }

    #[test]
    fn parses_friend_invite_without_guild_or_channel() {
        let invite: Invite = serde_json::from_str(
            r#"{
                "type":2,
                "code":"friend-code",
                "channel":null,
                "expires_at":null
            }"#,
        )
        .expect("friend invite");

        assert_eq!(invite.kind, InviteType::FRIEND);
        assert!(invite.guild.is_none());
        assert!(invite.channel.is_none());
    }

    #[test]
    fn parses_target_users_job_status() {
        let status: InviteTargetUsersJobStatus = serde_json::from_str(
            r#"{
                "status":3,
                "total_users":100,
                "processed_users":41,
                "created_at":"2025-01-08T12:00:00.000000+00:00",
                "completed_at":null,
                "error_message":"Failed to parse CSV file"
            }"#,
        )
        .expect("target users job status");

        assert_eq!(status.status, InviteTargetUsersJobStatusType::FAILED);
        assert_eq!(status.processed_users, 41);
    }

    #[test]
    fn invite_types_preserve_unknown_values() {
        let kind: InviteType = serde_json::from_str("9").expect("invite type");
        assert_eq!(kind, InviteType(9));
    }
}
