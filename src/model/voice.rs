use serde::{Deserialize, Serialize};

use super::{ChannelId, GuildId, GuildMember, UserId};

/// A user's voice connection state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceState {
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub channel_id: Option<ChannelId>,
    pub user_id: UserId,
    #[serde(default)]
    pub member: Option<GuildMember>,
    pub session_id: String,
    pub deaf: bool,
    pub mute: bool,
    pub self_deaf: bool,
    pub self_mute: bool,
    #[serde(default)]
    pub self_stream: Option<bool>,
    #[serde(default)]
    pub self_video: bool,
    pub suppress: bool,
    #[serde(default)]
    pub request_to_speak_timestamp: Option<String>,
}
