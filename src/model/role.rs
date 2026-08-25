use serde::{Deserialize, Serialize};

use super::{Permissions, RoleId};

/// Discord role color information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleColors {
    pub primary_color: u32,
    pub secondary_color: Option<u32>,
    pub tertiary_color: Option<u32>,
}

/// A Discord role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub colors: Option<RoleColors>,
    pub hoist: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub unicode_emoji: Option<String>,
    pub position: i32,
    pub permissions: Permissions,
    pub managed: bool,
    pub mentionable: bool,
    #[serde(default)]
    pub flags: u64,
}
