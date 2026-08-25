use serde::{Deserialize, Serialize};

use super::{ChannelId, GuildId, GuildMember, ScheduledEventId, Snowflake, User, UserId};

/// Discord guild scheduled-event privacy level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuildScheduledEventPrivacyLevel(pub u8);

impl GuildScheduledEventPrivacyLevel {
    pub const GUILD_ONLY: Self = Self(2);
}

/// Discord guild scheduled-event entity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuildScheduledEventEntityType(pub u8);

impl GuildScheduledEventEntityType {
    pub const STAGE_INSTANCE: Self = Self(1);
    pub const VOICE: Self = Self(2);
    pub const EXTERNAL: Self = Self(3);
}

/// Discord guild scheduled-event status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuildScheduledEventStatus(pub u8);

impl GuildScheduledEventStatus {
    pub const SCHEDULED: Self = Self(1);
    pub const ACTIVE: Self = Self(2);
    pub const COMPLETED: Self = Self(3);
    pub const CANCELED: Self = Self(4);
}

/// Recurrence frequency for a scheduled event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuildScheduledEventRecurrenceFrequency(pub u8);

impl GuildScheduledEventRecurrenceFrequency {
    pub const YEARLY: Self = Self(0);
    pub const MONTHLY: Self = Self(1);
    pub const WEEKLY: Self = Self(2);
    pub const DAILY: Self = Self(3);
}

/// Weekday used by scheduled-event recurrence rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuildScheduledEventRecurrenceWeekday(pub u8);

impl GuildScheduledEventRecurrenceWeekday {
    pub const MONDAY: Self = Self(0);
    pub const TUESDAY: Self = Self(1);
    pub const WEDNESDAY: Self = Self(2);
    pub const THURSDAY: Self = Self(3);
    pub const FRIDAY: Self = Self(4);
    pub const SATURDAY: Self = Self(5);
    pub const SUNDAY: Self = Self(6);
}

/// Month used by scheduled-event recurrence rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuildScheduledEventRecurrenceMonth(pub u8);

impl GuildScheduledEventRecurrenceMonth {
    pub const JANUARY: Self = Self(1);
    pub const FEBRUARY: Self = Self(2);
    pub const MARCH: Self = Self(3);
    pub const APRIL: Self = Self(4);
    pub const MAY: Self = Self(5);
    pub const JUNE: Self = Self(6);
    pub const JULY: Self = Self(7);
    pub const AUGUST: Self = Self(8);
    pub const SEPTEMBER: Self = Self(9);
    pub const OCTOBER: Self = Self(10);
    pub const NOVEMBER: Self = Self(11);
    pub const DECEMBER: Self = Self(12);
}

/// A specific numbered weekday in a month for recurrence rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEventRecurrenceNWeekday {
    pub n: u8,
    pub day: GuildScheduledEventRecurrenceWeekday,
}

/// Discord's recurrence definition for a guild scheduled event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEventRecurrenceRule {
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    pub frequency: GuildScheduledEventRecurrenceFrequency,
    pub interval: u32,
    #[serde(default)]
    pub by_weekday: Option<Vec<GuildScheduledEventRecurrenceWeekday>>,
    #[serde(default)]
    pub by_n_weekday: Option<Vec<GuildScheduledEventRecurrenceNWeekday>>,
    #[serde(default)]
    pub by_month: Option<Vec<GuildScheduledEventRecurrenceMonth>>,
    #[serde(default)]
    pub by_month_day: Option<Vec<i16>>,
    #[serde(default)]
    pub by_year_day: Option<Vec<u16>>,
    #[serde(default)]
    pub count: Option<u32>,
}

/// Additional metadata associated with a guild scheduled event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEventEntityMetadata {
    #[serde(default)]
    pub location: Option<String>,
}

/// A Discord guild scheduled event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEvent {
    pub id: ScheduledEventId,
    pub guild_id: GuildId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub creator_id: Option<UserId>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub scheduled_start_time: String,
    #[serde(default)]
    pub scheduled_end_time: Option<String>,
    pub privacy_level: GuildScheduledEventPrivacyLevel,
    pub status: GuildScheduledEventStatus,
    pub entity_type: GuildScheduledEventEntityType,
    #[serde(default)]
    pub entity_id: Option<Snowflake>,
    #[serde(default)]
    pub entity_metadata: Option<GuildScheduledEventEntityMetadata>,
    #[serde(default)]
    pub creator: Option<User>,
    #[serde(default)]
    pub user_count: Option<u32>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub recurrence_rule: Option<GuildScheduledEventRecurrenceRule>,
}

/// A user subscribed to a guild scheduled event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEventUser {
    pub guild_scheduled_event_id: ScheduledEventId,
    pub user: User,
    #[serde(default)]
    pub member: Option<GuildMember>,
}

#[cfg(test)]
mod tests {
    use super::{
        GuildScheduledEvent, GuildScheduledEventEntityType, GuildScheduledEventRecurrenceFrequency,
        GuildScheduledEventRecurrenceMonth, GuildScheduledEventStatus,
    };

    #[test]
    fn parses_external_recurring_event() {
        let event: GuildScheduledEvent = serde_json::from_str(
            r#"{
                "id":"100",
                "guild_id":"200",
                "channel_id":null,
                "creator_id":"300",
                "name":"Annual meetup",
                "description":"Outside",
                "scheduled_start_time":"2026-09-01T18:00:00+00:00",
                "scheduled_end_time":"2026-09-01T20:00:00+00:00",
                "privacy_level":2,
                "status":1,
                "entity_type":3,
                "entity_id":null,
                "entity_metadata":{"location":"Chicago"},
                "user_count":12,
                "image":null,
                "recurrence_rule":{
                    "start":"2026-09-01T18:00:00+00:00",
                    "end":null,
                    "frequency":0,
                    "interval":1,
                    "by_weekday":null,
                    "by_n_weekday":null,
                    "by_month":[9],
                    "by_month_day":[1],
                    "by_year_day":null,
                    "count":null
                }
            }"#,
        )
        .expect("scheduled event");

        assert_eq!(event.entity_type, GuildScheduledEventEntityType::EXTERNAL);
        assert_eq!(event.status, GuildScheduledEventStatus::SCHEDULED);
        let recurrence = event.recurrence_rule.expect("recurrence rule");
        assert_eq!(recurrence.frequency, GuildScheduledEventRecurrenceFrequency::YEARLY);
        assert_eq!(recurrence.by_month, Some(vec![GuildScheduledEventRecurrenceMonth::SEPTEMBER]));
    }

    #[test]
    fn scheduled_event_numeric_types_preserve_unknown_values() {
        let kind: GuildScheduledEventEntityType = serde_json::from_str("99").expect("entity type");
        assert_eq!(kind, GuildScheduledEventEntityType(99));
    }
}
