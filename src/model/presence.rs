use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{GuildId, User};

/// Presence status received from Discord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PresenceStatus(pub String);

impl PresenceStatus {
    pub const ONLINE: &'static str = "online";
    pub const DND: &'static str = "dnd";
    pub const IDLE: &'static str = "idle";
    pub const OFFLINE: &'static str = "offline";
}

/// Per-platform client status from a Presence Update event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientStatus {
    #[serde(default)]
    pub desktop: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub web: Option<String>,
    #[serde(default)]
    pub embedded: Option<String>,
}

/// A Discord Gateway Presence Update.
///
/// Activities remain raw JSON for now so additions to Discord's rich-presence
/// schema cannot make the core presence event fail to deserialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub user: User,
    pub guild_id: GuildId,
    pub status: PresenceStatus,
    #[serde(default)]
    pub activities: Vec<Value>,
    #[serde(default)]
    pub client_status: ClientStatus,
}
