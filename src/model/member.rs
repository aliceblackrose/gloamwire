use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{Permissions, RoleId, User};

bitflags! {
    /// Flags attached to a Discord guild member.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GuildMemberFlags: u64 {
        const DID_REJOIN = 1 << 0;
        const COMPLETED_ONBOARDING = 1 << 1;
        const BYPASSES_VERIFICATION = 1 << 2;
        const STARTED_ONBOARDING = 1 << 3;
        const IS_GUEST = 1 << 4;
        const STARTED_HOME_ACTIONS = 1 << 5;
        const COMPLETED_HOME_ACTIONS = 1 << 6;
        const AUTOMOD_QUARANTINED_USERNAME = 1 << 7;
        const DM_SETTINGS_UPSELL_ACKNOWLEDGED = 1 << 9;
        const AUTOMOD_QUARANTINED_GUILD_TAG = 1 << 10;
    }
}

impl Serialize for GuildMemberFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for GuildMemberFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

/// A Discord guild member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuildMember {
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub roles: Vec<RoleId>,
    #[serde(default)]
    pub joined_at: Option<String>,
    #[serde(default)]
    pub premium_since: Option<String>,
    #[serde(default)]
    pub deaf: bool,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub flags: GuildMemberFlags,
    #[serde(default)]
    pub pending: Option<bool>,
    #[serde(default)]
    pub permissions: Option<Permissions>,
    #[serde(default)]
    pub communication_disabled_until: Option<String>,
}
