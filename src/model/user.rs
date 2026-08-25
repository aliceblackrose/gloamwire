use serde::{Deserialize, Serialize};

use super::UserId;

/// A Discord user object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// The user's unique ID.
    pub id: UserId,
    /// The user's username.
    pub username: String,
    /// The user's display name, when one is configured.
    #[serde(default)]
    pub global_name: Option<String>,
    /// The user's legacy discriminator, when supplied by Discord.
    #[serde(default)]
    pub discriminator: Option<String>,
    /// Whether the user belongs to an OAuth2 application bot.
    #[serde(default)]
    pub bot: Option<bool>,
    /// The user's avatar hash.
    #[serde(default)]
    pub avatar: Option<String>,
}

/// A partial Discord user object.
///
/// Gateway payloads such as `PRESENCE_UPDATE` may legally include only `id`.
/// Keeping this separate from [`User`] prevents valid partial payloads from
/// being rejected while retaining a strict model for complete user objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialUser {
    /// The user's unique ID. This is the only guaranteed field.
    pub id: UserId,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub bot: Option<bool>,
    #[serde(default)]
    pub avatar: Option<String>,
}
