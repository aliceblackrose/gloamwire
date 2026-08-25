use serde::Deserialize;
use serde_json::Value;

use crate::model::{
    ApplicationId, AuditLogEntry, AutoModerationAction, AutoModerationRule, AutoModerationRuleId,
    AutoModerationTriggerType, Channel, ChannelId, Guild, GuildId, GuildMember, GuildMemberFlags,
    GuildScheduledEvent, Interaction, InviteTargetType, Message, MessageId, PartialEmoji,
    PresenceUpdate, ReactionType, Role, RoleId, ScheduledEventId, Snowflake, UnavailableGuild,
    User, UserId, VoiceState,
};

use super::DispatchEvent;

/// Typed data for commonly used Discord Gateway dispatches.
///
/// Unmodeled event names are preserved in [`Self::Unknown`] so a Discord API
/// addition does not make a newer Gateway event impossible to consume.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TypedDispatchEvent {
    Ready(ReadyEvent),
    Resumed,
    AutoModerationRuleCreate(AutoModerationRule),
    AutoModerationRuleUpdate(AutoModerationRule),
    AutoModerationRuleDelete(AutoModerationRule),
    AutoModerationActionExecution(AutoModerationActionExecutionEvent),
    GuildCreate(Guild),
    GuildUpdate(Guild),
    GuildDelete(UnavailableGuild),
    GuildAuditLogEntryCreate(GuildAuditLogEntryCreateEvent),
    GuildScheduledEventCreate(GuildScheduledEvent),
    GuildScheduledEventUpdate(GuildScheduledEvent),
    GuildScheduledEventDelete(GuildScheduledEvent),
    GuildScheduledEventUserAdd(GuildScheduledEventUserEvent),
    GuildScheduledEventUserRemove(GuildScheduledEventUserEvent),
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete(Channel),
    ThreadCreate(Channel),
    ThreadUpdate(Channel),
    ThreadDelete(Channel),
    GuildMemberAdd(GuildMemberAddEvent),
    GuildMemberUpdate(GuildMemberUpdateEvent),
    GuildMemberRemove(GuildMemberRemoveEvent),
    GuildMembersChunk(GuildMembersChunkEvent),
    GuildRoleCreate(GuildRoleEvent),
    GuildRoleUpdate(GuildRoleEvent),
    GuildRoleDelete(GuildRoleDeleteEvent),
    InviteCreate(InviteCreateEvent),
    InviteDelete(InviteDeleteEvent),
    MessageCreate(Message),
    MessageDelete(MessageDeleteEvent),
    MessageDeleteBulk(MessageDeleteBulkEvent),
    MessageReactionAdd(MessageReactionAddEvent),
    MessageReactionRemove(MessageReactionRemoveEvent),
    MessageReactionRemoveAll(MessageReactionRemoveAllEvent),
    MessageReactionRemoveEmoji(MessageReactionRemoveEmojiEvent),
    MessagePollVoteAdd(MessagePollVoteEvent),
    MessagePollVoteRemove(MessagePollVoteEvent),
    InteractionCreate(Box<Interaction>),
    PresenceUpdate(PresenceUpdate),
    VoiceStateUpdate(VoiceState),
    WebhooksUpdate(WebhooksUpdateEvent),
    UserUpdate(User),
    Unknown { name: String, data: Value },
}

/// Data delivered by Discord's `READY` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReadyEvent {
    pub v: u8,
    pub user: User,
    #[serde(default)]
    pub guilds: Vec<UnavailableGuild>,
    pub session_id: String,
    pub resume_gateway_url: String,
    #[serde(default)]
    pub shard: Option<[u32; 2]>,
    pub application: ReadyApplication,
}

/// The partial application object included in `READY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct ReadyApplication {
    pub id: ApplicationId,
    pub flags: u64,
}

/// An `AUTO_MODERATION_ACTION_EXECUTION` dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AutoModerationActionExecutionEvent {
    pub guild_id: GuildId,
    pub action: AutoModerationAction,
    pub rule_id: AutoModerationRuleId,
    pub rule_trigger_type: AutoModerationTriggerType,
    pub user_id: UserId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub message_id: Option<MessageId>,
    #[serde(default)]
    pub alert_system_message_id: Option<MessageId>,
    #[serde(default)]
    pub content: String,
    pub matched_keyword: Option<String>,
    #[serde(default)]
    pub matched_content: Option<String>,
}

/// A `GUILD_AUDIT_LOG_ENTRY_CREATE` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GuildAuditLogEntryCreateEvent {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub entry: AuditLogEntry,
}

/// A scheduled-event user subscription or unsubscription dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct GuildScheduledEventUserEvent {
    pub guild_scheduled_event_id: ScheduledEventId,
    pub user_id: UserId,
    pub guild_id: GuildId,
}

/// A `GUILD_MEMBER_ADD` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GuildMemberAddEvent {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub member: GuildMember,
}

/// A `GUILD_MEMBER_UPDATE` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GuildMemberUpdateEvent {
    pub guild_id: GuildId,
    #[serde(default)]
    pub roles: Vec<RoleId>,
    pub user: User,
    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub joined_at: Option<String>,
    #[serde(default)]
    pub premium_since: Option<String>,
    #[serde(default)]
    pub deaf: Option<bool>,
    #[serde(default)]
    pub mute: Option<bool>,
    #[serde(default)]
    pub pending: Option<bool>,
    #[serde(default)]
    pub communication_disabled_until: Option<String>,
    #[serde(default)]
    pub flags: Option<GuildMemberFlags>,
}

/// A `GUILD_MEMBER_REMOVE` dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GuildMemberRemoveEvent {
    pub guild_id: GuildId,
    pub user: User,
}

/// A `GUILD_MEMBERS_CHUNK` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GuildMembersChunkEvent {
    pub guild_id: GuildId,
    #[serde(default)]
    pub members: Vec<GuildMember>,
    pub chunk_index: u32,
    pub chunk_count: u32,
    #[serde(default)]
    pub not_found: Vec<Snowflake>,
    #[serde(default)]
    pub presences: Vec<Value>,
    #[serde(default)]
    pub nonce: Option<String>,
}

/// A role create/update dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GuildRoleEvent {
    pub guild_id: GuildId,
    pub role: Role,
}

/// A `GUILD_ROLE_DELETE` dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct GuildRoleDeleteEvent {
    pub guild_id: GuildId,
    pub role_id: RoleId,
}

/// An `INVITE_CREATE` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InviteCreateEvent {
    pub channel_id: ChannelId,
    pub code: String,
    pub created_at: String,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub inviter: Option<User>,
    pub max_age: u32,
    pub max_uses: u32,
    #[serde(default)]
    pub target_type: Option<InviteTargetType>,
    #[serde(default)]
    pub target_user: Option<User>,
    /// Partial application payload for embedded-application invites.
    #[serde(default)]
    pub target_application: Option<Value>,
    pub temporary: bool,
    pub uses: u32,
    pub expires_at: Option<String>,
    #[serde(default)]
    pub role_ids: Vec<RoleId>,
}

/// An `INVITE_DELETE` dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InviteDeleteEvent {
    pub channel_id: ChannelId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub code: String,
}

/// A `MESSAGE_DELETE` dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MessageDeleteEvent {
    pub id: MessageId,
    pub channel_id: ChannelId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
}

/// A `MESSAGE_DELETE_BULK` dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MessageDeleteBulkEvent {
    #[serde(default)]
    pub ids: Vec<MessageId>,
    pub channel_id: ChannelId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
}

/// A `MESSAGE_REACTION_ADD` dispatch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MessageReactionAddEvent {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub member: Option<GuildMember>,
    pub emoji: PartialEmoji,
    #[serde(default)]
    pub message_author_id: Option<UserId>,
    pub burst: bool,
    #[serde(default)]
    pub burst_colors: Vec<String>,
    #[serde(rename = "type")]
    pub kind: ReactionType,
}

/// A `MESSAGE_REACTION_REMOVE` dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MessageReactionRemoveEvent {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub emoji: PartialEmoji,
    pub burst: bool,
    #[serde(rename = "type")]
    pub kind: ReactionType,
}

/// A `MESSAGE_REACTION_REMOVE_ALL` dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MessageReactionRemoveAllEvent {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
}

/// A `MESSAGE_REACTION_REMOVE_EMOJI` dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MessageReactionRemoveEmojiEvent {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub emoji: PartialEmoji,
}

/// A message poll vote add/remove dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MessagePollVoteEvent {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub answer_id: u32,
}

/// A `WEBHOOKS_UPDATE` dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct WebhooksUpdateEvent {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}

impl DispatchEvent {
    /// Parses this raw dispatch into a typed event when Gloamwire models the event name.
    ///
    /// Unknown event names are returned as [`TypedDispatchEvent::Unknown`] without
    /// losing their payload. A known event with an invalid payload returns the
    /// underlying serde error rather than silently degrading to raw JSON.
    pub fn typed(&self) -> serde_json::Result<TypedDispatchEvent> {
        let data = self.data.clone();

        Ok(match self.name.as_str() {
            "READY" => TypedDispatchEvent::Ready(serde_json::from_value(data)?),
            "RESUMED" => TypedDispatchEvent::Resumed,
            "AUTO_MODERATION_RULE_CREATE" => {
                TypedDispatchEvent::AutoModerationRuleCreate(serde_json::from_value(data)?)
            }
            "AUTO_MODERATION_RULE_UPDATE" => {
                TypedDispatchEvent::AutoModerationRuleUpdate(serde_json::from_value(data)?)
            }
            "AUTO_MODERATION_RULE_DELETE" => {
                TypedDispatchEvent::AutoModerationRuleDelete(serde_json::from_value(data)?)
            }
            "AUTO_MODERATION_ACTION_EXECUTION" => {
                TypedDispatchEvent::AutoModerationActionExecution(serde_json::from_value(data)?)
            }
            "GUILD_CREATE" => TypedDispatchEvent::GuildCreate(serde_json::from_value(data)?),
            "GUILD_UPDATE" => TypedDispatchEvent::GuildUpdate(serde_json::from_value(data)?),
            "GUILD_DELETE" => TypedDispatchEvent::GuildDelete(serde_json::from_value(data)?),
            "GUILD_AUDIT_LOG_ENTRY_CREATE" => {
                TypedDispatchEvent::GuildAuditLogEntryCreate(serde_json::from_value(data)?)
            }
            "GUILD_SCHEDULED_EVENT_CREATE" => {
                TypedDispatchEvent::GuildScheduledEventCreate(serde_json::from_value(data)?)
            }
            "GUILD_SCHEDULED_EVENT_UPDATE" => {
                TypedDispatchEvent::GuildScheduledEventUpdate(serde_json::from_value(data)?)
            }
            "GUILD_SCHEDULED_EVENT_DELETE" => {
                TypedDispatchEvent::GuildScheduledEventDelete(serde_json::from_value(data)?)
            }
            "GUILD_SCHEDULED_EVENT_USER_ADD" => {
                TypedDispatchEvent::GuildScheduledEventUserAdd(serde_json::from_value(data)?)
            }
            "GUILD_SCHEDULED_EVENT_USER_REMOVE" => {
                TypedDispatchEvent::GuildScheduledEventUserRemove(serde_json::from_value(data)?)
            }
            "CHANNEL_CREATE" => TypedDispatchEvent::ChannelCreate(serde_json::from_value(data)?),
            "CHANNEL_UPDATE" => TypedDispatchEvent::ChannelUpdate(serde_json::from_value(data)?),
            "CHANNEL_DELETE" => TypedDispatchEvent::ChannelDelete(serde_json::from_value(data)?),
            "THREAD_CREATE" => TypedDispatchEvent::ThreadCreate(serde_json::from_value(data)?),
            "THREAD_UPDATE" => TypedDispatchEvent::ThreadUpdate(serde_json::from_value(data)?),
            "THREAD_DELETE" => TypedDispatchEvent::ThreadDelete(serde_json::from_value(data)?),
            "GUILD_MEMBER_ADD" => TypedDispatchEvent::GuildMemberAdd(serde_json::from_value(data)?),
            "GUILD_MEMBER_UPDATE" => {
                TypedDispatchEvent::GuildMemberUpdate(serde_json::from_value(data)?)
            }
            "GUILD_MEMBER_REMOVE" => {
                TypedDispatchEvent::GuildMemberRemove(serde_json::from_value(data)?)
            }
            "GUILD_MEMBERS_CHUNK" => {
                TypedDispatchEvent::GuildMembersChunk(serde_json::from_value(data)?)
            }
            "GUILD_ROLE_CREATE" => {
                TypedDispatchEvent::GuildRoleCreate(serde_json::from_value(data)?)
            }
            "GUILD_ROLE_UPDATE" => {
                TypedDispatchEvent::GuildRoleUpdate(serde_json::from_value(data)?)
            }
            "GUILD_ROLE_DELETE" => {
                TypedDispatchEvent::GuildRoleDelete(serde_json::from_value(data)?)
            }
            "INVITE_CREATE" => TypedDispatchEvent::InviteCreate(serde_json::from_value(data)?),
            "INVITE_DELETE" => TypedDispatchEvent::InviteDelete(serde_json::from_value(data)?),
            "MESSAGE_CREATE" => TypedDispatchEvent::MessageCreate(serde_json::from_value(data)?),
            "MESSAGE_DELETE" => TypedDispatchEvent::MessageDelete(serde_json::from_value(data)?),
            "MESSAGE_DELETE_BULK" => {
                TypedDispatchEvent::MessageDeleteBulk(serde_json::from_value(data)?)
            }
            "MESSAGE_REACTION_ADD" => {
                TypedDispatchEvent::MessageReactionAdd(serde_json::from_value(data)?)
            }
            "MESSAGE_REACTION_REMOVE" => {
                TypedDispatchEvent::MessageReactionRemove(serde_json::from_value(data)?)
            }
            "MESSAGE_REACTION_REMOVE_ALL" => {
                TypedDispatchEvent::MessageReactionRemoveAll(serde_json::from_value(data)?)
            }
            "MESSAGE_REACTION_REMOVE_EMOJI" => {
                TypedDispatchEvent::MessageReactionRemoveEmoji(serde_json::from_value(data)?)
            }
            "MESSAGE_POLL_VOTE_ADD" => {
                TypedDispatchEvent::MessagePollVoteAdd(serde_json::from_value(data)?)
            }
            "MESSAGE_POLL_VOTE_REMOVE" => {
                TypedDispatchEvent::MessagePollVoteRemove(serde_json::from_value(data)?)
            }
            "INTERACTION_CREATE" => {
                TypedDispatchEvent::InteractionCreate(Box::new(serde_json::from_value(data)?))
            }
            "PRESENCE_UPDATE" => TypedDispatchEvent::PresenceUpdate(serde_json::from_value(data)?),
            "VOICE_STATE_UPDATE" => {
                TypedDispatchEvent::VoiceStateUpdate(serde_json::from_value(data)?)
            }
            "WEBHOOKS_UPDATE" => TypedDispatchEvent::WebhooksUpdate(serde_json::from_value(data)?),
            "USER_UPDATE" => TypedDispatchEvent::UserUpdate(serde_json::from_value(data)?),
            _ => TypedDispatchEvent::Unknown {
                name: self.name.clone(),
                data,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::model::{
        AuditLogEvent, AutoModerationActionType, AutoModerationTriggerType,
        GuildScheduledEventEntityType, InteractionType, InviteTargetType, ReactionType,
    };

    use super::{DispatchEvent, TypedDispatchEvent};

    #[test]
    fn parses_ready() {
        let dispatch = DispatchEvent {
            name: "READY".to_owned(),
            sequence: 1,
            data: json!({
                "v": 10,
                "user": {"id":"1", "username":"gloam", "discriminator":"0"},
                "guilds": [{"id":"2", "unavailable":true}],
                "session_id": "session",
                "resume_gateway_url": "wss://gateway.discord.gg",
                "application": {"id":"3", "flags":0}
            }),
        };

        let TypedDispatchEvent::Ready(ready) = dispatch.typed().expect("ready") else {
            panic!("expected ready event");
        };
        assert_eq!(ready.v, 10);
        assert_eq!(ready.guilds[0].id.get(), 2);
    }

    #[test]
    fn parses_auto_moderation_action_execution() {
        let dispatch = DispatchEvent {
            name: "AUTO_MODERATION_ACTION_EXECUTION".to_owned(),
            sequence: 2,
            data: json!({
                "guild_id":"10",
                "action":{"type":1,"metadata":{"custom_message":"blocked"}},
                "rule_id":"20",
                "rule_trigger_type":6,
                "user_id":"30",
                "content":"profile text",
                "matched_keyword":"bad*",
                "matched_content":"badname"
            }),
        };

        let TypedDispatchEvent::AutoModerationActionExecution(event) =
            dispatch.typed().expect("automod execution")
        else {
            panic!("expected automod execution");
        };

        assert_eq!(event.action.kind, AutoModerationActionType::BLOCK_MESSAGE);
        assert_eq!(
            event.rule_trigger_type,
            AutoModerationTriggerType::MEMBER_PROFILE
        );
        assert_eq!(event.rule_id.get(), 20);
    }

    #[test]
    fn parses_guild_audit_log_entry_create() {
        let dispatch = DispatchEvent {
            name: "GUILD_AUDIT_LOG_ENTRY_CREATE".to_owned(),
            sequence: 3,
            data: json!({
                "guild_id":"10",
                "target_id":"20",
                "changes":[{"key":"name","new_value":"new name"}],
                "user_id":"30",
                "id":"40",
                "action_type":31,
                "reason":"rename"
            }),
        };

        let TypedDispatchEvent::GuildAuditLogEntryCreate(event) =
            dispatch.typed().expect("audit log entry")
        else {
            panic!("expected guild audit log entry");
        };

        assert_eq!(event.guild_id.get(), 10);
        assert_eq!(event.entry.action_type, AuditLogEvent::ROLE_UPDATE);
        assert_eq!(event.entry.id.get(), 40);
    }

    #[test]
    fn parses_scheduled_event_create() {
        let dispatch = DispatchEvent {
            name: "GUILD_SCHEDULED_EVENT_CREATE".to_owned(),
            sequence: 4,
            data: json!({
                "id":"100",
                "guild_id":"200",
                "channel_id":null,
                "name":"Meetup",
                "scheduled_start_time":"2026-09-01T18:00:00+00:00",
                "scheduled_end_time":"2026-09-01T20:00:00+00:00",
                "privacy_level":2,
                "status":1,
                "entity_type":3,
                "entity_id":null,
                "entity_metadata":{"location":"Chicago"},
                "recurrence_rule":null
            }),
        };

        let TypedDispatchEvent::GuildScheduledEventCreate(event) =
            dispatch.typed().expect("scheduled event")
        else {
            panic!("expected scheduled event create");
        };

        assert_eq!(event.entity_type, GuildScheduledEventEntityType::EXTERNAL);
        assert_eq!(event.id.get(), 100);
    }

    #[test]
    fn parses_scheduled_event_user_add() {
        let dispatch = DispatchEvent {
            name: "GUILD_SCHEDULED_EVENT_USER_ADD".to_owned(),
            sequence: 5,
            data: json!({
                "guild_scheduled_event_id":"100",
                "user_id":"200",
                "guild_id":"300"
            }),
        };

        let TypedDispatchEvent::GuildScheduledEventUserAdd(event) =
            dispatch.typed().expect("scheduled event user")
        else {
            panic!("expected scheduled event user add");
        };

        assert_eq!(event.guild_scheduled_event_id.get(), 100);
        assert_eq!(event.user_id.get(), 200);
    }

    #[test]
    fn parses_super_reaction_add() {
        let dispatch = DispatchEvent {
            name: "MESSAGE_REACTION_ADD".to_owned(),
            sequence: 6,
            data: json!({
                "user_id":"10",
                "channel_id":"20",
                "message_id":"30",
                "guild_id":"40",
                "member":{"roles":[],"deaf":false,"mute":false,"flags":0},
                "emoji":{"id":"50","name":"spark","animated":true},
                "message_author_id":"60",
                "burst":true,
                "burst_colors":["#ff00aa"],
                "type":1
            }),
        };

        let TypedDispatchEvent::MessageReactionAdd(event) = dispatch.typed().expect("reaction add")
        else {
            panic!("expected reaction add event");
        };

        assert_eq!(event.user_id.get(), 10);
        assert_eq!(event.kind, ReactionType::BURST);
        assert!(event.burst);
        assert_eq!(event.burst_colors, ["#ff00aa"]);
        assert_eq!(event.emoji.animated, Some(true));
    }

    #[test]
    fn parses_reaction_remove_emoji_with_deleted_name() {
        let dispatch = DispatchEvent {
            name: "MESSAGE_REACTION_REMOVE_EMOJI".to_owned(),
            sequence: 7,
            data: json!({
                "channel_id":"20",
                "message_id":"30",
                "guild_id":"40",
                "emoji":{"id":"50","name":null}
            }),
        };

        let TypedDispatchEvent::MessageReactionRemoveEmoji(event) =
            dispatch.typed().expect("reaction remove emoji")
        else {
            panic!("expected reaction remove emoji event");
        };

        assert_eq!(event.emoji.id.expect("emoji id").get(), 50);
        assert!(event.emoji.name.is_none());
    }

    #[test]
    fn parses_poll_vote_add() {
        let dispatch = DispatchEvent {
            name: "MESSAGE_POLL_VOTE_ADD".to_owned(),
            sequence: 8,
            data: json!({
                "user_id":"10",
                "channel_id":"20",
                "message_id":"30",
                "guild_id":"40",
                "answer_id":7
            }),
        };

        let TypedDispatchEvent::MessagePollVoteAdd(event) = dispatch.typed().expect("poll vote")
        else {
            panic!("expected poll vote add");
        };

        assert_eq!(event.answer_id, 7);
        assert_eq!(event.user_id.get(), 10);
    }

    #[test]
    fn parses_interaction_create() {
        let dispatch = DispatchEvent {
            name: "INTERACTION_CREATE".to_owned(),
            sequence: 9,
            data: json!({
                "id":"100",
                "application_id":"200",
                "type":1,
                "token":"token",
                "version":1,
                "entitlements":[],
                "authorizing_integration_owners":{},
                "attachment_size_limit":0
            }),
        };

        let TypedDispatchEvent::InteractionCreate(interaction) =
            dispatch.typed().expect("interaction create")
        else {
            panic!("expected interaction create event");
        };

        assert_eq!(interaction.kind, InteractionType::PING);
        assert_eq!(interaction.id.get(), 100);
    }

    #[test]
    fn parses_invite_create() {
        let dispatch = DispatchEvent {
            name: "INVITE_CREATE".to_owned(),
            sequence: 10,
            data: json!({
                "channel_id":"10",
                "code":"guest-code",
                "created_at":"2026-08-25T20:00:00+00:00",
                "guild_id":"20",
                "max_age":3600,
                "max_uses":5,
                "target_type":2,
                "temporary":false,
                "uses":0,
                "expires_at":null,
                "role_ids":["30","31"]
            }),
        };

        let TypedDispatchEvent::InviteCreate(event) = dispatch.typed().expect("invite create")
        else {
            panic!("expected invite create event");
        };

        assert_eq!(
            event.target_type,
            Some(InviteTargetType::EMBEDDED_APPLICATION)
        );
        assert_eq!(event.role_ids[0].get(), 30);
    }

    #[test]
    fn parses_webhooks_update() {
        let dispatch = DispatchEvent {
            name: "WEBHOOKS_UPDATE".to_owned(),
            sequence: 11,
            data: json!({"guild_id":"20", "channel_id":"10"}),
        };

        let TypedDispatchEvent::WebhooksUpdate(event) = dispatch.typed().expect("webhooks update")
        else {
            panic!("expected webhooks update event");
        };

        assert_eq!(event.guild_id.get(), 20);
        assert_eq!(event.channel_id.get(), 10);
    }

    #[test]
    fn preserves_unknown_dispatches() {
        let dispatch = DispatchEvent {
            name: "FUTURE_EVENT".to_owned(),
            sequence: 12,
            data: json!({"new": true}),
        };

        let TypedDispatchEvent::Unknown { name, data } = dispatch.typed().expect("unknown") else {
            panic!("expected unknown event");
        };
        assert_eq!(name, "FUTURE_EVENT");
        assert_eq!(data["new"], true);
    }
}
