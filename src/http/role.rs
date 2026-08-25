use std::collections::BTreeMap;

use reqwest::Method;
use serde::Serialize;

use crate::{
    Result,
    model::{GuildId, Permissions, Role, RoleColors, RoleId},
};

use super::{
    RestClient,
    encoding::audit_reason_headers,
    guild::guild_route,
    route::{RetrySafety, Route},
};

/// Parameters for creating a guild role.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CreateGuildRole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<RoleColors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,
}

/// One entry in Discord's bulk role-position update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModifyGuildRolePosition {
    pub id: RoleId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<i32>>,
}

/// Parameters for modifying one guild role.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyGuildRole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Option<Permissions>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Option<RoleColors>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<Option<bool>>,
}

impl RestClient {
    /// Lists every role in a guild.
    pub async fn get_guild_roles(&self, guild_id: GuildId) -> Result<Vec<Role>> {
        self.request_json::<Vec<Role>, ()>(
            roles_route(Method::GET, guild_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Returns one guild role.
    pub async fn get_guild_role(&self, guild_id: GuildId, role_id: RoleId) -> Result<Role> {
        self.request_json::<Role, ()>(
            role_route(Method::GET, guild_id, role_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Returns member counts keyed by role ID.
    pub async fn get_guild_role_member_counts(
        &self,
        guild_id: GuildId,
    ) -> Result<BTreeMap<RoleId, u64>> {
        self.request_json::<BTreeMap<RoleId, u64>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                "/roles/member-counts",
                "/guilds/{guild_id}/roles/member-counts",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Creates a guild role.
    pub async fn create_guild_role(
        &self,
        guild_id: GuildId,
        role: &CreateGuildRole,
        reason: Option<&str>,
    ) -> Result<Role> {
        self.request_json_with_headers(
            roles_route(Method::POST, guild_id, RetrySafety::Unsafe),
            Some(role),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Reorders guild roles and returns the resulting role list.
    pub async fn modify_guild_role_positions(
        &self,
        guild_id: GuildId,
        positions: &[ModifyGuildRolePosition],
        reason: Option<&str>,
    ) -> Result<Vec<Role>> {
        self.request_json_with_headers(
            roles_route(Method::PATCH, guild_id, RetrySafety::Unsafe),
            Some(positions),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Modifies one guild role.
    pub async fn modify_guild_role(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
        modify: &ModifyGuildRole,
        reason: Option<&str>,
    ) -> Result<Role> {
        self.request_json_with_headers(
            role_route(Method::PATCH, guild_id, role_id, RetrySafety::Unsafe),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Deletes one guild role.
    pub async fn delete_guild_role(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            role_route(Method::DELETE, guild_id, role_id, RetrySafety::Safe),
            None,
            audit_reason_headers(reason),
        )
        .await
    }
}

fn roles_route(method: Method, guild_id: GuildId, safety: RetrySafety) -> Route {
    guild_route(
        method,
        guild_id,
        "/roles",
        "/guilds/{guild_id}/roles",
        safety,
    )
}

fn role_route(method: Method, guild_id: GuildId, role_id: RoleId, safety: RetrySafety) -> Route {
    guild_route(
        method,
        guild_id,
        &format!("/roles/{role_id}"),
        "/guilds/{guild_id}/roles/{role_id}",
        safety,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::model::RoleId;

    use super::ModifyGuildRole;

    #[test]
    fn role_member_count_keys_decode_as_typed_ids() {
        let counts: BTreeMap<RoleId, u64> =
            serde_json::from_str(r#"{"42":7}"#).expect("role member counts");

        assert_eq!(counts[&RoleId::new(42)], 7);
    }

    #[test]
    fn role_icon_can_be_explicitly_cleared() {
        let modify = ModifyGuildRole {
            icon: Some(None),
            ..ModifyGuildRole::default()
        };
        let value = serde_json::to_value(modify).expect("modify role");

        assert!(value["icon"].is_null());
        assert!(value.get("name").is_none());
    }
}
