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
