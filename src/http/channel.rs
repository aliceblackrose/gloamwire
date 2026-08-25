use reqwest::{Method, header::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::{
    Result,
    model::{
        AllowedMentions, AttachmentRequest, Channel, ChannelFlags, ChannelId, ChannelType,
        Component, DefaultReaction, Embed, EmojiId, MessageFlags, MessageId,
        PermissionOverwriteType, Permissions, Snowflake, StickerId, ThreadList, ThreadMember,
        UserId, WebhookId,
    },
};

use super::{
    RestClient, UploadFile,
    encoding::{QueryBuilder, audit_reason_headers},
    route::{RetrySafety, Route},
};

/// A partial permission overwrite accepted by channel create/modify endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PermissionOverwriteRequest {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: PermissionOverwriteType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Permissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Permissions>,
}

/// A forum/media tag accepted by channel create/modify endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForumTagRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<EmojiId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

/// Parameters accepted by Discord's Modify Channel endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ModifyChannel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<Option<Vec<PermissionOverwriteRequest>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<Option<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_tags: Option<Vec<ForumTagRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<Option<DefaultReaction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<Option<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<Vec<Snowflake>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
}

/// New status for a voice or stage channel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SetVoiceChannelStatus {
    pub status: Option<String>,
}

/// Permission overwrite written to one channel target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EditChannelPermission {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Permissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Permissions>,
    #[serde(rename = "type")]
    pub kind: PermissionOverwriteType,
}

/// Response returned when following an announcement channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct FollowedChannel {
    pub channel_id: ChannelId,
    pub webhook_id: WebhookId,
}

/// Credentials used to add a recipient to a group DM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupDmAddRecipient {
    pub access_token: String,
    pub nick: String,
}

/// Parameters for creating a thread from an existing message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartThreadFromMessage {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
}

/// Parameters for creating a thread without an existing message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartThread {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
}

/// Message fields accepted when starting a forum or media thread.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ForumThreadMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sticker_ids: Vec<StickerId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,
}

/// Parameters for creating a forum or media thread and its initial message.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StartForumThread {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
    pub message: ForumThreadMessage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_tags: Vec<Snowflake>,
}

/// Parameters for listing thread members.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThreadMembersQuery {
    pub with_member: bool,
    pub after: Option<UserId>,
    pub limit: Option<u8>,
}

impl ThreadMembersQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if self.with_member {
            query.push("with_member", true);
        }
        if let Some(after) = self.after {
            query.push("after", after);
        }
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        query.finish()
    }
}

/// Timestamp pagination for public/private archived thread lists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchivedThreadsQuery {
    pub before: Option<String>,
    pub limit: Option<u16>,
}

impl ArchivedThreadsQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(before) = &self.before {
            query.push_str("before", before);
        }
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        query.finish()
    }
}

/// Snowflake pagination for joined private archived threads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JoinedPrivateArchivedThreadsQuery {
    pub before: Option<ChannelId>,
    pub limit: Option<u16>,
}

impl JoinedPrivateArchivedThreadsQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(before) = self.before {
            query.push("before", before);
        }
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        query.finish()
    }
}

impl RestClient {
    /// Returns one channel by ID.
    pub async fn get_channel(&self, channel_id: ChannelId) -> Result<Channel> {
        self.request_json::<Channel, ()>(
            channel_route(
                Method::GET,
                channel_id,
                "",
                "/channels/{channel_id}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Updates a channel, optionally recording an audit-log reason.
    pub async fn modify_channel(
        &self,
        channel_id: ChannelId,
        modify: &ModifyChannel,
        reason: Option<&str>,
    ) -> Result<Channel> {
        self.request_json_with_headers(
            channel_route(
                Method::PATCH,
                channel_id,
                "",
                "/channels/{channel_id}",
                RetrySafety::Unsafe,
            ),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Sets the displayed status for a voice or stage channel.
    pub async fn set_voice_channel_status(
        &self,
        channel_id: ChannelId,
        status: &SetVoiceChannelStatus,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty(
            channel_route(
                Method::PUT,
                channel_id,
                "/voice-status",
                "/channels/{channel_id}/voice-status",
                RetrySafety::Safe,
            ),
            Some(status),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Deletes a guild channel or closes a private channel.
    pub async fn delete_channel(
        &self,
        channel_id: ChannelId,
        reason: Option<&str>,
    ) -> Result<Channel> {
        self.request_json_with_headers::<Channel, ()>(
            channel_route(
                Method::DELETE,
                channel_id,
                "",
                "/channels/{channel_id}",
                RetrySafety::Safe,
            ),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Creates or replaces one channel permission overwrite.
    pub async fn edit_channel_permission(
        &self,
        channel_id: ChannelId,
        overwrite_id: Snowflake,
        overwrite: &EditChannelPermission,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty(
            channel_route(
                Method::PUT,
                channel_id,
                &format!("/permissions/{overwrite_id}"),
                "/channels/{channel_id}/permissions/{overwrite_id}",
                RetrySafety::Safe,
            ),
            Some(overwrite),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Removes one channel permission overwrite.
    pub async fn delete_channel_permission(
        &self,
        channel_id: ChannelId,
        overwrite_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            channel_route(
                Method::DELETE,
                channel_id,
                &format!("/permissions/{overwrite_id}"),
                "/channels/{channel_id}/permissions/{overwrite_id}",
                RetrySafety::Safe,
            ),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Follows an announcement channel into another channel.
    pub async fn follow_announcement_channel(
        &self,
        channel_id: ChannelId,
        webhook_channel_id: ChannelId,
        reason: Option<&str>,
    ) -> Result<FollowedChannel> {
        #[derive(Serialize)]
        struct Body {
            webhook_channel_id: ChannelId,
        }

        self.request_json_with_headers(
            channel_route(
                Method::POST,
                channel_id,
                "/followers",
                "/channels/{channel_id}/followers",
                RetrySafety::Unsafe,
            ),
            Some(&Body { webhook_channel_id }),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Triggers a typing indicator for ten seconds.
    pub async fn trigger_typing_indicator(&self, channel_id: ChannelId) -> Result<()> {
        self.request_empty::<()>(
            channel_route(
                Method::POST,
                channel_id,
                "/typing",
                "/channels/{channel_id}/typing",
                RetrySafety::Unsafe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Adds a recipient to a group DM.
    pub async fn add_group_dm_recipient(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        recipient: &GroupDmAddRecipient,
    ) -> Result<()> {
        self.request_empty(
            recipient_route(Method::PUT, channel_id, user_id),
            Some(recipient),
            HeaderMap::new(),
        )
        .await
    }

    /// Removes a recipient from a group DM.
    pub async fn remove_group_dm_recipient(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<()> {
        self.request_empty::<()>(
            recipient_route(Method::DELETE, channel_id, user_id),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Starts a thread attached to an existing message.
    pub async fn start_thread_from_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        thread: &StartThreadFromMessage,
        reason: Option<&str>,
    ) -> Result<Channel> {
        self.request_json_with_headers(
            channel_route(
                Method::POST,
                channel_id,
                &format!("/messages/{message_id}/threads"),
                "/channels/{channel_id}/messages/{message_id}/threads",
                RetrySafety::Unsafe,
            ),
            Some(thread),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Starts a thread without attaching it to an existing message.
    pub async fn start_thread(
        &self,
        channel_id: ChannelId,
        thread: &StartThread,
        reason: Option<&str>,
    ) -> Result<Channel> {
        self.request_json_with_headers(
            threads_route(Method::POST, channel_id, RetrySafety::Unsafe),
            Some(thread),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Starts a forum/media thread without file uploads.
    pub async fn start_forum_thread(
        &self,
        channel_id: ChannelId,
        thread: &StartForumThread,
        reason: Option<&str>,
    ) -> Result<Channel> {
        self.request_json_with_headers(
            threads_route(Method::POST, channel_id, RetrySafety::Unsafe),
            Some(thread),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Starts a forum/media thread with streamed multipart uploads.
    pub async fn start_forum_thread_with_files(
        &self,
        channel_id: ChannelId,
        thread: &StartForumThread,
        files: &[UploadFile],
        reason: Option<&str>,
    ) -> Result<Channel> {
        let mut request = thread.clone();
        request
            .message
            .attachments
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_multipart_json(
            threads_route(Method::POST, channel_id, RetrySafety::Unsafe),
            &request,
            files,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Adds the current user to a thread.
    pub async fn join_thread(&self, thread_id: ChannelId) -> Result<()> {
        self.request_empty::<()>(
            thread_member_route(Method::PUT, thread_id, "@me"),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Adds another user to a thread.
    pub async fn add_thread_member(&self, thread_id: ChannelId, user_id: UserId) -> Result<()> {
        self.request_empty::<()>(
            thread_member_route(Method::PUT, thread_id, &user_id.to_string()),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Removes the current user from a thread.
    pub async fn leave_thread(&self, thread_id: ChannelId) -> Result<()> {
        self.request_empty::<()>(
            thread_member_route(Method::DELETE, thread_id, "@me"),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Removes another user from a thread.
    pub async fn remove_thread_member(&self, thread_id: ChannelId, user_id: UserId) -> Result<()> {
        self.request_empty::<()>(
            thread_member_route(Method::DELETE, thread_id, &user_id.to_string()),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Returns one thread member, optionally including guild-member details.
    pub async fn get_thread_member(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        with_member: bool,
    ) -> Result<ThreadMember> {
        let suffix = if with_member { "?with_member=true" } else { "" };
        self.request_json::<ThreadMember, ()>(
            channel_route(
                Method::GET,
                thread_id,
                &format!("/thread-members/{user_id}{suffix}"),
                "/channels/{channel_id}/thread-members/{user_id}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Lists users that joined a thread.
    pub async fn list_thread_members(
        &self,
        thread_id: ChannelId,
        query: &ThreadMembersQuery,
    ) -> Result<Vec<ThreadMember>> {
        self.request_json::<Vec<ThreadMember>, ()>(
            channel_route(
                Method::GET,
                thread_id,
                &format!("/thread-members{}", query.suffix()),
                "/channels/{channel_id}/thread-members",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Lists public archived threads in a channel.
    pub async fn list_public_archived_threads(
        &self,
        channel_id: ChannelId,
        query: &ArchivedThreadsQuery,
    ) -> Result<ThreadList> {
        self.list_archived_threads(channel_id, "public", query)
            .await
    }

    /// Lists private archived threads in a channel.
    pub async fn list_private_archived_threads(
        &self,
        channel_id: ChannelId,
        query: &ArchivedThreadsQuery,
    ) -> Result<ThreadList> {
        self.list_archived_threads(channel_id, "private", query)
            .await
    }

    /// Lists private archived threads joined by the current user.
    pub async fn list_joined_private_archived_threads(
        &self,
        channel_id: ChannelId,
        query: &JoinedPrivateArchivedThreadsQuery,
    ) -> Result<ThreadList> {
        self.request_json::<ThreadList, ()>(
            channel_route(
                Method::GET,
                channel_id,
                &format!("/users/@me/threads/archived/private{}", query.suffix()),
                "/channels/{channel_id}/users/@me/threads/archived/private",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    async fn list_archived_threads(
        &self,
        channel_id: ChannelId,
        kind: &'static str,
        query: &ArchivedThreadsQuery,
    ) -> Result<ThreadList> {
        self.request_json::<ThreadList, ()>(
            channel_route(
                Method::GET,
                channel_id,
                &format!("/threads/archived/{kind}{}", query.suffix()),
                "/channels/{channel_id}/threads/archived/{kind}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }
}

fn channel_route(
    method: Method,
    channel_id: ChannelId,
    suffix: &str,
    template: &'static str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/channels/{channel_id}{suffix}"),
        template,
        Some(channel_id.to_string()),
        safety,
    )
}

fn recipient_route(method: Method, channel_id: ChannelId, user_id: UserId) -> Route {
    channel_route(
        method,
        channel_id,
        &format!("/recipients/{user_id}"),
        "/channels/{channel_id}/recipients/{user_id}",
        RetrySafety::Safe,
    )
}

fn threads_route(method: Method, channel_id: ChannelId, safety: RetrySafety) -> Route {
    channel_route(
        method,
        channel_id,
        "/threads",
        "/channels/{channel_id}/threads",
        safety,
    )
}

fn thread_member_route(method: Method, thread_id: ChannelId, user: &str) -> Route {
    channel_route(
        method,
        thread_id,
        &format!("/thread-members/{user}"),
        "/channels/{channel_id}/thread-members/{user_id}",
        RetrySafety::Safe,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::UserId;

    use super::{ArchivedThreadsQuery, ModifyChannel, ThreadMembersQuery};

    #[test]
    fn nullable_channel_fields_distinguish_clear_from_omission() {
        let modify = ModifyChannel {
            topic: Some(None),
            parent_id: Some(None),
            ..ModifyChannel::default()
        };
        let value = serde_json::to_value(modify).expect("modify channel");

        assert!(value["topic"].is_null());
        assert!(value["parent_id"].is_null());
        assert!(value.get("name").is_none());
    }

    #[test]
    fn thread_member_query_uses_v10_pagination_switch() {
        let query = ThreadMembersQuery {
            with_member: true,
            after: Some(UserId::new(10)),
            limit: Some(100),
        };

        assert_eq!(query.suffix(), "?with_member=true&after=10&limit=100");
    }

    #[test]
    fn archived_thread_timestamps_are_percent_encoded() {
        let query = ArchivedThreadsQuery {
            before: Some("2026-08-25T20:00:00+00:00".to_owned()),
            limit: Some(50),
        };

        assert_eq!(
            query.suffix(),
            "?before=2026-08-25T20%3A00%3A00%2B00%3A00&limit=50"
        );
    }
}
