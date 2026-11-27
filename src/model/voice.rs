use serde::{Deserialize, Serialize};

use super::{ChannelId, GuildId, GuildMember, UserId};

/// A user's voice connection state.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

impl std::fmt::Debug for VoiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceState")
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .field("user_id", &self.user_id)
            .field("session_id", &"[REDACTED]")
            .field("deaf", &self.deaf)
            .field("mute", &self.mute)
            .field("self_deaf", &self.self_deaf)
            .field("self_mute", &self.self_mute)
            .field("suppress", &self.suppress)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn voice_state_debug_does_not_disclose_session_identifier() {
        let state: super::VoiceState = serde_json::from_value(serde_json::json!({
            "guild_id":"1","channel_id":"2","user_id":"3","session_id":"secret-session",
            "deaf":false,"mute":false,"self_deaf":false,"self_mute":false,"suppress":false
        }))
        .unwrap();
        assert!(!format!("{state:?}").contains("secret-session"));
        assert!(format!("{state:?}").contains("[REDACTED]"));
    }
}
