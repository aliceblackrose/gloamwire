use reqwest::{Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    Result,
    model::{
        BulkDeleteMessages, Channel, ChannelId, ChannelPins, CreateMessage, EditMessage, GuildId,
        Message, MessageId, ReactionType, RoleId, User, UserId,
    },
};

use super::{
    Pagination, RestClient, UploadFile,
    encoding::{QueryBuilder, percent_encode},
    route::{RetrySafety, Route},
};

const AUDIT_LOG_REASON: HeaderName = HeaderName::from_static("x-audit-log-reason");

/// Pagination parameters for Discord's Get Channel Messages endpoint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageListQuery {
    pub around: Option<MessageId>,
    pub pagination: Pagination<MessageId>,
}

impl MessageListQuery {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            around: None,
            pagination: Pagination::new(),
        }
    }

    #[must_use]
    pub fn around(mut self, message_id: MessageId) -> Self {
        self.around = Some(message_id);
        self.pagination.before = None;
        self.pagination.after = None;
        self
    }

    #[must_use]
    pub fn before(mut self, message_id: MessageId) -> Self {
        self.around = None;
        self.pagination = self.pagination.before(message_id);
        self
    }

    #[must_use]
    pub fn after(mut self, message_id: MessageId) -> Self {
        self.around = None;
        self.pagination = self.pagination.after(message_id);
        self
    }

    #[must_use]
    pub const fn limit(mut self, limit: u16) -> Self {
        self.pagination.limit = Some(limit);
        self
    }

    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(around) = self.around {
            query.push("around", around);
        }
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

/// Filters supported by Discord's current Search Guild Messages endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageSearchQuery {
    pub limit: Option<u8>,
    pub offset: Option<u16>,
    pub max_id: Option<MessageId>,
    pub min_id: Option<MessageId>,
    pub slop: Option<u8>,
    pub content: Option<String>,
    pub channel_ids: Vec<ChannelId>,
    pub author_types: Vec<String>,
    pub author_ids: Vec<UserId>,
    pub mentions: Vec<UserId>,
    pub mention_role_ids: Vec<RoleId>,
    pub mention_everyone: Option<bool>,
    pub replied_to_user_ids: Vec<UserId>,
    pub replied_to_message_ids: Vec<MessageId>,
    pub pinned: Option<bool>,
    pub has: Vec<String>,
    pub embed_types: Vec<String>,
    pub embed_providers: Vec<String>,
    pub link_hostnames: Vec<String>,
    pub attachment_filenames: Vec<String>,
    pub attachment_extensions: Vec<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub include_nsfw: Option<bool>,
}

impl MessageSearchQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        push_option(&mut query, "limit", self.limit);
        push_option(&mut query, "offset", self.offset);
        push_option(&mut query, "max_id", self.max_id);
        push_option(&mut query, "min_id", self.min_id);
        push_option(&mut query, "slop", self.slop);
        push_option_str(&mut query, "content", self.content.as_deref());
        push_many(&mut query, "channel_id", &self.channel_ids);
        push_many_str(&mut query, "author_type", &self.author_types);
        push_many(&mut query, "author_id", &self.author_ids);
        push_many(&mut query, "mentions", &self.mentions);
        push_many(&mut query, "mentions_role_id", &self.mention_role_ids);
        push_option(&mut query, "mention_everyone", self.mention_everyone);
        push_many(&mut query, "replied_to_user_id", &self.replied_to_user_ids);
        push_many(
            &mut query,
            "replied_to_message_id",
            &self.replied_to_message_ids,
        );
        push_option(&mut query, "pinned", self.pinned);
        push_many_str(&mut query, "has", &self.has);
        push_many_str(&mut query, "embed_type", &self.embed_types);
        push_many_str(&mut query, "embed_provider", &self.embed_providers);
        push_many_str(&mut query, "link_hostname", &self.link_hostnames);
        push_many_str(
            &mut query,
            "attachment_filename",
            &self.attachment_filenames,
        );
        push_many_str(
            &mut query,
            "attachment_extension",
            &self.attachment_extensions,
        );
        push_option_str(&mut query, "sort_by", self.sort_by.as_deref());
        push_option_str(&mut query, "sort_order", self.sort_order.as_deref());
        push_option(&mut query, "include_nsfw", self.include_nsfw);
        query.finish()
    }
}

/// Successful guild-message search results.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MessageSearchResult {
    pub doing_deep_historical_index: bool,
    #[serde(default)]
    pub documents_indexed: Option<u64>,
    pub total_results: u64,
    #[serde(default)]
    pub messages: Vec<Vec<Message>>,
    #[serde(default)]
    pub threads: Vec<Channel>,
    /// Thread-member payloads are preserved until a dedicated thread-member model is added.
    #[serde(default)]
    pub members: Vec<Value>,
}

/// Search response returned while Discord is still indexing a guild.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MessageSearchIndexing {
    pub message: String,
    pub code: i64,
    #[serde(default)]
    pub documents_indexed: Option<u64>,
    pub retry_after: f64,
}

/// Response from Discord's Search Guild Messages endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum MessageSearchResponse {
    Results(MessageSearchResult),
    Indexing(MessageSearchIndexing),
}

/// Parameters for Discord's Get Reactions endpoint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReactionUsersQuery {
    pub kind: Option<ReactionType>,
    pub after: Option<UserId>,
    pub limit: Option<u16>,
}

impl ReactionUsersQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(kind) = self.kind {
            query.push("type", kind.0);
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

/// Parameters for Discord's current Get Channel Pins endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelPinsQuery {
    pub before: Option<String>,
    pub limit: Option<u8>,
}

impl ChannelPinsQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        push_option_str(&mut query, "before", self.before.as_deref());
        push_option(&mut query, "limit", self.limit);
        query.finish()
    }
}

impl RestClient {
    /// Returns messages in a channel using before/after/around pagination.
    pub async fn get_channel_messages(
        &self,
        channel_id: ChannelId,
        query: &MessageListQuery,
    ) -> Result<Vec<Message>> {
        self.request_json::<Vec<Message>, ()>(
            channel_route(
                Method::GET,
                format!("/channels/{channel_id}/messages{}", query.suffix()),
                "/channels/{channel_id}/messages",
                channel_id,
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Searches messages in a guild using Discord's current indexed search API.
    pub async fn search_guild_messages(
        &self,
        guild_id: GuildId,
        query: &MessageSearchQuery,
    ) -> Result<MessageSearchResponse> {
        self.request_json::<MessageSearchResponse, ()>(
            Route::new(
                Method::GET,
                format!("/guilds/{guild_id}/messages/search{}", query.suffix()),
                "/guilds/{guild_id}/messages/search",
                Some(guild_id.to_string()),
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Returns a single message from a channel.
    pub async fn get_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<Message> {
        self.request_json::<Message, ()>(
            message_route(Method::GET, channel_id, message_id, "", RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Creates a message with one or more multipart file uploads.
    pub async fn create_message_with_files(
        &self,
        channel_id: ChannelId,
        message: &CreateMessage,
        files: &[UploadFile],
    ) -> Result<Message> {
        let mut request = message.clone();
        request
            .attachments
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_multipart_json(
            Route::create_message(channel_id),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }

    /// Crossposts a message from an announcement channel.
    pub async fn crosspost_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<Message> {
        self.request_json::<Message, ()>(
            message_route(
                Method::POST,
                channel_id,
                message_id,
                "/crosspost",
                RetrySafety::Unsafe,
            ),
            None,
        )
        .await
    }

    /// Adds the current user reaction to a message.
    pub async fn create_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: &str,
    ) -> Result<()> {
        self.request_empty::<()>(
            reaction_route(Method::PUT, channel_id, message_id, emoji, "/@me"),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Removes the current user's reaction from a message.
    pub async fn delete_own_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: &str,
    ) -> Result<()> {
        self.request_empty::<()>(
            reaction_route(Method::DELETE, channel_id, message_id, emoji, "/@me"),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Removes another user's reaction from a message.
    pub async fn delete_user_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: &str,
        user_id: UserId,
    ) -> Result<()> {
        self.request_empty::<()>(
            reaction_route(
                Method::DELETE,
                channel_id,
                message_id,
                emoji,
                &format!("/{user_id}"),
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Returns users who reacted to a message with one emoji.
    pub async fn get_reactions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: &str,
        query: &ReactionUsersQuery,
    ) -> Result<Vec<User>> {
        let encoded = percent_encode(emoji);
        self.request_json::<Vec<User>, ()>(
            channel_route(
                Method::GET,
                format!(
                    "/channels/{channel_id}/messages/{message_id}/reactions/{encoded}{}",
                    query.suffix()
                ),
                "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}",
                channel_id,
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Removes every reaction from a message.
    pub async fn delete_all_reactions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        self.request_empty::<()>(
            message_route(
                Method::DELETE,
                channel_id,
                message_id,
                "/reactions",
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Removes every reaction for one emoji from a message.
    pub async fn delete_all_reactions_for_emoji(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: &str,
    ) -> Result<()> {
        self.request_empty::<()>(
            reaction_route(Method::DELETE, channel_id, message_id, emoji, ""),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Edits a message without uploading new files.
    pub async fn edit_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        edit: &EditMessage,
    ) -> Result<Message> {
        self.request_json(
            message_route(Method::PATCH, channel_id, message_id, "", RetrySafety::Unsafe),
            Some(edit),
        )
        .await
    }

    /// Edits a message and appends multipart file uploads.
    pub async fn edit_message_with_files(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        edit: &EditMessage,
        files: &[UploadFile],
    ) -> Result<Message> {
        let mut request = edit.clone();
        request
            .attachments
            .get_or_insert_default()
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_multipart_json(
            message_route(Method::PATCH, channel_id, message_id, "", RetrySafety::Unsafe),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }

    /// Deletes a message.
    pub async fn delete_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        self.delete_message_with_reason(channel_id, message_id, None)
            .await
    }

    /// Deletes a message with an optional audit-log reason.
    pub async fn delete_message_with_reason(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            message_route(Method::DELETE, channel_id, message_id, "", RetrySafety::Safe),
            None,
            audit_headers(reason),
        )
        .await
    }

    /// Bulk-deletes 2–100 recent messages from a guild channel.
    pub async fn bulk_delete_messages(
        &self,
        channel_id: ChannelId,
        message_ids: impl IntoIterator<Item = MessageId>,
    ) -> Result<()> {
        self.bulk_delete_messages_with_reason(channel_id, message_ids, None)
            .await
    }

    /// Bulk-deletes messages with an optional audit-log reason.
    pub async fn bulk_delete_messages_with_reason(
        &self,
        channel_id: ChannelId,
        message_ids: impl IntoIterator<Item = MessageId>,
        reason: Option<&str>,
    ) -> Result<()> {
        let body = BulkDeleteMessages {
            messages: message_ids.into_iter().collect(),
        };
        self.request_empty(
            channel_route(
                Method::POST,
                format!("/channels/{channel_id}/messages/bulk-delete"),
                "/channels/{channel_id}/messages/bulk-delete",
                channel_id,
                RetrySafety::Unsafe,
            ),
            Some(&body),
            audit_headers(reason),
        )
        .await
    }

    /// Returns pins using Discord's current timestamp-paginated pin endpoint.
    pub async fn get_channel_pins(
        &self,
        channel_id: ChannelId,
        query: &ChannelPinsQuery,
    ) -> Result<ChannelPins> {
        self.request_json::<ChannelPins, ()>(
            channel_route(
                Method::GET,
                format!("/channels/{channel_id}/messages/pins{}", query.suffix()),
                "/channels/{channel_id}/messages/pins",
                channel_id,
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Pins a message using Discord's current pin endpoint.
    pub async fn pin_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            pin_route(Method::PUT, channel_id, message_id),
            None,
            audit_headers(reason),
        )
        .await
    }

    /// Unpins a message using Discord's current pin endpoint.
    pub async fn unpin_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            pin_route(Method::DELETE, channel_id, message_id),
            None,
            audit_headers(reason),
        )
        .await
    }
}

fn channel_route(
    method: Method,
    path: String,
    template: &'static str,
    channel_id: ChannelId,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        path,
        template,
        Some(channel_id.to_string()),
        safety,
    )
}

fn message_route(
    method: Method,
    channel_id: ChannelId,
    message_id: MessageId,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    channel_route(
        method,
        format!("/channels/{channel_id}/messages/{message_id}{suffix}"),
        "/channels/{channel_id}/messages/{message_id}",
        channel_id,
        safety,
    )
}

fn reaction_route(
    method: Method,
    channel_id: ChannelId,
    message_id: MessageId,
    emoji: &str,
    suffix: &str,
) -> Route {
    let encoded = percent_encode(emoji);
    channel_route(
        method,
        format!(
            "/channels/{channel_id}/messages/{message_id}/reactions/{encoded}{suffix}"
        ),
        "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}",
        channel_id,
        RetrySafety::Safe,
    )
}

fn pin_route(method: Method, channel_id: ChannelId, message_id: MessageId) -> Route {
    channel_route(
        method,
        format!("/channels/{channel_id}/messages/pins/{message_id}"),
        "/channels/{channel_id}/messages/pins/{message_id}",
        channel_id,
        RetrySafety::Safe,
    )
}

fn audit_headers(reason: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(reason) = reason {
        let encoded = percent_encode(reason);
        if let Ok(value) = HeaderValue::from_str(&encoded) {
            headers.insert(AUDIT_LOG_REASON, value);
        }
    }
    headers
}

fn push_option<T: std::fmt::Display>(query: &mut QueryBuilder, name: &str, value: Option<T>) {
    if let Some(value) = value {
        query.push(name, value);
    }
}

fn push_option_str(query: &mut QueryBuilder, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        query.push_str(name, value);
    }
}

fn push_many<T: std::fmt::Display>(query: &mut QueryBuilder, name: &str, values: &[T]) {
    for value in values {
        query.push(name, value);
    }
}

fn push_many_str(query: &mut QueryBuilder, name: &str, values: &[String]) {
    for value in values {
        query.push_str(name, value);
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{MessageId, UserId};

    use super::{MessageListQuery, MessageSearchQuery, ReactionUsersQuery};

    #[test]
    fn channel_message_cursors_are_mutually_exclusive() {
        let query = MessageListQuery::new()
            .around(MessageId::new(1))
            .before(MessageId::new(2))
            .limit(100);
        assert_eq!(query.suffix(), "?before=2&limit=100");
    }

    #[test]
    fn search_repeats_multi_value_filters() {
        let query = MessageSearchQuery {
            author_ids: vec![UserId::new(1), UserId::new(2)],
            has: vec!["image".to_owned(), "poll".to_owned()],
            ..MessageSearchQuery::default()
        };
        assert_eq!(
            query.suffix(),
            "?author_id=1&author_id=2&has=image&has=poll"
        );
    }

    #[test]
    fn reaction_query_serializes_current_reaction_type() {
        let query = ReactionUsersQuery {
            kind: Some(crate::model::ReactionType::BURST),
            after: Some(UserId::new(9)),
            limit: Some(100),
        };
        assert_eq!(query.suffix(), "?type=1&after=9&limit=100");
    }
}
