use serde::Deserialize;

use crate::{
    gateway::DispatchEvent,
    model::{ChannelId, GuildId, UserId, VoiceState},
};

/// Main-Gateway `VOICE_SERVER_UPDATE` data required to establish voice.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VoiceServerUpdate {
    pub token: String,
    pub guild_id: GuildId,
    /// Voice server host, or `None` while Discord is reallocating the server.
    pub endpoint: Option<String>,
}

/// Complete set of main-Gateway values needed to identify to the Voice Gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceConnectionInfo {
    pub guild_id: GuildId,
    /// Voice channel ID. DAVE uses this snowflake as its MLS group ID.
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub session_id: String,
    pub token: String,
    pub endpoint: String,
}

/// Current result of feeding a main-Gateway dispatch into a voice rendezvous.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceRendezvousStatus {
    /// The dispatch was unrelated or one half of the rendezvous is still missing.
    Pending,
    /// Discord temporarily has no voice server allocated for this guild.
    ServerUnavailable,
    /// Both the bot's Voice State Update and Voice Server Update are available.
    Ready(VoiceConnectionInfo),
}

/// Collects the two main-Gateway dispatches Discord sends when joining voice.
///
/// Discord does not guarantee whether the bot's `VOICE_STATE_UPDATE` or the
/// guild's `VOICE_SERVER_UPDATE` arrives first. Feed every main-Gateway
/// [`DispatchEvent`] through this value until it returns
/// [`VoiceRendezvousStatus::Ready`].
#[derive(Debug, Clone)]
pub struct VoiceRendezvous {
    guild_id: GuildId,
    user_id: UserId,
    voice_state: Option<VoiceState>,
    server_update: Option<VoiceServerUpdate>,
}

impl VoiceRendezvous {
    #[must_use]
    pub const fn new(guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            guild_id,
            user_id,
            voice_state: None,
            server_update: None,
        }
    }

    /// Clears any previously collected state before a new join or channel move.
    pub fn reset(&mut self) {
        self.voice_state = None;
        self.server_update = None;
    }

    /// Applies one main-Gateway dispatch to this rendezvous.
    pub fn update_dispatch(
        &mut self,
        dispatch: &DispatchEvent,
    ) -> serde_json::Result<VoiceRendezvousStatus> {
        match dispatch.name.as_str() {
            "VOICE_STATE_UPDATE" => {
                let voice_state = serde_json::from_value::<VoiceState>(dispatch.data.clone())?;
                if voice_state.guild_id == Some(self.guild_id)
                    && voice_state.user_id == self.user_id
                {
                    self.voice_state = Some(voice_state);
                }
            }
            "VOICE_SERVER_UPDATE" => {
                let update = serde_json::from_value::<VoiceServerUpdate>(dispatch.data.clone())?;
                if update.guild_id == self.guild_id {
                    if update.endpoint.is_none() {
                        self.server_update = Some(update);
                        return Ok(VoiceRendezvousStatus::ServerUnavailable);
                    }
                    self.server_update = Some(update);
                }
            }
            _ => return Ok(VoiceRendezvousStatus::Pending),
        }

        Ok(self.status())
    }

    /// Returns the current rendezvous status without consuming collected state.
    #[must_use]
    pub fn status(&self) -> VoiceRendezvousStatus {
        let Some(voice_state) = &self.voice_state else {
            return VoiceRendezvousStatus::Pending;
        };
        let Some(channel_id) = voice_state.channel_id else {
            return VoiceRendezvousStatus::Pending;
        };

        let Some(server) = &self.server_update else {
            return VoiceRendezvousStatus::Pending;
        };
        let Some(endpoint) = &server.endpoint else {
            return VoiceRendezvousStatus::ServerUnavailable;
        };

        VoiceRendezvousStatus::Ready(VoiceConnectionInfo {
            guild_id: self.guild_id,
            channel_id,
            user_id: self.user_id,
            session_id: voice_state.session_id.clone(),
            token: server.token.clone(),
            endpoint: endpoint.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        gateway::DispatchEvent,
        model::{ChannelId, GuildId, UserId},
    };

    use super::{VoiceRendezvous, VoiceRendezvousStatus};

    #[test]
    fn accepts_voice_events_in_either_order() {
        let state = DispatchEvent {
            name: "VOICE_STATE_UPDATE".to_owned(),
            sequence: 1,
            data: json!({
                "guild_id": "10",
                "channel_id": "20",
                "user_id": "30",
                "session_id": "session",
                "deaf": false,
                "mute": false,
                "self_deaf": false,
                "self_mute": false,
                "self_video": false,
                "suppress": false
            }),
        };
        let server = DispatchEvent {
            name: "VOICE_SERVER_UPDATE".to_owned(),
            sequence: 2,
            data: json!({
                "token": "voice-token",
                "guild_id": "10",
                "endpoint": "voice.example.test:443"
            }),
        };

        let mut rendezvous = VoiceRendezvous::new(GuildId::new(10), UserId::new(30));
        assert_eq!(
            rendezvous.update_dispatch(&server).expect("server update"),
            VoiceRendezvousStatus::Pending
        );
        let VoiceRendezvousStatus::Ready(info) =
            rendezvous.update_dispatch(&state).expect("voice state")
        else {
            panic!("expected completed rendezvous");
        };
        assert_eq!(info.channel_id, ChannelId::new(20));
        assert_eq!(info.session_id, "session");
        assert_eq!(info.token, "voice-token");
    }

    #[test]
    fn null_endpoint_reports_server_unavailable() {
        let server = DispatchEvent {
            name: "VOICE_SERVER_UPDATE".to_owned(),
            sequence: 1,
            data: json!({
                "token": "voice-token",
                "guild_id": "10",
                "endpoint": null
            }),
        };
        let mut rendezvous = VoiceRendezvous::new(GuildId::new(10), UserId::new(30));

        assert_eq!(
            rendezvous.update_dispatch(&server).expect("server update"),
            VoiceRendezvousStatus::ServerUnavailable
        );
    }
}
