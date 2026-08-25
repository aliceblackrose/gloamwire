use reqwest::{Method, header::HeaderMap};
use serde::Serialize;
use serde_json::Value;

use crate::{
    Result,
    model::{
        AllowedMentions, AttachmentRequest, ChannelId, Component, Embed, GuildId, Message,
        MessageFlags, MessageId, PollCreateRequest, Snowflake, Webhook, WebhookId,
    },
};

use super::{
    RestClient, UploadFile,
    encoding::{QueryBuilder, audit_reason_headers, percent_encode},
    guild::guild_route,
    route::{RetrySafety, Route},
};

/// Parameters accepted by Discord's Create Webhook endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateWebhook {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

impl CreateWebhook {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            avatar: None,
        }
    }
}

/// Parameters accepted by Discord's authenticated Modify Webhook endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
}

/// Parameters accepted by Discord's token-authenticated Modify Webhook endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyWebhookWithToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<String>>,
}

/// Message body accepted by Discord's Execute Webhook endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ExecuteWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_tags: Vec<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<PollCreateRequest>,
}

impl ExecuteWebhook {
    /// Creates webhook parameters containing plain text content.
    #[must_use]
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

/// Query options for executing a webhook.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExecuteWebhookQuery {
    pub wait: Option<bool>,
    pub thread_id: Option<ChannelId>,
    pub with_components: Option<bool>,
}

impl ExecuteWebhookQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(wait) = self.wait {
            query.push("wait", wait);
        }
        if let Some(thread_id) = self.thread_id {
            query.push("thread_id", thread_id);
        }
        if let Some(with_components) = self.with_components {
            query.push("with_components", with_components);
        }
        query.finish()
    }
}

/// Query options for Discord's Slack- and GitHub-compatible webhook endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExecuteCompatibleWebhookQuery {
    pub wait: Option<bool>,
    pub thread_id: Option<ChannelId>,
}

impl ExecuteCompatibleWebhookQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(wait) = self.wait {
            query.push("wait", wait);
        }
        if let Some(thread_id) = self.thread_id {
            query.push("thread_id", thread_id);
        }
        query.finish()
    }
}

/// Query options for fetching or deleting a webhook message in a thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WebhookMessageQuery {
    pub thread_id: Option<ChannelId>,
}

impl WebhookMessageQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(thread_id) = self.thread_id {
            query.push("thread_id", thread_id);
        }
        query.finish()
    }
}

/// Query options for editing a webhook message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditWebhookMessageQuery {
    pub thread_id: Option<ChannelId>,
    pub with_components: Option<bool>,
}

impl EditWebhookMessageQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(thread_id) = self.thread_id {
            query.push("thread_id", thread_id);
        }
        if let Some(with_components) = self.with_components {
            query.push("with_components", with_components);
        }
        query.finish()
    }
}

/// Nullable fields accepted by Discord's Edit Webhook Message endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct EditWebhookMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Option<Vec<Embed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Option<MessageFlags>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<Option<AllowedMentions>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Option<Vec<Component>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Option<Vec<AttachmentRequest>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<Option<PollCreateRequest>>,
}

impl RestClient {
    /// Creates a webhook in a channel.
    pub async fn create_webhook(
        &self,
        channel_id: ChannelId,
        create: &CreateWebhook,
        reason: Option<&str>,
    ) -> Result<Webhook> {
        self.request_json_with_headers(
            channel_webhooks_route(Method::POST, channel_id, RetrySafety::Unsafe),
            Some(create),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Lists webhooks configured for a channel.
    pub async fn get_channel_webhooks(&self, channel_id: ChannelId) -> Result<Vec<Webhook>> {
        self.request_json::<Vec<Webhook>, ()>(
            channel_webhooks_route(Method::GET, channel_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Lists webhooks configured for a guild.
    pub async fn get_guild_webhooks(&self, guild_id: GuildId) -> Result<Vec<Webhook>> {
        self.request_json::<Vec<Webhook>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                "/webhooks",
                "/guilds/{guild_id}/webhooks",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Returns a webhook using bot authentication.
    pub async fn get_webhook(&self, webhook_id: WebhookId) -> Result<Webhook> {
        self.request_json::<Webhook, ()>(
            webhook_route(Method::GET, webhook_id, "", RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Returns a webhook using its token.
    pub async fn get_webhook_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
    ) -> Result<Webhook> {
        self.request_json::<Webhook, ()>(
            webhook_token_route(
                Method::GET,
                webhook_id,
                token,
                "",
                "/webhooks/{webhook_id}/{webhook_token}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Modifies a webhook using bot authentication.
    pub async fn modify_webhook(
        &self,
        webhook_id: WebhookId,
        modify: &ModifyWebhook,
        reason: Option<&str>,
    ) -> Result<Webhook> {
        self.request_json_with_headers(
            webhook_route(Method::PATCH, webhook_id, "", RetrySafety::Unsafe),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Modifies a webhook using its token.
    pub async fn modify_webhook_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
        modify: &ModifyWebhookWithToken,
    ) -> Result<Webhook> {
        self.request_json(
            webhook_token_route(
                Method::PATCH,
                webhook_id,
                token,
                "",
                "/webhooks/{webhook_id}/{webhook_token}",
                RetrySafety::Unsafe,
            ),
            Some(modify),
        )
        .await
    }

    /// Permanently deletes a webhook using bot authentication.
    pub async fn delete_webhook(&self, webhook_id: WebhookId, reason: Option<&str>) -> Result<()> {
        self.request_empty::<()>(
            webhook_route(Method::DELETE, webhook_id, "", RetrySafety::Safe),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Permanently deletes a webhook using its token.
    pub async fn delete_webhook_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
    ) -> Result<()> {
        self.request_empty::<()>(
            webhook_token_route(
                Method::DELETE,
                webhook_id,
                token,
                "",
                "/webhooks/{webhook_id}/{webhook_token}",
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Executes a webhook and returns its message when `wait` is enabled.
    pub async fn execute_webhook(
        &self,
        webhook_id: WebhookId,
        token: &str,
        execute: &ExecuteWebhook,
        query: &ExecuteWebhookQuery,
    ) -> Result<Option<Message>> {
        self.request_optional_json(
            webhook_token_route(
                Method::POST,
                webhook_id,
                token,
                &query.suffix(),
                "/webhooks/{webhook_id}/{webhook_token}",
                RetrySafety::Unsafe,
            ),
            Some(execute),
            HeaderMap::new(),
        )
        .await
    }

    /// Executes a webhook with streamed multipart file uploads.
    pub async fn execute_webhook_with_files(
        &self,
        webhook_id: WebhookId,
        token: &str,
        execute: &ExecuteWebhook,
        files: &[UploadFile],
        query: &ExecuteWebhookQuery,
    ) -> Result<Option<Message>> {
        let mut request = execute.clone();
        request
            .attachments
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_optional_multipart_json(
            webhook_token_route(
                Method::POST,
                webhook_id,
                token,
                &query.suffix(),
                "/webhooks/{webhook_id}/{webhook_token}",
                RetrySafety::Unsafe,
            ),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }

    /// Executes Discord's Slack-compatible webhook endpoint.
    pub async fn execute_slack_compatible_webhook(
        &self,
        webhook_id: WebhookId,
        token: &str,
        payload: &Value,
        query: &ExecuteCompatibleWebhookQuery,
    ) -> Result<Option<Message>> {
        self.execute_compatible_webhook(webhook_id, token, "slack", payload, query)
            .await
    }

    /// Executes Discord's GitHub-compatible webhook endpoint.
    pub async fn execute_github_compatible_webhook(
        &self,
        webhook_id: WebhookId,
        token: &str,
        payload: &Value,
        query: &ExecuteCompatibleWebhookQuery,
    ) -> Result<Option<Message>> {
        self.execute_compatible_webhook(webhook_id, token, "github", payload, query)
            .await
    }

    /// Returns a message previously sent by a webhook.
    pub async fn get_webhook_message(
        &self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        query: &WebhookMessageQuery,
    ) -> Result<Message> {
        self.request_json::<Message, ()>(
            webhook_message_route(
                Method::GET,
                webhook_id,
                token,
                message_id,
                &query.suffix(),
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Edits a message previously sent by a webhook.
    pub async fn edit_webhook_message(
        &self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        edit: &EditWebhookMessage,
        query: &EditWebhookMessageQuery,
    ) -> Result<Message> {
        self.request_json(
            webhook_message_route(
                Method::PATCH,
                webhook_id,
                token,
                message_id,
                &query.suffix(),
                RetrySafety::Unsafe,
            ),
            Some(edit),
        )
        .await
    }

    /// Edits a webhook message and appends streamed multipart file uploads.
    pub async fn edit_webhook_message_with_files(
        &self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        edit: &EditWebhookMessage,
        files: &[UploadFile],
        query: &EditWebhookMessageQuery,
    ) -> Result<Message> {
        let mut request = edit.clone();
        request
            .attachments
            .get_or_insert_with(|| Some(Vec::new()))
            .get_or_insert_default()
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_multipart_json(
            webhook_message_route(
                Method::PATCH,
                webhook_id,
                token,
                message_id,
                &query.suffix(),
                RetrySafety::Unsafe,
            ),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }

    /// Deletes a message previously sent by a webhook.
    pub async fn delete_webhook_message(
        &self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        query: &WebhookMessageQuery,
    ) -> Result<()> {
        self.request_empty::<()>(
            webhook_message_route(
                Method::DELETE,
                webhook_id,
                token,
                message_id,
                &query.suffix(),
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    async fn execute_compatible_webhook(
        &self,
        webhook_id: WebhookId,
        token: &str,
        kind: &'static str,
        payload: &Value,
        query: &ExecuteCompatibleWebhookQuery,
    ) -> Result<Option<Message>> {
        self.request_optional_json(
            webhook_token_route(
                Method::POST,
                webhook_id,
                token,
                &format!("/{kind}{}", query.suffix()),
                match kind {
                    "slack" => "/webhooks/{webhook_id}/{webhook_token}/slack",
                    "github" => "/webhooks/{webhook_id}/{webhook_token}/github",
                    _ => "/webhooks/{webhook_id}/{webhook_token}/{compatible_kind}",
                },
                RetrySafety::Unsafe,
            ),
            Some(payload),
            HeaderMap::new(),
        )
        .await
    }
}

fn channel_webhooks_route(method: Method, channel_id: ChannelId, safety: RetrySafety) -> Route {
    Route::new(
        method,
        format!("/channels/{channel_id}/webhooks"),
        "/channels/{channel_id}/webhooks",
        Some(channel_id.to_string()),
        safety,
    )
}

fn webhook_route(
    method: Method,
    webhook_id: WebhookId,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/webhooks/{webhook_id}{suffix}"),
        "/webhooks/{webhook_id}",
        Some(webhook_id.to_string()),
        safety,
    )
}

fn webhook_token_route(
    method: Method,
    webhook_id: WebhookId,
    token: &str,
    suffix: &str,
    template: &'static str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/webhooks/{webhook_id}/{}{suffix}", percent_encode(token)),
        template,
        Some(webhook_id.to_string()),
        safety,
    )
}

fn webhook_message_route(
    method: Method,
    webhook_id: WebhookId,
    token: &str,
    message_id: MessageId,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    webhook_token_route(
        method,
        webhook_id,
        token,
        &format!("/messages/{message_id}{suffix}"),
        "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}",
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::{ChannelId, WebhookId};

    use super::{
        EditWebhookMessage, ExecuteWebhookQuery, ModifyWebhook, WebhookMessageQuery,
        webhook_message_route, webhook_token_route,
    };
    use crate::http::route::RetrySafety;
    use reqwest::Method;

    #[test]
    fn execute_query_serializes_all_current_switches() {
        let query = ExecuteWebhookQuery {
            wait: Some(true),
            thread_id: Some(ChannelId::new(42)),
            with_components: Some(true),
        };

        assert_eq!(
            query.suffix(),
            "?wait=true&thread_id=42&with_components=true"
        );
    }

    #[test]
    fn nullable_webhook_fields_distinguish_clear_from_omission() {
        let modify = ModifyWebhook {
            avatar: Some(None),
            ..ModifyWebhook::default()
        };
        let edit = EditWebhookMessage {
            content: Some(None),
            ..EditWebhookMessage::default()
        };

        assert!(serde_json::to_value(modify).expect("modify")["avatar"].is_null());
        assert!(serde_json::to_value(edit).expect("edit")["content"].is_null());
    }

    #[test]
    fn webhook_tokens_are_encoded_as_path_segments() {
        let route = webhook_token_route(
            Method::GET,
            WebhookId::new(1),
            "token/with space",
            &WebhookMessageQuery::default().suffix(),
            "/webhooks/{webhook_id}/{webhook_token}",
            RetrySafety::Safe,
        );

        assert_eq!(route.path, "/webhooks/1/token%2Fwith%20space");
    }

    #[test]
    fn webhook_subroutes_keep_distinct_rate_limit_identities() {
        let webhook_id = WebhookId::new(1);
        let base = webhook_token_route(
            Method::GET,
            webhook_id,
            "token",
            "",
            "/webhooks/{webhook_id}/{webhook_token}",
            RetrySafety::Safe,
        );
        let message = webhook_message_route(
            Method::GET,
            webhook_id,
            "token",
            crate::model::MessageId::new(2),
            "",
            RetrySafety::Safe,
        );

        assert_ne!(base.identity(), message.identity());
    }
}
