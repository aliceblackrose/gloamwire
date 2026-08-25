use serde::{Deserialize, Serialize};

use super::{
    ApplicationId, Channel, ChannelId, GuildId, GuildMember, Permissions, Role, UserId, VoiceState,
};

/// A Discord guild object with the core fields needed by REST and Gateway state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guild {
    pub id: GuildId,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub splash: Option<String>,
    #[serde(default)]
    pub discovery_splash: Option<String>,
    pub owner_id: UserId,
    #[serde(default)]
    pub permissions: Option<Permissions>,
    #[serde(default)]
    pub afk_channel_id: Option<ChannelId>,
    #[serde(default)]
    pub afk_timeout: u32,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub application_id: Option<ApplicationId>,
    #[serde(default)]
    pub system_channel_id: Option<ChannelId>,
    #[serde(default)]
    pub rules_channel_id: Option<ChannelId>,
    #[serde(default)]
    pub vanity_url_code: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub preferred_locale: Option<String>,
    #[serde(default)]
    pub public_updates_channel_id: Option<ChannelId>,
    #[serde(default)]
    pub safety_alerts_channel_id: Option<ChannelId>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub threads: Vec<Channel>,
    #[serde(default)]
    pub members: Vec<GuildMember>,
    #[serde(default)]
    pub voice_states: Vec<VoiceState>,
    #[serde(default)]
    pub member_count: Option<u64>,
    #[serde(default)]
    pub unavailable: Option<bool>,
}

/// A guild that Discord has not made available on the current Gateway session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnavailableGuild {
    pub id: GuildId,
    #[serde(default)]
    pub unavailable: bool,
}
