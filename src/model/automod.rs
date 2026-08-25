use serde::{Deserialize, Serialize};

use super::{AutoModerationRuleId, ChannelId, GuildId, RoleId, UserId};

/// Discord Auto Moderation event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoModerationEventType(pub u8);

impl AutoModerationEventType {
    pub const MESSAGE_SEND: Self = Self(1);
    pub const MEMBER_UPDATE: Self = Self(2);
}

/// Discord Auto Moderation trigger type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoModerationTriggerType(pub u8);

impl AutoModerationTriggerType {
    pub const KEYWORD: Self = Self(1);
    pub const SPAM: Self = Self(3);
    pub const KEYWORD_PRESET: Self = Self(4);
    pub const MENTION_SPAM: Self = Self(5);
    pub const MEMBER_PROFILE: Self = Self(6);
}

/// Discord Auto Moderation keyword preset type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoModerationKeywordPresetType(pub u8);

impl AutoModerationKeywordPresetType {
    pub const PROFANITY: Self = Self(1);
    pub const SEXUAL_CONTENT: Self = Self(2);
    pub const SLURS: Self = Self(3);
}

/// Metadata used to determine whether an Auto Moderation rule is triggered.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationTriggerMetadata {
    #[serde(default)]
    pub keyword_filter: Vec<String>,
    #[serde(default)]
    pub regex_patterns: Vec<String>,
    #[serde(default)]
    pub presets: Vec<AutoModerationKeywordPresetType>,
    #[serde(default)]
    pub allow_list: Vec<String>,
    #[serde(default)]
    pub mention_total_limit: Option<u8>,
    #[serde(default)]
    pub mention_raid_protection_enabled: Option<bool>,
}

/// Discord Auto Moderation action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoModerationActionType(pub u8);

impl AutoModerationActionType {
    pub const BLOCK_MESSAGE: Self = Self(1);
    pub const SEND_ALERT_MESSAGE: Self = Self(2);
    pub const TIMEOUT: Self = Self(3);
    pub const BLOCK_MEMBER_INTERACTION: Self = Self(4);
}

/// Metadata required by some Auto Moderation actions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationActionMetadata {
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub duration_seconds: Option<u32>,
    #[serde(default)]
    pub custom_message: Option<String>,
}

/// Action executed when an Auto Moderation rule is triggered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationAction {
    #[serde(rename = "type")]
    pub kind: AutoModerationActionType,
    #[serde(default)]
    pub metadata: Option<AutoModerationActionMetadata>,
}

/// A Discord Auto Moderation rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoModerationRule {
    pub id: AutoModerationRuleId,
    pub guild_id: GuildId,
    pub name: String,
    pub creator_id: UserId,
    pub event_type: AutoModerationEventType,
    pub trigger_type: AutoModerationTriggerType,
    pub trigger_metadata: AutoModerationTriggerMetadata,
    #[serde(default)]
    pub actions: Vec<AutoModerationAction>,
    pub enabled: bool,
    #[serde(default)]
    pub exempt_roles: Vec<RoleId>,
    #[serde(default)]
    pub exempt_channels: Vec<ChannelId>,
}

#[cfg(test)]
mod tests {
    use super::{
        AutoModerationActionType, AutoModerationEventType, AutoModerationRule,
        AutoModerationTriggerType,
    };

    #[test]
    fn parses_current_keyword_rule() {
        let rule: AutoModerationRule = serde_json::from_str(
            r#"{
                "id":"969707018069872670",
                "guild_id":"613425648685547541",
                "name":"Keyword Filter 1",
                "creator_id":"423457898095789043",
                "trigger_type":1,
                "event_type":1,
                "actions":[
                    {"type":1,"metadata":{"custom_message":"Keep it civil"}},
                    {"type":2,"metadata":{"channel_id":"123456789123456789"}},
                    {"type":3,"metadata":{"duration_seconds":60}},
                    {"type":4}
                ],
                "trigger_metadata":{
                    "keyword_filter":["cat*","*dog"],
                    "regex_patterns":["(b|c)at"]
                },
                "enabled":true,
                "exempt_roles":["323456789123456789"],
                "exempt_channels":["523456789123456789"]
            }"#,
        )
        .expect("auto moderation rule");

        assert_eq!(rule.event_type, AutoModerationEventType::MESSAGE_SEND);
        assert_eq!(rule.trigger_type, AutoModerationTriggerType::KEYWORD);
        assert_eq!(
            rule.actions[3].kind,
            AutoModerationActionType::BLOCK_MEMBER_INTERACTION
        );
    }

    #[test]
    fn parses_member_profile_rule() {
        let rule: AutoModerationRule = serde_json::from_str(
            r#"{
                "id":"1",
                "guild_id":"2",
                "name":"Profile filter",
                "creator_id":"3",
                "trigger_type":6,
                "event_type":2,
                "actions":[{"type":1}],
                "trigger_metadata":{"keyword_filter":["bad*"]},
                "enabled":true,
                "exempt_roles":[],
                "exempt_channels":[]
            }"#,
        )
        .expect("member profile rule");

        assert_eq!(rule.event_type, AutoModerationEventType::MEMBER_UPDATE);
        assert_eq!(rule.trigger_type, AutoModerationTriggerType::MEMBER_PROFILE);
    }

    #[test]
    fn trigger_types_preserve_unknown_values() {
        let kind: AutoModerationTriggerType = serde_json::from_str("99").expect("trigger type");
        assert_eq!(kind, AutoModerationTriggerType(99));
    }
}
