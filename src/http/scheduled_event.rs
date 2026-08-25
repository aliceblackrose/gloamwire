use reqwest::{Method, header::HeaderMap};
use serde::Serialize;

use crate::{
    Result,
    model::{
        ChannelId, GuildId, GuildScheduledEvent, GuildScheduledEventEntityMetadata,
        GuildScheduledEventEntityType, GuildScheduledEventPrivacyLevel,
        GuildScheduledEventRecurrenceFrequency, GuildScheduledEventRecurrenceMonth,
        GuildScheduledEventRecurrenceNWeekday, GuildScheduledEventRecurrenceWeekday,
        GuildScheduledEventStatus, GuildScheduledEventUser, ScheduledEventId, UserId,
    },
};

use super::{
    RestClient,
    encoding::{QueryBuilder, audit_reason_headers},
    guild::guild_route,
    route::RetrySafety,
};

/// Recurrence fields accepted when creating or modifying a scheduled event.
///
/// Response-only recurrence fields (`end`, `count`, and `by_year_day`) are
/// intentionally omitted because Discord currently rejects clients setting them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GuildScheduledEventRecurrenceRuleRequest {
    pub start: String,
    pub frequency: GuildScheduledEventRecurrenceFrequency,
    pub interval: u32,
    pub by_weekday: Option<Vec<GuildScheduledEventRecurrenceWeekday>>,
    pub by_n_weekday: Option<Vec<GuildScheduledEventRecurrenceNWeekday>>,
    pub by_month: Option<Vec<GuildScheduledEventRecurrenceMonth>>,
    pub by_month_day: Option<Vec<i16>>,
}

/// Parameters accepted by Discord's Create Guild Scheduled Event endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateGuildScheduledEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<GuildScheduledEventEntityMetadata>,
    pub name: String,
    pub privacy_level: GuildScheduledEventPrivacyLevel,
    pub scheduled_start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entity_type: GuildScheduledEventEntityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<GuildScheduledEventRecurrenceRuleRequest>,
}

/// Parameters accepted by Discord's Modify Guild Scheduled Event endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyGuildScheduledEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Option<ChannelId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<Option<GuildScheduledEventEntityMetadata>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<GuildScheduledEventPrivacyLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<GuildScheduledEventEntityType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GuildScheduledEventStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<Option<GuildScheduledEventRecurrenceRuleRequest>>,
}

/// Pagination for subscribers to a guild scheduled event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuildScheduledEventUsersQuery {
    pub limit: Option<u8>,
    pub with_member: Option<bool>,
    pub before: Option<UserId>,
    pub after: Option<UserId>,
}

impl GuildScheduledEventUsersQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(limit) = self.limit {
            query.push("limit", limit);
        }
        if let Some(with_member) = self.with_member {
            query.push("with_member", with_member);
        }
        if let Some(before) = self.before {
            query.push("before", before);
        }
        if let Some(after) = self.after {
            query.push("after", after);
        }
        query.finish()
    }
}

impl RestClient {
    /// Lists scheduled events in a guild.
    pub async fn list_guild_scheduled_events(
        &self,
        guild_id: GuildId,
        with_user_count: bool,
    ) -> Result<Vec<GuildScheduledEvent>> {
        let suffix = if with_user_count {
            "/scheduled-events?with_user_count=true"
        } else {
            "/scheduled-events"
        };
        self.request_json::<Vec<GuildScheduledEvent>, ()>(
            scheduled_event_collection_route(Method::GET, guild_id, suffix, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Creates a scheduled event, optionally recording an audit-log reason.
    pub async fn create_guild_scheduled_event(
        &self,
        guild_id: GuildId,
        create: &CreateGuildScheduledEvent,
        reason: Option<&str>,
    ) -> Result<GuildScheduledEvent> {
        self.request_json_with_headers(
            scheduled_event_collection_route(
                Method::POST,
                guild_id,
                "/scheduled-events",
                RetrySafety::Unsafe,
            ),
            Some(create),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Returns one scheduled event.
    pub async fn get_guild_scheduled_event(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
        with_user_count: bool,
    ) -> Result<GuildScheduledEvent> {
        let query = if with_user_count {
            "?with_user_count=true"
        } else {
            ""
        };
        self.request_json::<GuildScheduledEvent, ()>(
            scheduled_event_route(
                Method::GET,
                guild_id,
                event_id,
                query,
                "/guilds/{guild_id}/scheduled-events/{event_id}",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Updates one scheduled event, optionally recording an audit-log reason.
    pub async fn modify_guild_scheduled_event(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
        modify: &ModifyGuildScheduledEvent,
        reason: Option<&str>,
    ) -> Result<GuildScheduledEvent> {
        self.request_json_with_headers(
            scheduled_event_route(
                Method::PATCH,
                guild_id,
                event_id,
                "",
                "/guilds/{guild_id}/scheduled-events/{event_id}",
                RetrySafety::Unsafe,
            ),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Deletes one scheduled event.
    pub async fn delete_guild_scheduled_event(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
    ) -> Result<()> {
        self.request_empty::<()>(
            scheduled_event_route(
                Method::DELETE,
                guild_id,
                event_id,
                "",
                "/guilds/{guild_id}/scheduled-events/{event_id}",
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Lists users subscribed to one scheduled event.
    pub async fn get_guild_scheduled_event_users(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
        query: &GuildScheduledEventUsersQuery,
    ) -> Result<Vec<GuildScheduledEventUser>> {
        self.request_json::<Vec<GuildScheduledEventUser>, ()>(
            scheduled_event_route(
                Method::GET,
                guild_id,
                event_id,
                &format!("/users{}", query.suffix()),
                "/guilds/{guild_id}/scheduled-events/{event_id}/users",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }
}

fn scheduled_event_collection_route(
    method: Method,
    guild_id: GuildId,
    suffix: &str,
    safety: RetrySafety,
) -> super::route::Route {
    guild_route(
        method,
        guild_id,
        suffix,
        "/guilds/{guild_id}/scheduled-events",
        safety,
    )
}

fn scheduled_event_route(
    method: Method,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    suffix: &str,
    template: &'static str,
    safety: RetrySafety,
) -> super::route::Route {
    guild_route(
        method,
        guild_id,
        &format!("/scheduled-events/{event_id}{suffix}"),
        template,
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::{
        GuildScheduledEventRecurrenceFrequency, GuildScheduledEventRecurrenceMonth, UserId,
    };

    use super::{
        GuildScheduledEventRecurrenceRuleRequest, GuildScheduledEventUsersQuery,
        ModifyGuildScheduledEvent,
    };

    #[test]
    fn recurrence_request_uses_documented_frequency_values() {
        let recurrence = GuildScheduledEventRecurrenceRuleRequest {
            start: "2026-09-01T18:00:00+00:00".to_owned(),
            frequency: GuildScheduledEventRecurrenceFrequency::YEARLY,
            interval: 1,
            by_weekday: None,
            by_n_weekday: None,
            by_month: Some(vec![GuildScheduledEventRecurrenceMonth::SEPTEMBER]),
            by_month_day: Some(vec![1]),
        };
        let value = serde_json::to_value(recurrence).expect("recurrence request");

        assert_eq!(value["frequency"], 0);
        assert!(value["by_weekday"].is_null());
        assert!(value.get("end").is_none());
        assert!(value.get("count").is_none());
    }

    #[test]
    fn nullable_event_fields_distinguish_clear_from_omission() {
        let modify = ModifyGuildScheduledEvent {
            channel_id: Some(None),
            recurrence_rule: Some(None),
            ..ModifyGuildScheduledEvent::default()
        };
        let value = serde_json::to_value(modify).expect("modify event");

        assert!(value["channel_id"].is_null());
        assert!(value["recurrence_rule"].is_null());
        assert!(value.get("name").is_none());
    }

    #[test]
    fn event_users_query_serializes_cursor() {
        let query = GuildScheduledEventUsersQuery {
            limit: Some(100),
            with_member: Some(true),
            before: None,
            after: Some(UserId::new(42)),
        };

        assert_eq!(query.suffix(), "?limit=100&with_member=true&after=42");
    }
}
