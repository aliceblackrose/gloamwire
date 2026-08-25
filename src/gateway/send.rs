use serde::{Serialize, Serializer};

use crate::{
    error::{Error, Result},
    model::{ChannelId, GuildId, UserId},
};

/// Activity type used in a Gateway presence update.
///
/// This is a numeric newtype so callers can use newly introduced Discord
/// activity types without waiting for a Gloamwire release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct GatewayActivityType(pub u8);

impl GatewayActivityType {
    pub const PLAYING: Self = Self(0);
    pub const STREAMING: Self = Self(1);
    pub const LISTENING: Self = Self(2);
    pub const WATCHING: Self = Self(3);
    pub const CUSTOM: Self = Self(4);
    pub const COMPETING: Self = Self(5);
}

/// Activity fields that Discord allows bot users to set through Update Presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GatewayActivity {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: GatewayActivityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl GatewayActivity {
    #[must_use]
    pub fn new(name: impl Into<String>, kind: GatewayActivityType) -> Self {
        Self {
            name: name.into(),
            kind,
            state: None,
            url: None,
        }
    }

    #[must_use]
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

/// Status accepted by Discord's Gateway Update Presence event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GatewayStatus {
    Online,
    DoNotDisturb,
    Idle,
    Invisible,
    Offline,
}

impl Serialize for GatewayStatus {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Online => "online",
            Self::DoNotDisturb => "dnd",
            Self::Idle => "idle",
            Self::Invisible => "invisible",
            Self::Offline => "offline",
        })
    }
}

/// Payload for Gateway opcode 3 (Update Presence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdatePresence {
    /// Unix time in milliseconds when the client became idle, or `None` when not idle.
    pub since: Option<u64>,
    /// Activities shown for the bot.
    pub activities: Vec<GatewayActivity>,
    /// New bot status.
    pub status: GatewayStatus,
    /// Whether the bot should be considered AFK.
    pub afk: bool,
}

impl UpdatePresence {
    #[must_use]
    pub fn new(status: GatewayStatus) -> Self {
        Self {
            since: None,
            activities: Vec::new(),
            status,
            afk: false,
        }
    }

    #[must_use]
    pub fn with_activity(mut self, activity: GatewayActivity) -> Self {
        self.activities.push(activity);
        self
    }

    #[must_use]
    pub const fn idle_since(mut self, since: u64) -> Self {
        self.since = Some(since);
        self.afk = true;
        self
    }

    #[must_use]
    pub const fn with_afk(mut self, afk: bool) -> Self {
        self.afk = afk;
        self
    }
}

/// Payload for Gateway opcode 4 (Update Voice State).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UpdateVoiceState {
    pub guild_id: GuildId,
    /// Target voice channel, or `None` to disconnect.
    pub channel_id: Option<ChannelId>,
    pub self_mute: bool,
    pub self_deaf: bool,
}

impl UpdateVoiceState {
    #[must_use]
    pub const fn new(guild_id: GuildId, channel_id: Option<ChannelId>) -> Self {
        Self {
            guild_id,
            channel_id,
            self_mute: false,
            self_deaf: false,
        }
    }

    #[must_use]
    pub const fn with_self_mute(mut self, self_mute: bool) -> Self {
        self.self_mute = self_mute;
        self
    }

    #[must_use]
    pub const fn with_self_deaf(mut self, self_deaf: bool) -> Self {
        self.self_deaf = self_deaf;
        self
    }
}

/// Payload for Gateway opcode 8 (Request Guild Members).
///
/// Construct this with [`Self::query`] or [`Self::users`] so exactly one of
/// Discord's `query` and `user_ids` selectors is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestGuildMembers {
    pub guild_id: GuildId,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "is_false")]
    presences: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_ids: Option<Vec<UserId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
}

impl RequestGuildMembers {
    /// Requests members whose usernames start with `query`.
    ///
    /// An empty query with a limit of zero requests the entire member list when
    /// the connection has the required intent.
    #[must_use]
    pub fn query(guild_id: GuildId, query: impl Into<String>, limit: u32) -> Self {
        Self {
            guild_id,
            query: Some(query.into()),
            limit: Some(limit),
            presences: false,
            user_ids: None,
            nonce: None,
        }
    }

    /// Requests specific guild members by user ID.
    pub fn users(
        guild_id: GuildId,
        user_ids: impl IntoIterator<Item = UserId>,
    ) -> Result<Self> {
        let user_ids: Vec<_> = user_ids.into_iter().collect();
        if user_ids.is_empty() {
            return Err(Error::InvalidGatewaySendEvent(
                "Request Guild Members requires at least one user ID".to_owned(),
            ));
        }
        if user_ids.len() > 100 {
            return Err(Error::InvalidGatewaySendEvent(
                "Request Guild Members accepts at most 100 user IDs".to_owned(),
            ));
        }

        Ok(Self {
            guild_id,
            query: None,
            limit: None,
            presences: false,
            user_ids: Some(user_ids),
            nonce: None,
        })
    }

    #[must_use]
    pub const fn with_presences(mut self, presences: bool) -> Self {
        self.presences = presences;
        self
    }

    /// Adds a response nonce, rejecting values above Discord's 32-byte limit.
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Result<Self> {
        let nonce = nonce.into();
        if nonce.len() > 32 {
            return Err(Error::InvalidGatewaySendEvent(
                "Request Guild Members nonce must not exceed 32 bytes".to_owned(),
            ));
        }
        self.nonce = Some(nonce);
        Ok(self)
    }
}

/// Payload for Gateway opcode 31 (Request Soundboard Sounds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestSoundboardSounds {
    pub guild_ids: Vec<GuildId>,
}

impl RequestSoundboardSounds {
    #[must_use]
    pub fn new(guild_ids: impl IntoIterator<Item = GuildId>) -> Self {
        Self {
            guild_ids: guild_ids.into_iter().collect(),
        }
    }
}

/// A field requested by Gateway opcode 43 (Request Channel Info).
///
/// [`Self::new`] accepts arbitrary field names so future Discord fields remain
/// usable without a Gloamwire release.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ChannelInfoField(String);

impl ChannelInfoField {
    #[must_use]
    pub fn new(field: impl Into<String>) -> Self {
        Self(field.into())
    }

    #[must_use]
    pub fn status() -> Self {
        Self::new("status")
    }

    #[must_use]
    pub fn voice_start_time() -> Self {
        Self::new("voice_start_time")
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ChannelInfoField {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ChannelInfoField {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Payload for Gateway opcode 43 (Request Channel Info).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestChannelInfo {
    pub guild_id: GuildId,
    pub fields: Vec<ChannelInfoField>,
}

impl RequestChannelInfo {
    #[must_use]
    pub fn new(
        guild_id: GuildId,
        fields: impl IntoIterator<Item = ChannelInfoField>,
    ) -> Self {
        Self {
            guild_id,
            fields: fields.into_iter().collect(),
        }
    }
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn presence_uses_discord_status_and_activity_shape() {
        let presence = UpdatePresence::new(GatewayStatus::DoNotDisturb).with_activity(
            GatewayActivity::new("Gloamwire", GatewayActivityType::PLAYING)
                .with_state("testing"),
        );

        assert_eq!(
            serde_json::to_value(presence).expect("serialize"),
            json!({
                "since": null,
                "activities": [{"name":"Gloamwire", "type":0, "state":"testing"}],
                "status": "dnd",
                "afk": false
            })
        );
    }

    #[test]
    fn voice_disconnect_serializes_null_channel() {
        let update = UpdateVoiceState::new(GuildId::new(1), None);
        let value = serde_json::to_value(update).expect("serialize");
        assert_eq!(value["channel_id"], Value::Null);
    }

    #[test]
    fn member_query_has_exact_selector_shape() {
        let request = RequestGuildMembers::query(GuildId::new(1), "ab", 100)
            .with_nonce("chunk-1")
            .expect("valid nonce");
        let value = serde_json::to_value(request).expect("serialize");

        assert_eq!(value["query"], "ab");
        assert_eq!(value["limit"], 100);
        assert!(value.get("user_ids").is_none());
        assert_eq!(value["nonce"], "chunk-1");
    }

    #[test]
    fn member_user_request_enforces_discord_limit() {
        let users = (0..101).map(UserId::new);
        assert!(RequestGuildMembers::users(GuildId::new(1), users).is_err());
    }

    #[test]
    fn member_request_enforces_nonce_byte_limit() {
        let request = RequestGuildMembers::query(GuildId::new(1), "", 0);
        assert!(request.with_nonce("x".repeat(33)).is_err());
    }

    #[test]
    fn channel_info_supports_current_and_future_fields() {
        let request = RequestChannelInfo::new(
            GuildId::new(1),
            [
                ChannelInfoField::status(),
                ChannelInfoField::voice_start_time(),
                ChannelInfoField::new("future_field"),
            ],
        );
        let value = serde_json::to_value(request).expect("serialize");
        assert_eq!(value["fields"], json!(["status", "voice_start_time", "future_field"]));
    }
}
