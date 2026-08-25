use reqwest::Method;

use crate::{
    Result,
    model::{AuditLog, AuditLogEntryId, AuditLogEvent, GuildId, UserId},
};

use super::{RestClient, encoding::QueryBuilder, guild::guild_route, route::RetrySafety};

/// Filters and pagination for Discord's Get Guild Audit Log endpoint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuditLogQuery {
    pub user_id: Option<UserId>,
    pub action_type: Option<AuditLogEvent>,
    pub before: Option<AuditLogEntryId>,
    pub after: Option<AuditLogEntryId>,
    pub limit: Option<u8>,
}

impl AuditLogQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(user_id) = self.user_id {
            query.push("user_id", user_id);
        }
        if let Some(action_type) = self.action_type {
            query.push("action_type", action_type.0);
        }
        if let Some(before) = self.before {
            query.push("before", before);
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

impl RestClient {
    /// Returns administrative actions recorded for a guild.
    pub async fn get_guild_audit_log(
        &self,
        guild_id: GuildId,
        query: &AuditLogQuery,
    ) -> Result<AuditLog> {
        self.request_json::<AuditLog, ()>(
            guild_route(
                Method::GET,
                guild_id,
                &format!("/audit-logs{}", query.suffix()),
                "/guilds/{guild_id}/audit-logs",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{AuditLogEntryId, AuditLogEvent, UserId};

    use super::AuditLogQuery;

    #[test]
    fn audit_log_query_serializes_filters_and_cursor() {
        let query = AuditLogQuery {
            user_id: Some(UserId::new(10)),
            action_type: Some(AuditLogEvent::MEMBER_BAN_ADD),
            before: Some(AuditLogEntryId::new(20)),
            after: None,
            limit: Some(100),
        };

        assert_eq!(
            query.suffix(),
            "?user_id=10&action_type=22&before=20&limit=100"
        );
    }
}
