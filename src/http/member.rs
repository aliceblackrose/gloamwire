use reqwest::{Method, header::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::{
    Result,
    model::{ChannelId, GuildBan, GuildId, GuildMember, GuildMemberFlags, RoleId, UserId},
};

use super::{
    Pagination, RestClient,
    encoding::{QueryBuilder, audit_reason_headers},
    guild::guild_route,
    route::RetrySafety,
};

/// Pagination parameters for listing guild members.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuildMembersQuery {
    pub after: Option<UserId>,
    pub limit: Option<u16>,
}

impl GuildMembersQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(after) = self.after {
            query.push("after", after);
        }
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        query.finish()
    }
}

/// Prefix search parameters for guild members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchGuildMembersQuery {
    pub query: String,
    pub limit: Option<u16>,
}

impl SearchGuildMembersQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        query.push_str("query", &self.query);
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        query.finish()
    }
}

/// Parameters for adding an OAuth2-authorized user to a guild.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddGuildMember {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<RoleId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
}

/// Parameters for updating another guild member.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyGuildMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Option<Vec<RoleId>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Option<GuildMemberFlags>>,
}

/// Parameters for updating the current user's guild member profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyCurrentMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<Option<String>>,
}

/// Pagination parameters for guild bans.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuildBansQuery {
    pub pagination: Pagination<UserId>,
}

impl GuildBansQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(before) = self.pagination.before {
            query.push("before", before);
        }
        if let Some(after) = self.pagination.after {
            query.push("after", after);
        }
        if let Some(limit) = self.pagination.limit {
            query.push("limit", limit);
        }
        query.finish()
    }
}

/// Parameters for banning one guild member.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CreateGuildBan {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u32>,
}

/// Parameters for banning up to 200 guild members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BulkGuildBan {
    pub user_ids: Vec<UserId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u32>,
}

/// Per-user outcome returned by Discord's bulk-ban endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BulkGuildBanResponse {
    #[serde(default)]
    pub banned_users: Vec<UserId>,
    #[serde(default)]
    pub failed_users: Vec<UserId>,
}

/// Query parameters for estimating a guild prune.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildPruneQuery {
    pub days: Option<u8>,
    pub include_roles: Vec<RoleId>,
}

impl GuildPruneQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(days) = self.days {
            query.push("days", days);
        }
        if !self.include_roles.is_empty() {
            let roles = self
                .include_roles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            query.push_str("include_roles", &roles);
        }
        query.finish()
    }
}

/// Parameters for beginning a guild prune.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BeginGuildPrune {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_prune_count: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_roles: Vec<RoleId>,
}

/// Count returned by a guild prune estimate or execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct GuildPruneResult {
    pub pruned: Option<u64>,
}

impl RestClient {
    /// Returns one guild member.
    pub async fn get_guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<GuildMember> {
        self.request_json::<GuildMember, ()>(
            member_route(Method::GET, guild_id, user_id, "", RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Lists guild members using snowflake pagination.
    pub async fn list_guild_members(
        &self,
        guild_id: GuildId,
        query: &GuildMembersQuery,
    ) -> Result<Vec<GuildMember>> {
        self.request_json::<Vec<GuildMember>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                &format!("/members{}", query.suffix()),
                "/guilds/{guild_id}/members",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Searches guild members by username or nickname prefix.
    pub async fn search_guild_members(
        &self,
        guild_id: GuildId,
        query: &SearchGuildMembersQuery,
    ) -> Result<Vec<GuildMember>> {
        self.request_json::<Vec<GuildMember>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                &format!("/members/search{}", query.suffix()),
                "/guilds/{guild_id}/members/search",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Adds an OAuth2-authorized user to a guild.
    ///
    /// Returns `None` when the user was already a member and Discord responds
    /// with `204 No Content`.
    pub async fn add_guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        member: &AddGuildMember,
    ) -> Result<Option<GuildMember>> {
        self.request_optional_json(
            member_route(Method::PUT, guild_id, user_id, "", RetrySafety::Safe),
            Some(member),
            HeaderMap::new(),
        )
        .await
    }

    /// Updates another guild member.
    pub async fn modify_guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        modify: &ModifyGuildMember,
        reason: Option<&str>,
    ) -> Result<GuildMember> {
        self.request_json_with_headers(
            member_route(Method::PATCH, guild_id, user_id, "", RetrySafety::Unsafe),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Updates the current user's guild member profile.
    pub async fn modify_current_member(
        &self,
        guild_id: GuildId,
        modify: &ModifyCurrentMember,
        reason: Option<&str>,
    ) -> Result<GuildMember> {
        self.request_json_with_headers(
            guild_route(
                Method::PATCH,
                guild_id,
                "/members/@me",
                "/guilds/{guild_id}/members/@me",
                RetrySafety::Unsafe,
            ),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Adds a role to a guild member.
    pub async fn add_guild_member_role(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            member_role_route(Method::PUT, guild_id, user_id, role_id),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Removes a role from a guild member.
    pub async fn remove_guild_member_role(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            member_role_route(Method::DELETE, guild_id, user_id, role_id),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Kicks a member from a guild.
    pub async fn remove_guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            member_route(Method::DELETE, guild_id, user_id, "", RetrySafety::Safe),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Lists guild bans.
    pub async fn get_guild_bans(
        &self,
        guild_id: GuildId,
        query: &GuildBansQuery,
    ) -> Result<Vec<GuildBan>> {
        self.request_json::<Vec<GuildBan>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                &format!("/bans{}", query.suffix()),
                "/guilds/{guild_id}/bans",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Returns one guild ban.
    pub async fn get_guild_ban(&self, guild_id: GuildId, user_id: UserId) -> Result<GuildBan> {
        self.request_json::<GuildBan, ()>(
            ban_route(Method::GET, guild_id, user_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Bans one user from a guild.
    pub async fn create_guild_ban(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        ban: &CreateGuildBan,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty(
            ban_route(Method::PUT, guild_id, user_id, RetrySafety::Safe),
            Some(ban),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Removes one guild ban.
    pub async fn remove_guild_ban(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            ban_route(Method::DELETE, guild_id, user_id, RetrySafety::Safe),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Bans up to 200 users from a guild.
    pub async fn bulk_guild_ban(
        &self,
        guild_id: GuildId,
        ban: &BulkGuildBan,
        reason: Option<&str>,
    ) -> Result<BulkGuildBanResponse> {
        self.request_json_with_headers(
            guild_route(
                Method::POST,
                guild_id,
                "/bulk-ban",
                "/guilds/{guild_id}/bulk-ban",
                RetrySafety::Unsafe,
            ),
            Some(ban),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Estimates how many members a guild prune would remove.
    pub async fn get_guild_prune_count(
        &self,
        guild_id: GuildId,
        query: &GuildPruneQuery,
    ) -> Result<GuildPruneResult> {
        self.request_json::<GuildPruneResult, ()>(
            guild_route(
                Method::GET,
                guild_id,
                &format!("/prune{}", query.suffix()),
                "/guilds/{guild_id}/prune",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Begins a guild prune operation.
    pub async fn begin_guild_prune(
        &self,
        guild_id: GuildId,
        prune: &BeginGuildPrune,
        reason: Option<&str>,
    ) -> Result<GuildPruneResult> {
        self.request_json_with_headers(
            guild_route(
                Method::POST,
                guild_id,
                "/prune",
                "/guilds/{guild_id}/prune",
                RetrySafety::Unsafe,
            ),
            Some(prune),
            audit_reason_headers(reason),
        )
        .await
    }
}

fn member_route(
    method: Method,
    guild_id: GuildId,
    user_id: UserId,
    suffix: &str,
    safety: RetrySafety,
) -> super::route::Route {
    guild_route(
        method,
        guild_id,
        &format!("/members/{user_id}{suffix}"),
        "/guilds/{guild_id}/members/{user_id}",
        safety,
    )
}

fn member_role_route(
    method: Method,
    guild_id: GuildId,
    user_id: UserId,
    role_id: RoleId,
) -> super::route::Route {
    member_route(
        method,
        guild_id,
        user_id,
        &format!("/roles/{role_id}"),
        RetrySafety::Safe,
    )
}

fn ban_route(
    method: Method,
    guild_id: GuildId,
    user_id: UserId,
    safety: RetrySafety,
) -> super::route::Route {
    guild_route(
        method,
        guild_id,
        &format!("/bans/{user_id}"),
        "/guilds/{guild_id}/bans/{user_id}",
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::{RoleId, UserId};

    use super::{GuildBansQuery, GuildPruneQuery, ModifyGuildMember, SearchGuildMembersQuery};

    #[test]
    fn member_search_percent_encodes_prefix() {
        let query = SearchGuildMembersQuery {
            query: "gloam wire".to_owned(),
            limit: Some(25),
        };

        assert_eq!(query.suffix(), "?query=gloam%20wire&limit=25");
    }

    #[test]
    fn member_updates_can_explicitly_disconnect_voice() {
        let modify = ModifyGuildMember {
            channel_id: Some(None),
            communication_disabled_until: Some(None),
            ..ModifyGuildMember::default()
        };
        let value = serde_json::to_value(modify).expect("modify member");

        assert!(value["channel_id"].is_null());
        assert!(value["communication_disabled_until"].is_null());
    }

    #[test]
    fn ban_pagination_keeps_cursors_mutually_exclusive() {
        let query = GuildBansQuery {
            pagination: super::Pagination::new()
                .before(UserId::new(20))
                .after(UserId::new(10))
                .limit(1000),
        };

        assert_eq!(query.suffix(), "?after=10&limit=1000");
    }

    #[test]
    fn prune_roles_use_a_comma_delimited_query_value() {
        let query = GuildPruneQuery {
            days: Some(7),
            include_roles: vec![RoleId::new(1), RoleId::new(2)],
        };

        assert_eq!(query.suffix(), "?days=7&include_roles=1%2C2");
    }
}
