use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    ApplicationCommand, ApplicationId, AuditLogEntryId, AutoModerationRule, Channel, ChannelId,
    MessageId, Snowflake, User, UserId, Webhook,
};

/// Discord audit-log action type.
///
/// The numeric representation is retained so new Discord action types remain
/// deserializable without requiring an immediate Gloamwire release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuditLogEvent(pub u16);

impl AuditLogEvent {
    pub const GUILD_UPDATE: Self = Self(1);
    pub const CHANNEL_CREATE: Self = Self(10);
    pub const CHANNEL_UPDATE: Self = Self(11);
    pub const CHANNEL_DELETE: Self = Self(12);
    pub const CHANNEL_OVERWRITE_CREATE: Self = Self(13);
    pub const CHANNEL_OVERWRITE_UPDATE: Self = Self(14);
    pub const CHANNEL_OVERWRITE_DELETE: Self = Self(15);
    pub const MEMBER_KICK: Self = Self(20);
    pub const MEMBER_PRUNE: Self = Self(21);
    pub const MEMBER_BAN_ADD: Self = Self(22);
    pub const MEMBER_BAN_REMOVE: Self = Self(23);
    pub const MEMBER_UPDATE: Self = Self(24);
    pub const MEMBER_ROLE_UPDATE: Self = Self(25);
    pub const MEMBER_MOVE: Self = Self(26);
    pub const MEMBER_DISCONNECT: Self = Self(27);
    pub const BOT_ADD: Self = Self(28);
    pub const ROLE_CREATE: Self = Self(30);
    pub const ROLE_UPDATE: Self = Self(31);
    pub const ROLE_DELETE: Self = Self(32);
    pub const INVITE_CREATE: Self = Self(40);
    pub const INVITE_UPDATE: Self = Self(41);
    pub const INVITE_DELETE: Self = Self(42);
    pub const WEBHOOK_CREATE: Self = Self(50);
    pub const WEBHOOK_UPDATE: Self = Self(51);
    pub const WEBHOOK_DELETE: Self = Self(52);
    pub const EMOJI_CREATE: Self = Self(60);
    pub const EMOJI_UPDATE: Self = Self(61);
    pub const EMOJI_DELETE: Self = Self(62);
    pub const MESSAGE_DELETE: Self = Self(72);
    pub const MESSAGE_BULK_DELETE: Self = Self(73);
    pub const MESSAGE_PIN: Self = Self(74);
    pub const MESSAGE_UNPIN: Self = Self(75);
    pub const INTEGRATION_CREATE: Self = Self(80);
    pub const INTEGRATION_UPDATE: Self = Self(81);
    pub const INTEGRATION_DELETE: Self = Self(82);
    pub const STAGE_INSTANCE_CREATE: Self = Self(83);
    pub const STAGE_INSTANCE_UPDATE: Self = Self(84);
    pub const STAGE_INSTANCE_DELETE: Self = Self(85);
    pub const STICKER_CREATE: Self = Self(90);
    pub const STICKER_UPDATE: Self = Self(91);
    pub const STICKER_DELETE: Self = Self(92);
    pub const GUILD_SCHEDULED_EVENT_CREATE: Self = Self(100);
    pub const GUILD_SCHEDULED_EVENT_UPDATE: Self = Self(101);
    pub const GUILD_SCHEDULED_EVENT_DELETE: Self = Self(102);
    pub const THREAD_CREATE: Self = Self(110);
    pub const THREAD_UPDATE: Self = Self(111);
    pub const THREAD_DELETE: Self = Self(112);
    pub const APPLICATION_COMMAND_PERMISSION_UPDATE: Self = Self(121);
    pub const SOUNDBOARD_SOUND_CREATE: Self = Self(130);
    pub const SOUNDBOARD_SOUND_UPDATE: Self = Self(131);
    pub const SOUNDBOARD_SOUND_DELETE: Self = Self(132);
    pub const AUTO_MODERATION_RULE_CREATE: Self = Self(140);
    pub const AUTO_MODERATION_RULE_UPDATE: Self = Self(141);
    pub const AUTO_MODERATION_RULE_DELETE: Self = Self(142);
    pub const AUTO_MODERATION_BLOCK_MESSAGE: Self = Self(143);
    pub const AUTO_MODERATION_FLAG_TO_CHANNEL: Self = Self(144);
    pub const AUTO_MODERATION_USER_COMMUNICATION_DISABLED: Self = Self(145);
    pub const AUTO_MODERATION_QUARANTINE_USER: Self = Self(146);
    pub const CREATOR_MONETIZATION_REQUEST_CREATED: Self = Self(150);
    pub const CREATOR_MONETIZATION_TERMS_ACCEPTED: Self = Self(151);
    pub const ONBOARDING_PROMPT_CREATE: Self = Self(163);
    pub const ONBOARDING_PROMPT_UPDATE: Self = Self(164);
    pub const ONBOARDING_PROMPT_DELETE: Self = Self(165);
    pub const ONBOARDING_CREATE: Self = Self(166);
    pub const ONBOARDING_UPDATE: Self = Self(167);
    pub const HOME_SETTINGS_CREATE: Self = Self(190);
    pub const HOME_SETTINGS_UPDATE: Self = Self(191);
    pub const VOICE_CHANNEL_STATUS_CREATE: Self = Self(192);
    pub const VOICE_CHANNEL_STATUS_DELETE: Self = Self(193);
}

/// One heterogeneous field change in an audit log entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLogChange {
    #[serde(default)]
    pub new_value: Option<Value>,
    #[serde(default)]
    pub old_value: Option<Value>,
    pub key: String,
}

/// Additional information attached to audit-log actions that need context.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLogEntryOptions {
    #[serde(default)]
    pub application_id: Option<ApplicationId>,
    #[serde(default)]
    pub auto_moderation_rule_name: Option<String>,
    #[serde(default)]
    pub auto_moderation_rule_trigger_type: Option<String>,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub count: Option<String>,
    #[serde(default)]
    pub delete_member_days: Option<String>,
    #[serde(default)]
    pub id: Option<Snowflake>,
    #[serde(default)]
    pub members_removed: Option<String>,
    #[serde(default)]
    pub message_id: Option<MessageId>,
    #[serde(default)]
    pub role_name: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub integration_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// One administrative action recorded in a guild audit log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub target_id: Option<String>,
    #[serde(default)]
    pub changes: Vec<AuditLogChange>,
    pub user_id: Option<UserId>,
    pub id: AuditLogEntryId,
    pub action_type: AuditLogEvent,
    #[serde(default)]
    pub options: Option<AuditLogEntryOptions>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Partial integration object included in audit-log responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLogIntegration {
    pub id: Snowflake,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub account: Value,
    #[serde(default)]
    pub application_id: Option<ApplicationId>,
}

/// Discord's guild audit-log response object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AuditLog {
    #[serde(default)]
    pub application_commands: Vec<ApplicationCommand>,
    #[serde(default)]
    pub audit_log_entries: Vec<AuditLogEntry>,
    #[serde(default)]
    pub auto_moderation_rules: Vec<AutoModerationRule>,
    /// Scheduled events remain lossless until their dedicated Phase 3 model slice.
    #[serde(default)]
    pub guild_scheduled_events: Vec<Value>,
    #[serde(default)]
    pub integrations: Vec<AuditLogIntegration>,
    #[serde(default)]
    pub threads: Vec<Channel>,
    #[serde(default)]
    pub users: Vec<User>,
    #[serde(default)]
    pub webhooks: Vec<Webhook>,
}

#[cfg(test)]
mod tests {
    use super::{AuditLog, AuditLogEntry, AuditLogEvent};

    #[test]
    fn parses_heterogeneous_audit_log_entry_changes() {
        let entry: AuditLogEntry = serde_json::from_str(
            r#"{
                "target_id":"123",
                "changes":[
                    {"key":"name","old_value":"old","new_value":"new"},
                    {"key":"$add","new_value":[{"id":"5","name":"Role"}]}
                ],
                "user_id":"456",
                "id":"789",
                "action_type":25,
                "options":{"integration_type":"discord"},
                "reason":"sync roles"
            }"#,
        )
        .expect("audit log entry");

        assert_eq!(entry.action_type, AuditLogEvent::MEMBER_ROLE_UPDATE);
        assert_eq!(entry.changes[1].new_value.as_ref().expect("new value")[0]["name"], "Role");
    }

    #[test]
    fn parses_audit_log_with_typed_related_objects() {
        let audit_log: AuditLog = serde_json::from_str(
            r#"{
                "application_commands":[],
                "audit_log_entries":[],
                "auto_moderation_rules":[],
                "guild_scheduled_events":[],
                "integrations":[{
                    "id":"33590653072239123",
                    "name":"A Name",
                    "type":"twitch",
                    "account":{"name":"twitchusername","id":"1234567"},
                    "application_id":"94651234501213162"
                }],
                "threads":[],
                "users":[],
                "webhooks":[]
            }"#,
        )
        .expect("audit log");

        assert_eq!(audit_log.integrations[0].kind, "twitch");
    }

    #[test]
    fn audit_log_events_preserve_unknown_values() {
        let event: AuditLogEvent = serde_json::from_str("999").expect("audit event");
        assert_eq!(event, AuditLogEvent(999));
    }
}
