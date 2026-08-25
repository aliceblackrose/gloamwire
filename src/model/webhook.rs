use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ApplicationId, ChannelId, GuildId, User, WebhookId};

/// Discord webhook type.
///
/// The numeric representation is retained so future webhook types remain
/// deserializable before Gloamwire adds dedicated handling for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WebhookType(pub u8);

impl WebhookType {
    pub const INCOMING: Self = Self(1);
    pub const CHANNEL_FOLLOWER: Self = Self(2);
    pub const APPLICATION: Self = Self(3);
}

/// Partial source guild included with a channel-follower webhook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookSourceGuild {
    pub id: GuildId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial source channel included with a channel-follower webhook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookSourceChannel {
    pub id: ChannelId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A Discord webhook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Webhook {
    pub id: WebhookId,
    #[serde(rename = "type")]
    pub kind: WebhookType,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub application_id: Option<ApplicationId>,
    #[serde(default)]
    pub source_guild: Option<WebhookSourceGuild>,
    #[serde(default)]
    pub source_channel: Option<WebhookSourceChannel>,
    #[serde(default)]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Webhook, WebhookType};

    #[test]
    fn parses_channel_follower_webhook() {
        let webhook: Webhook = serde_json::from_str(
            r#"{
                "type":2,
                "id":"752831914402115456",
                "name":"Guildy name",
                "avatar":"hash",
                "channel_id":"561885260615255432",
                "guild_id":"56188498421443265",
                "application_id":null,
                "source_guild":{
                    "id":"56188498421476534",
                    "name":"Source guild",
                    "icon":"icon",
                    "future_field":true
                },
                "source_channel":{
                    "id":"5618852344134324",
                    "name":"announcements"
                }
            }"#,
        )
        .expect("channel follower webhook");

        assert_eq!(webhook.kind, WebhookType::CHANNEL_FOLLOWER);
        assert_eq!(
            webhook.source_guild.as_ref().expect("source guild").extra["future_field"],
            true
        );
    }

    #[test]
    fn parses_application_webhook_with_null_channel_and_guild() {
        let webhook: Webhook = serde_json::from_str(
            r#"{
                "type":3,
                "id":"658822586720976555",
                "name":"Clyde",
                "avatar":"hash",
                "channel_id":null,
                "guild_id":null,
                "application_id":"658822586720976555"
            }"#,
        )
        .expect("application webhook");

        assert_eq!(webhook.kind, WebhookType::APPLICATION);
        assert!(webhook.channel_id.is_none());
        assert!(webhook.guild_id.is_none());
    }

    #[test]
    fn webhook_type_preserves_unknown_values() {
        let kind: WebhookType = serde_json::from_str("9").expect("webhook type");
        assert_eq!(kind, WebhookType(9));
    }
}
