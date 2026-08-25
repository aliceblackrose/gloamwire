use reqwest::{Method, header::HeaderMap};
use serde::Serialize;

use crate::{
    Result,
    model::{
        ApplicationId, ChannelId, GuildId, Invite, InviteTargetType, InviteTargetUsersJobStatus,
        RoleId, ScheduledEventId, UserId,
    },
};

use super::{
    RestClient, UploadFile,
    encoding::{QueryBuilder, audit_reason_headers, percent_encode},
    guild::guild_route,
    route::{RetrySafety, Route},
};

/// Parameters accepted by Discord's Create Channel Invite endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CreateChannelInvite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<InviteTargetType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_user_id: Option<UserId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_application_id: Option<ApplicationId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<RoleId>,
}

/// Query options for retrieving an invite by code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GetInviteQuery {
    pub with_counts: Option<bool>,
    pub guild_scheduled_event_id: Option<ScheduledEventId>,
}

impl GetInviteQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(with_counts) = self.with_counts {
            query.push("with_counts", with_counts);
        }
        if let Some(event_id) = self.guild_scheduled_event_id {
            query.push("guild_scheduled_event_id", event_id);
        }
        query.finish()
    }
}

impl RestClient {
    /// Lists invites for a guild channel, including invite metadata.
    pub async fn get_channel_invites(&self, channel_id: ChannelId) -> Result<Vec<Invite>> {
        self.request_json::<Vec<Invite>, ()>(
            channel_invites_route(Method::GET, channel_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Creates an invite for a guild channel.
    pub async fn create_channel_invite(
        &self,
        channel_id: ChannelId,
        create: &CreateChannelInvite,
        reason: Option<&str>,
    ) -> Result<Invite> {
        self.request_json_with_headers(
            channel_invites_route(Method::POST, channel_id, RetrySafety::Unsafe),
            Some(create),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Creates a restricted invite using Discord's target-user CSV upload.
    pub async fn create_channel_invite_with_target_users(
        &self,
        channel_id: ChannelId,
        create: &CreateChannelInvite,
        csv: &UploadFile,
        reason: Option<&str>,
    ) -> Result<Invite> {
        self.request_named_multipart_json(
            channel_invites_route(Method::POST, channel_id, RetrySafety::Unsafe),
            Some(create),
            "target_users_file",
            csv,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Lists all invites for a guild, including invite metadata.
    pub async fn get_guild_invites(&self, guild_id: GuildId) -> Result<Vec<Invite>> {
        self.request_json::<Vec<Invite>, ()>(
            guild_route(
                Method::GET,
                guild_id,
                "/invites",
                "/guilds/{guild_id}/invites",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Returns an invite by code.
    pub async fn get_invite(&self, code: &str, query: &GetInviteQuery) -> Result<Invite> {
        self.request_json::<Invite, ()>(
            invite_route(
                Method::GET,
                code,
                &query.suffix(),
                "/invites/{invite_code}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Deletes an invite by code, optionally recording an audit-log reason.
    pub async fn delete_invite(&self, code: &str, reason: Option<&str>) -> Result<Invite> {
        self.request_json_with_headers::<Invite, ()>(
            invite_route(
                Method::DELETE,
                code,
                "",
                "/invites/{invite_code}",
                RetrySafety::Safe,
            ),
            None,
            audit_reason_headers(reason),
        )
        .await
    }

    /// Downloads the invite's allowed target users as CSV bytes.
    pub async fn get_invite_target_users(&self, code: &str) -> Result<Vec<u8>> {
        self.request_bytes(
            invite_route(
                Method::GET,
                code,
                "/target-users",
                "/invites/{invite_code}/target-users",
                RetrySafety::Safe,
            ),
            HeaderMap::new(),
        )
        .await
    }

    /// Replaces the invite's allowed target users from a CSV upload.
    pub async fn update_invite_target_users(&self, code: &str, csv: &UploadFile) -> Result<()> {
        self.request_named_multipart_empty::<()>(
            invite_route(
                Method::PUT,
                code,
                "/target-users",
                "/invites/{invite_code}/target-users",
                RetrySafety::Safe,
            ),
            None,
            "target_users_file",
            csv,
            HeaderMap::new(),
        )
        .await
    }

    /// Returns the asynchronous target-user CSV processing status.
    pub async fn get_invite_target_users_job_status(
        &self,
        code: &str,
    ) -> Result<InviteTargetUsersJobStatus> {
        self.request_json::<InviteTargetUsersJobStatus, ()>(
            invite_route(
                Method::GET,
                code,
                "/target-users/job-status",
                "/invites/{invite_code}/target-users/job-status",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }
}

fn channel_invites_route(method: Method, channel_id: ChannelId, safety: RetrySafety) -> Route {
    Route::new(
        method,
        format!("/channels/{channel_id}/invites"),
        "/channels/{channel_id}/invites",
        Some(channel_id.to_string()),
        safety,
    )
}

fn invite_route(
    method: Method,
    code: &str,
    suffix: &str,
    template: &'static str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/invites/{}{suffix}", percent_encode(code)),
        template,
        None,
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::ScheduledEventId;

    use super::{CreateChannelInvite, GetInviteQuery, invite_route};
    use crate::http::route::RetrySafety;
    use reqwest::Method;

    #[test]
    fn empty_create_invite_serializes_as_required_object() {
        let value = serde_json::to_value(CreateChannelInvite::default()).expect("invite request");
        assert_eq!(value, serde_json::json!({}));
    }

    #[test]
    fn get_invite_query_includes_counts_and_event() {
        let query = GetInviteQuery {
            with_counts: Some(true),
            guild_scheduled_event_id: Some(ScheduledEventId::new(42)),
        };

        assert_eq!(
            query.suffix(),
            "?with_counts=true&guild_scheduled_event_id=42"
        );
    }

    #[test]
    fn invite_codes_are_encoded_as_path_segments() {
        let route = invite_route(
            Method::GET,
            "code/with space",
            "",
            "/invites/{invite_code}",
            RetrySafety::Safe,
        );

        assert_eq!(route.path, "/invites/code%2Fwith%20space");
    }
}
