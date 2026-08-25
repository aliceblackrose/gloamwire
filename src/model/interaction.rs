use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    ApplicationCommandOptionType, ApplicationCommandType, ApplicationId,
    ApplicationIntegrationType, Attachment, AttachmentId, Channel, ChannelId, CommandId, Component,
    ComponentType, GuildId, GuildMember, InteractionContextType, InteractionId, Message, MessageId,
    Permissions, Role, RoleId, Snowflake, User, UserId,
};

/// Discord interaction type.
///
/// Numeric representation is retained so future interaction types remain
/// deserializable before Gloamwire adds dedicated handling for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InteractionType(pub u8);

impl InteractionType {
    pub const PING: Self = Self(1);
    pub const APPLICATION_COMMAND: Self = Self(2);
    pub const MESSAGE_COMPONENT: Self = Self(3);
    pub const APPLICATION_COMMAND_AUTOCOMPLETE: Self = Self(4);
    pub const MODAL_SUBMIT: Self = Self(5);
}

/// Installation owners that authorized an interaction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthorizingIntegrationOwners(pub BTreeMap<String, Snowflake>);

impl AuthorizingIntegrationOwners {
    /// Returns the authorizing guild/user ID for one installation context.
    #[must_use]
    pub fn get(&self, integration_type: ApplicationIntegrationType) -> Option<Snowflake> {
        self.0.get(&integration_type.0.to_string()).copied()
    }
}

/// A Discord interaction received over the Gateway or interactions webhook.
///
/// `data` stays lossless because its shape depends on the interaction type.
/// Typed accessors are available for command, message-component, and modal-submit
/// interactions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interaction {
    pub id: InteractionId,
    pub application_id: ApplicationId,
    #[serde(rename = "type")]
    pub kind: InteractionType,
    #[serde(default)]
    pub data: Option<Value>,
    /// Discord's partial guild payload. Kept raw until the dedicated partial-guild model lands.
    #[serde(default)]
    pub guild: Option<Value>,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub channel: Option<Channel>,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub member: Option<GuildMember>,
    #[serde(default)]
    pub user: Option<User>,
    pub token: String,
    pub version: u8,
    #[serde(default)]
    pub message: Option<Message>,
    #[serde(default)]
    pub app_permissions: Option<Permissions>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub guild_locale: Option<String>,
    /// Monetization entitlement objects are preserved until their Phase 3 model slice.
    #[serde(default)]
    pub entitlements: Vec<Value>,
    #[serde(default)]
    pub authorizing_integration_owners: AuthorizingIntegrationOwners,
    #[serde(default)]
    pub context: Option<InteractionContextType>,
    #[serde(default)]
    pub attachment_size_limit: Option<u64>,
}

impl Interaction {
    /// Parses command-specific data for command and autocomplete interactions.
    pub fn application_command_data(
        &self,
    ) -> serde_json::Result<Option<ApplicationCommandInteractionData>> {
        if self.kind != InteractionType::APPLICATION_COMMAND
            && self.kind != InteractionType::APPLICATION_COMMAND_AUTOCOMPLETE
        {
            return Ok(None);
        }

        self.data.clone().map(serde_json::from_value).transpose()
    }

    /// Parses data for a message-component interaction.
    pub fn message_component_data(
        &self,
    ) -> serde_json::Result<Option<MessageComponentInteractionData>> {
        if self.kind != InteractionType::MESSAGE_COMPONENT {
            return Ok(None);
        }

        self.data.clone().map(serde_json::from_value).transpose()
    }

    /// Parses data for a modal-submit interaction.
    pub fn modal_submit_data(&self) -> serde_json::Result<Option<ModalSubmitInteractionData>> {
        if self.kind != InteractionType::MODAL_SUBMIT {
            return Ok(None);
        }

        self.data.clone().map(serde_json::from_value).transpose()
    }
}

/// Data sent for application-command and autocomplete interactions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommandInteractionData {
    pub id: CommandId,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ApplicationCommandType,
    #[serde(default)]
    pub resolved: Option<InteractionResolvedData>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandInteractionDataOption>,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub target_id: Option<Snowflake>,
}

/// One submitted command option in an interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommandInteractionDataOption {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ApplicationCommandOptionType,
    #[serde(default)]
    pub value: Option<ApplicationCommandInteractionValue>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandInteractionDataOption>,
    #[serde(default)]
    pub focused: Option<bool>,
}

/// User-submitted application-command option value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApplicationCommandInteractionValue {
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(String),
}

/// Data sent when a user activates an interactive message component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageComponentInteractionData {
    pub custom_id: String,
    pub component_type: ComponentType,
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub resolved: Option<InteractionResolvedData>,
}

/// Data submitted by a Discord modal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModalSubmitInteractionData {
    pub custom_id: String,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub resolved: Option<InteractionResolvedData>,
}

/// Objects resolved from IDs submitted in an interaction.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InteractionResolvedData {
    #[serde(default)]
    pub users: BTreeMap<UserId, User>,
    #[serde(default)]
    pub members: BTreeMap<UserId, GuildMember>,
    #[serde(default)]
    pub roles: BTreeMap<RoleId, Role>,
    #[serde(default)]
    pub channels: BTreeMap<ChannelId, Channel>,
    /// Resolved messages are partial objects and remain raw until a partial-message model is added.
    #[serde(default)]
    pub messages: BTreeMap<MessageId, Value>,
    #[serde(default)]
    pub attachments: BTreeMap<AttachmentId, Attachment>,
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationCommandInteractionValue, AuthorizingIntegrationOwners, Interaction,
        InteractionType,
    };
    use crate::model::{
        ApplicationIntegrationType, ComponentType, ComponentValue, InteractionContextType,
    };

    #[test]
    fn parses_current_application_command_interaction() {
        let interaction: Interaction = serde_json::from_str(
            r#"{
                "id":"100",
                "application_id":"200",
                "type":2,
                "data":{
                    "id":"300",
                    "name":"upload",
                    "type":1,
                    "resolved":{
                        "attachments":{
                            "500":{
                                "id":"500",
                                "filename":"photo.png",
                                "size":1234,
                                "url":"https://cdn.discordapp.com/a",
                                "proxy_url":"https://media.discordapp.net/a"
                            }
                        }
                    },
                    "options":[{
                        "name":"file",
                        "type":11,
                        "value":"500"
                    }]
                },
                "guild_id":"400",
                "channel":{"id":"600","type":0,"name":"general"},
                "channel_id":"600",
                "member":{"roles":[],"permissions":"2048"},
                "token":"token",
                "version":1,
                "app_permissions":"2048",
                "locale":"en-US",
                "guild_locale":"en-US",
                "entitlements":[],
                "authorizing_integration_owners":{"0":"400","1":"700"},
                "context":0,
                "attachment_size_limit":10485760
            }"#,
        )
        .expect("interaction");

        assert_eq!(interaction.kind, InteractionType::APPLICATION_COMMAND);
        assert_eq!(interaction.context, Some(InteractionContextType::GUILD));
        assert_eq!(interaction.attachment_size_limit, Some(10_485_760));
        assert_eq!(
            interaction
                .authorizing_integration_owners
                .get(ApplicationIntegrationType::GUILD_INSTALL)
                .expect("guild owner")
                .get(),
            400
        );

        let data = interaction
            .application_command_data()
            .expect("command data")
            .expect("command interaction");
        assert_eq!(data.id.get(), 300);
        assert_eq!(
            data.resolved
                .as_ref()
                .expect("resolved data")
                .attachments
                .len(),
            1
        );
        assert_eq!(
            data.options[0].value,
            Some(ApplicationCommandInteractionValue::String("500".to_owned()))
        );
    }

    #[test]
    fn parses_message_component_interaction() {
        let interaction: Interaction = serde_json::from_str(
            r#"{
                "id":"100",
                "application_id":"200",
                "type":3,
                "data":{
                    "component_type":8,
                    "id":2,
                    "custom_id":"notification_channel",
                    "values":["333"],
                    "resolved":{
                        "channels":{
                            "333":{"id":"333","type":0,"name":"general"}
                        }
                    }
                },
                "token":"token",
                "version":1,
                "entitlements":[],
                "authorizing_integration_owners":{},
                "attachment_size_limit":0
            }"#,
        )
        .expect("message component interaction");

        let data = interaction
            .message_component_data()
            .expect("component data")
            .expect("component interaction");
        assert_eq!(data.component_type, ComponentType::CHANNEL_SELECT);
        assert_eq!(data.id, Some(2));
        assert_eq!(data.values, ["333"]);
        assert_eq!(
            data.resolved
                .as_ref()
                .expect("resolved data")
                .channels
                .len(),
            1
        );
    }

    #[test]
    fn parses_modal_submit_interaction() {
        let interaction: Interaction = serde_json::from_str(
            r#"{
                "id":"100",
                "application_id":"200",
                "type":5,
                "data":{
                    "custom_id":"settings_modal",
                    "components":[{
                        "type":18,
                        "id":1,
                        "component":{
                            "type":23,
                            "id":2,
                            "custom_id":"confirm",
                            "value":true
                        }
                    }]
                },
                "token":"token",
                "version":1,
                "entitlements":[],
                "authorizing_integration_owners":{},
                "attachment_size_limit":0
            }"#,
        )
        .expect("modal interaction");

        let data = interaction
            .modal_submit_data()
            .expect("modal data")
            .expect("modal submit");
        let checkbox = data.components[0]
            .component
            .as_deref()
            .expect("checkbox component");
        assert_eq!(checkbox.kind, ComponentType::CHECKBOX);
        assert_eq!(checkbox.value, Some(ComponentValue::Boolean(true)));
    }

    #[test]
    fn non_command_interaction_has_no_command_data() {
        let interaction: Interaction = serde_json::from_str(
            r#"{
                "id":"100",
                "application_id":"200",
                "type":1,
                "token":"token",
                "version":1,
                "entitlements":[],
                "authorizing_integration_owners":{},
                "attachment_size_limit":0
            }"#,
        )
        .expect("ping interaction");

        assert!(
            interaction
                .application_command_data()
                .expect("command data")
                .is_none()
        );
    }

    #[test]
    fn authorizing_owners_default_empty() {
        let owners = AuthorizingIntegrationOwners::default();
        assert!(
            owners
                .get(ApplicationIntegrationType::USER_INSTALL)
                .is_none()
        );
    }
}
