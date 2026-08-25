use std::fmt;

use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::{GuildId, RoleId, UserId};

bitflags! {
    /// Discord permission bitset.
    ///
    /// Unknown bits are retained when deserializing so newly introduced Discord
    /// permissions do not require an immediate Gloamwire release.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Permissions: u64 {
        const CREATE_INSTANT_INVITE = 1 << 0;
        const KICK_MEMBERS = 1 << 1;
        const BAN_MEMBERS = 1 << 2;
        const ADMINISTRATOR = 1 << 3;
        const MANAGE_CHANNELS = 1 << 4;
        const MANAGE_GUILD = 1 << 5;
        const ADD_REACTIONS = 1 << 6;
        const VIEW_AUDIT_LOG = 1 << 7;
        const PRIORITY_SPEAKER = 1 << 8;
        const STREAM = 1 << 9;
        const VIEW_CHANNEL = 1 << 10;
        const SEND_MESSAGES = 1 << 11;
        const SEND_TTS_MESSAGES = 1 << 12;
        const MANAGE_MESSAGES = 1 << 13;
        const EMBED_LINKS = 1 << 14;
        const ATTACH_FILES = 1 << 15;
        const READ_MESSAGE_HISTORY = 1 << 16;
        const MENTION_EVERYONE = 1 << 17;
        const USE_EXTERNAL_EMOJIS = 1 << 18;
        const VIEW_GUILD_INSIGHTS = 1 << 19;
        const CONNECT = 1 << 20;
        const SPEAK = 1 << 21;
        const MUTE_MEMBERS = 1 << 22;
        const DEAFEN_MEMBERS = 1 << 23;
        const MOVE_MEMBERS = 1 << 24;
        const USE_VAD = 1 << 25;
        const CHANGE_NICKNAME = 1 << 26;
        const MANAGE_NICKNAMES = 1 << 27;
        const MANAGE_ROLES = 1 << 28;
        const MANAGE_WEBHOOKS = 1 << 29;
        const MANAGE_GUILD_EXPRESSIONS = 1 << 30;
        const USE_APPLICATION_COMMANDS = 1 << 31;
        const REQUEST_TO_SPEAK = 1 << 32;
        const MANAGE_EVENTS = 1 << 33;
        const MANAGE_THREADS = 1 << 34;
        const CREATE_PUBLIC_THREADS = 1 << 35;
        const CREATE_PRIVATE_THREADS = 1 << 36;
        const USE_EXTERNAL_STICKERS = 1 << 37;
        const SEND_MESSAGES_IN_THREADS = 1 << 38;
        const USE_EMBEDDED_ACTIVITIES = 1 << 39;
        const MODERATE_MEMBERS = 1 << 40;
        const VIEW_CREATOR_MONETIZATION_ANALYTICS = 1 << 41;
        const USE_SOUNDBOARD = 1 << 42;
        const CREATE_GUILD_EXPRESSIONS = 1 << 43;
        const CREATE_EVENTS = 1 << 44;
        const USE_EXTERNAL_SOUNDS = 1 << 45;
        const SEND_VOICE_MESSAGES = 1 << 46;
        const SET_VOICE_CHANNEL_STATUS = 1 << 48;
        const SEND_POLLS = 1 << 49;
        const USE_EXTERNAL_APPS = 1 << 50;
        const PIN_MESSAGES = 1 << 51;
        const BYPASS_SLOWMODE = 1 << 52;
    }
}

impl Permissions {
    /// Every bit representable by the current storage type.
    ///
    /// This is intentionally broader than `Permissions::all()` so
    /// Administrator behavior remains forward-compatible with future flags.
    #[must_use]
    pub const fn administrator_permissions() -> Self {
        Self::from_bits_retain(u64::MAX)
    }
}

impl Serialize for Permissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.bits())
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PermissionsVisitor;

        impl de::Visitor<'_> for PermissionsVisitor {
            type Value = Permissions;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a Discord permission integer encoded as a string or u64")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Permissions::from_bits_retain(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse::<u64>()
                    .map(Permissions::from_bits_retain)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_any(PermissionsVisitor)
    }
}

/// Discord permission-overwrite target kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PermissionOverwriteType(pub u8);

impl PermissionOverwriteType {
    pub const ROLE: Self = Self(0);
    pub const MEMBER: Self = Self(1);
}

/// A channel-level Discord permission overwrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOverwrite {
    pub id: super::Snowflake,
    #[serde(rename = "type")]
    pub kind: PermissionOverwriteType,
    pub allow: Permissions,
    pub deny: Permissions,
}

/// Computes guild-level base permissions according to Discord's documented hierarchy.
#[must_use]
pub fn compute_base_permissions(
    guild_id: GuildId,
    owner_id: UserId,
    member_id: UserId,
    everyone_permissions: Permissions,
    role_permissions: impl IntoIterator<Item = Permissions>,
) -> Permissions {
    if member_id == owner_id {
        return Permissions::administrator_permissions();
    }

    let mut permissions = everyone_permissions;
    for role in role_permissions {
        permissions |= role;
    }

    if permissions.contains(Permissions::ADMINISTRATOR) {
        return Permissions::administrator_permissions();
    }

    let _ = guild_id;
    permissions
}

/// Applies channel overwrites according to Discord's overwrite hierarchy.
#[must_use]
pub fn compute_channel_permissions(
    guild_id: GuildId,
    member_id: UserId,
    member_roles: &[RoleId],
    base: Permissions,
    overwrites: &[PermissionOverwrite],
) -> Permissions {
    if base.contains(Permissions::ADMINISTRATOR) {
        return Permissions::administrator_permissions();
    }

    let mut permissions = base;
    let everyone_id = guild_id.snowflake();

    if let Some(overwrite) = overwrites.iter().find(|overwrite| {
        overwrite.kind == PermissionOverwriteType::ROLE && overwrite.id == everyone_id
    }) {
        permissions.remove(overwrite.deny);
        permissions.insert(overwrite.allow);
    }

    let mut role_allow = Permissions::empty();
    let mut role_deny = Permissions::empty();
    for role_id in member_roles {
        if let Some(overwrite) = overwrites.iter().find(|overwrite| {
            overwrite.kind == PermissionOverwriteType::ROLE && overwrite.id == role_id.snowflake()
        }) {
            role_allow |= overwrite.allow;
            role_deny |= overwrite.deny;
        }
    }
    permissions.remove(role_deny);
    permissions.insert(role_allow);

    if let Some(overwrite) = overwrites.iter().find(|overwrite| {
        overwrite.kind == PermissionOverwriteType::MEMBER && overwrite.id == member_id.snowflake()
    }) {
        permissions.remove(overwrite.deny);
        permissions.insert(overwrite.allow);
    }

    permissions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_serialize_as_decimal_string() {
        let permissions = Permissions::SEND_MESSAGES | Permissions::ADD_REACTIONS;
        assert_eq!(
            serde_json::to_string(&permissions).expect("serialize"),
            "\"2112\""
        );
    }

    #[test]
    fn administrator_bypasses_overwrites() {
        let result = compute_channel_permissions(
            GuildId::new(1),
            UserId::new(2),
            &[],
            Permissions::ADMINISTRATOR,
            &[PermissionOverwrite {
                id: GuildId::new(1).snowflake(),
                kind: PermissionOverwriteType::ROLE,
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
            }],
        );
        assert!(result.contains(Permissions::VIEW_CHANNEL));
    }
}
