use reqwest::{Method, header::HeaderMap};
use serde::Serialize;

use crate::{
    Result,
    model::{
        Channel, ChannelFlags, ChannelId, ChannelType, DefaultReaction, Guild, GuildId,
        GuildPreview, ThreadList,
    },
};

use super::{
    RestClient,
    channel::{ForumTagRequest, PermissionOverwriteRequest},
    encoding::{QueryBuilder, audit_reason_headers},
    route::{RetrySafety, Route},
};

/// Parameters accepted by Discord's Modify Guild endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyGuild {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<Option<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_message_notifications: Option<Option<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_content_filter: Option<Option<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_channel_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_splash: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_channel_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_updates_channel_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_locale: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_progress_bar_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_alerts_channel_id: Option<Option<ChannelId>>,
}

/// Parameters accepted by Discord's Create Guild Channel endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateGuildChannel {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<Vec<PermissionOverwriteRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ChannelId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<DefaultReaction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_tags: Option<Vec<ForumTagRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
}

impl CreateGuildChannel {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: None,
            topic: None,
            bitrate: None,
            user_limit: None,
            rate_limit_per_user: None,
            position: None,
            permission_overwrites: None,
            parent_id: None,
            nsfw: None,
            rtc_region: None,
            video_quality_mode: None,
            default_auto_archive_duration: None,
            default_reaction_emoji: None,
            available_tags: None,
            default_sort_order: None,
            default_forum_layout: None,
            default_thread_rate_limit_per_user: None,
            flags: None,
        }
    }
}

/// One entry in Discord's bulk channel-position update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModifyGuildChannelPosition {
    pub id: ChannelId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_permissions: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Option<ChannelFlags>>,
}

impl RestClient {
    /// Returns a guild, optionally including approximate member/presence counts.
    pub async fn get_guild(&self, guild_id: GuildId, with_counts: bool) -> Result<Guild> {
        let mut query = QueryBuilder::default();
        if with_counts {
            query.push("with_counts", true);
        }
        self.request_json::<Guild, ()>(
            guild_route(
                Method::GET,
                guild_id,
                &query.finish(),
                "/guilds/{guild_id}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Returns public preview metadata for a guild.
    pub async fn get_guild_preview(&self, guild_id: GuildId) -> Result<GuildPreview> {
        self.request_json::<GuildPreview, ()>(
            guild_route(
                Method::GET,
                guild_id,
                "/preview",
                "/guilds/{guild_id}/preview",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Updates guild settings, optionally recording an audit-log reason.
    pub async fn modify_guild(
        &self,
        guild_id: GuildId,
        modify: &ModifyGuild,
        reason: Option<&str>,
    ) -> Result<Guild> {
        self.request_json_with_headers(
            guild_route(
                Method::PATCH,
                guild_id,
                "",
                "/guilds/{guild_id}",
                RetrySafety::Unsafe,
            ),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Permanently deletes a guild owned by the current user.
    pub async fn delete_guild(&self, guild_id: GuildId) -> Result<()> {
        self.request_empty::<()>(
            guild_route(
                Method::DELETE,
                guild_id,
                "",
                "/guilds/{guild_id}",
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Lists the guild's non-thread channels.
    pub async fn get_guild_channels(&self, guild_id: GuildId) -> Result<Vec<Channel>> {
        self.request_json::<Vec<Channel>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                "/channels",
                "/guilds/{guild_id}/channels",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Creates a guild channel, optionally recording an audit-log reason.
    pub async fn create_guild_channel(
        &self,
        guild_id: GuildId,
        channel: &CreateGuildChannel,
        reason: Option<&str>,
    ) -> Result<Channel> {
        self.request_json_with_headers(
            guild_route(
                Method::POST,
                guild_id,
                "/channels",
                "/guilds/{guild_id}/channels",
                RetrySafety::Unsafe,
            ),
            Some(channel),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Reorders or reparents a set of guild channels.
    pub async fn modify_guild_channel_positions(
        &self,
        guild_id: GuildId,
        positions: &[ModifyGuildChannelPosition],
    ) -> Result<()> {
        self.request_empty(
            guild_route(
                Method::PATCH,
                guild_id,
                "/channels",
                "/guilds/{guild_id}/channels",
                RetrySafety::Unsafe,
            ),
            Some(positions),
            HeaderMap::new(),
        )
        .await
    }

    /// Lists all active public and private threads in a guild.
    pub async fn list_active_guild_threads(&self, guild_id: GuildId) -> Result<ThreadList> {
        self.request_json::<ThreadList, ()>(
            guild_route(
                Method::GET,
                guild_id,
                "/threads/active",
                "/guilds/{guild_id}/threads/active",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }
}

pub(crate) fn guild_route(
    method: Method,
    guild_id: GuildId,
    suffix: &str,
    template: &'static str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/guilds/{guild_id}{suffix}"),
        template,
        Some(guild_id.to_string()),
        safety,
    )
}

#[cfg(test)]
mod tests {
    use super::{CreateGuildChannel, ModifyGuild};

    #[test]
    fn nullable_guild_fields_distinguish_clear_from_omission() {
        let modify = ModifyGuild {
            icon: Some(None),
            rules_channel_id: Some(None),
            ..ModifyGuild::default()
        };
        let value = serde_json::to_value(modify).expect("modify guild");

        assert!(value["icon"].is_null());
        assert!(value["rules_channel_id"].is_null());
        assert!(value.get("name").is_none());
    }

    #[test]
    fn create_channel_requires_only_a_name() {
        let value =
            serde_json::to_value(CreateGuildChannel::new("general")).expect("create channel");

        assert_eq!(value, serde_json::json!({"name":"general"}));
    }
}
