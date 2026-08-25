use reqwest::Method;
use serde::Serialize;

use crate::{
    Result,
    model::{
        AutoModerationAction, AutoModerationEventType, AutoModerationRule, AutoModerationRuleId,
        AutoModerationTriggerMetadata, AutoModerationTriggerType, ChannelId, GuildId, RoleId,
    },
};

use super::{
    RestClient,
    encoding::audit_reason_headers,
    guild::guild_route,
    route::{RetrySafety, Route},
};

/// Parameters for creating an Auto Moderation rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateAutoModerationRule {
    pub name: String,
    pub event_type: AutoModerationEventType,
    pub trigger_type: AutoModerationTriggerType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_metadata: Option<AutoModerationTriggerMetadata>,
    pub actions: Vec<AutoModerationAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exempt_roles: Vec<RoleId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exempt_channels: Vec<ChannelId>,
}

/// Parameters for modifying an Auto Moderation rule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModifyAutoModerationRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<AutoModerationEventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_metadata: Option<AutoModerationTriggerMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<AutoModerationAction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_roles: Option<Vec<RoleId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_channels: Option<Vec<ChannelId>>,
}

impl RestClient {
    /// Lists all Auto Moderation rules configured for a guild.
    pub async fn list_auto_moderation_rules(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<AutoModerationRule>> {
        self.request_json::<Vec<AutoModerationRule>, ()>(
            auto_moderation_rules_route(Method::GET, guild_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Returns one Auto Moderation rule.
    pub async fn get_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule_id: AutoModerationRuleId,
    ) -> Result<AutoModerationRule> {
        self.request_json::<AutoModerationRule, ()>(
            auto_moderation_rule_route(Method::GET, guild_id, rule_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Creates an Auto Moderation rule.
    pub async fn create_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule: &CreateAutoModerationRule,
        reason: Option<&str>,
    ) -> Result<AutoModerationRule> {
        self.request_json_with_headers(
            auto_moderation_rules_route(Method::POST, guild_id, RetrySafety::Unsafe),
            Some(rule),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Modifies an Auto Moderation rule.
    pub async fn modify_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule_id: AutoModerationRuleId,
        modify: &ModifyAutoModerationRule,
        reason: Option<&str>,
    ) -> Result<AutoModerationRule> {
        self.request_json_with_headers(
            auto_moderation_rule_route(Method::PATCH, guild_id, rule_id, RetrySafety::Unsafe),
            Some(modify),
            audit_reason_headers(reason),
        )
        .await
    }

    /// Deletes an Auto Moderation rule.
    pub async fn delete_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule_id: AutoModerationRuleId,
        reason: Option<&str>,
    ) -> Result<()> {
        self.request_empty::<()>(
            auto_moderation_rule_route(Method::DELETE, guild_id, rule_id, RetrySafety::Safe),
            None,
            audit_reason_headers(reason),
        )
        .await
    }
}

fn auto_moderation_rules_route(method: Method, guild_id: GuildId, safety: RetrySafety) -> Route {
    guild_route(
        method,
        guild_id,
        "/auto-moderation/rules",
        "/guilds/{guild_id}/auto-moderation/rules",
        safety,
    )
}

fn auto_moderation_rule_route(
    method: Method,
    guild_id: GuildId,
    rule_id: AutoModerationRuleId,
    safety: RetrySafety,
) -> Route {
    guild_route(
        method,
        guild_id,
        &format!("/auto-moderation/rules/{rule_id}"),
        "/guilds/{guild_id}/auto-moderation/rules/{rule_id}",
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::{
        AutoModerationAction, AutoModerationActionType, AutoModerationEventType,
        AutoModerationTriggerType,
    };

    use super::CreateAutoModerationRule;

    #[test]
    fn create_rule_omits_optional_defaults() {
        let rule = CreateAutoModerationRule {
            name: "Spam".to_owned(),
            event_type: AutoModerationEventType::MESSAGE_SEND,
            trigger_type: AutoModerationTriggerType::SPAM,
            trigger_metadata: None,
            actions: vec![AutoModerationAction {
                kind: AutoModerationActionType::BLOCK_MESSAGE,
                metadata: None,
            }],
            enabled: None,
            exempt_roles: Vec::new(),
            exempt_channels: Vec::new(),
        };
        let value = serde_json::to_value(rule).expect("create rule");

        assert!(value.get("trigger_metadata").is_none());
        assert!(value.get("exempt_roles").is_none());
        assert_eq!(value["trigger_type"], 3);
    }
}
