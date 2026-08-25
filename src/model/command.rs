use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ApplicationId, ChannelType, CommandId, GuildId, Permissions, Snowflake};

/// Discord application-command type.
///
/// This remains a numeric newtype so future command types are retained rather
/// than rejected during deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationCommandType(pub u8);

impl ApplicationCommandType {
    pub const CHAT_INPUT: Self = Self(1);
    pub const USER: Self = Self(2);
    pub const MESSAGE: Self = Self(3);
    pub const PRIMARY_ENTRY_POINT: Self = Self(4);
}

impl Default for ApplicationCommandType {
    fn default() -> Self {
        Self::CHAT_INPUT
    }
}

/// Discord application-command option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationCommandOptionType(pub u8);

impl ApplicationCommandOptionType {
    pub const SUB_COMMAND: Self = Self(1);
    pub const SUB_COMMAND_GROUP: Self = Self(2);
    pub const STRING: Self = Self(3);
    pub const INTEGER: Self = Self(4);
    pub const BOOLEAN: Self = Self(5);
    pub const USER: Self = Self(6);
    pub const CHANNEL: Self = Self(7);
    pub const ROLE: Self = Self(8);
    pub const MENTIONABLE: Self = Self(9);
    pub const NUMBER: Self = Self(10);
    pub const ATTACHMENT: Self = Self(11);
}

/// Where an application is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationIntegrationType(pub u8);

impl ApplicationIntegrationType {
    pub const GUILD_INSTALL: Self = Self(0);
    pub const USER_INSTALL: Self = Self(1);
}

/// Surface where an interaction can be invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InteractionContextType(pub u8);

impl InteractionContextType {
    pub const GUILD: Self = Self(0);
    pub const BOT_DM: Self = Self(1);
    pub const PRIVATE_CHANNEL: Self = Self(2);
}

/// Handler used for a primary entry-point command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationCommandHandlerType(pub u8);

impl ApplicationCommandHandlerType {
    pub const APP_HANDLER: Self = Self(1);
    pub const DISCORD_LAUNCH_ACTIVITY: Self = Self(2);
}

/// Numeric bounds or choice values accepted by an application-command option.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApplicationCommandNumericValue {
    Integer(i64),
    Number(f64),
}

/// Value associated with an application-command option choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApplicationCommandChoiceValue {
    String(String),
    Integer(i64),
    Number(f64),
}

/// A fixed choice for a string, integer, or number command option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommandOptionChoice {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<BTreeMap<String, String>>,
    pub value: ApplicationCommandChoiceValue,
}

/// One option accepted by a chat-input application command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommandOption {
    #[serde(rename = "type")]
    pub kind: ApplicationCommandOptionType,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<BTreeMap<String, String>>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ApplicationCommandOptionChoice>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ApplicationCommandOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_types: Vec<ChannelType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<ApplicationCommandNumericValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<ApplicationCommandNumericValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_types: Vec<String>,
}

/// A registered Discord application command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommand {
    pub id: CommandId,
    #[serde(rename = "type", default)]
    pub kind: ApplicationCommandType,
    pub application_id: ApplicationId,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    pub name: String,
    #[serde(default)]
    pub name_localizations: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub name_localized: Option<String>,
    pub description: String,
    #[serde(default)]
    pub description_localizations: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub description_localized: Option<String>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    pub default_member_permissions: Option<Permissions>,
    #[serde(default)]
    pub dm_permission: Option<bool>,
    #[serde(default)]
    pub default_permission: Option<bool>,
    #[serde(default)]
    pub nsfw: Option<bool>,
    #[serde(default)]
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,
    #[serde(default)]
    pub contexts: Option<Vec<InteractionContextType>>,
    pub version: Snowflake,
    #[serde(default)]
    pub handler: Option<ApplicationCommandHandlerType>,
}

/// Target kind for one application-command permission overwrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationCommandPermissionType(pub u8);

impl ApplicationCommandPermissionType {
    pub const ROLE: Self = Self(1);
    pub const USER: Self = Self(2);
    pub const CHANNEL: Self = Self(3);
}

/// One role, user, or channel permission overwrite for an application command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationCommandPermission {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: ApplicationCommandPermissionType,
    pub permission: bool,
}

/// Permission overwrites for one command, or defaults keyed by application ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildApplicationCommandPermissions {
    pub id: Snowflake,
    pub application_id: ApplicationId,
    pub guild_id: GuildId,
    #[serde(default)]
    pub permissions: Vec<ApplicationCommandPermission>,
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationCommand, ApplicationCommandChoiceValue, ApplicationCommandHandlerType,
        ApplicationCommandOptionType, ApplicationCommandType, ApplicationIntegrationType,
        InteractionContextType,
    };

    #[test]
    fn parses_current_chat_input_command_fields() {
        let command: ApplicationCommand = serde_json::from_str(
            r#"{
                "id":"10",
                "type":1,
                "application_id":"20",
                "name":"upload",
                "description":"Upload a file",
                "options":[{
                    "type":11,
                    "name":"file",
                    "description":"File to upload",
                    "required":true,
                    "file_types":["image",".pdf"]
                }],
                "default_member_permissions":"2048",
                "integration_types":[0,1],
                "contexts":[0,1,2],
                "version":"30"
            }"#,
        )
        .expect("application command");

        assert_eq!(command.kind, ApplicationCommandType::CHAT_INPUT);
        assert_eq!(
            command.options[0].kind,
            ApplicationCommandOptionType::ATTACHMENT
        );
        assert_eq!(command.options[0].file_types, ["image", ".pdf"]);
        assert_eq!(
            command.integration_types.as_deref(),
            Some(
                &[
                    ApplicationIntegrationType::GUILD_INSTALL,
                    ApplicationIntegrationType::USER_INSTALL,
                ][..]
            )
        );
        assert_eq!(
            command.contexts.as_deref(),
            Some(
                &[
                    InteractionContextType::GUILD,
                    InteractionContextType::BOT_DM,
                    InteractionContextType::PRIVATE_CHANNEL,
                ][..]
            )
        );
    }

    #[test]
    fn parses_primary_entry_point_handler() {
        let command: ApplicationCommand = serde_json::from_str(
            r#"{
                "id":"10",
                "type":4,
                "application_id":"20",
                "name":"Launch",
                "description":"",
                "default_member_permissions":null,
                "version":"30",
                "handler":2
            }"#,
        )
        .expect("entry point command");

        assert_eq!(command.kind, ApplicationCommandType::PRIMARY_ENTRY_POINT);
        assert_eq!(
            command.handler,
            Some(ApplicationCommandHandlerType::DISCORD_LAUNCH_ACTIVITY)
        );
    }

    #[test]
    fn choice_values_preserve_integer_and_string_types() {
        let integer: ApplicationCommandChoiceValue = serde_json::from_str("42").expect("integer");
        let string: ApplicationCommandChoiceValue =
            serde_json::from_str(r#""forty-two""#).expect("string");

        assert_eq!(integer, ApplicationCommandChoiceValue::Integer(42));
        assert_eq!(
            string,
            ApplicationCommandChoiceValue::String("forty-two".to_owned())
        );
    }

    #[test]
    fn command_types_preserve_unknown_values() {
        let kind: ApplicationCommandType = serde_json::from_str("9").expect("command type");
        assert_eq!(kind, ApplicationCommandType(9));
    }

    #[test]
    fn command_options_omit_unused_request_fields() {
        let option = super::ApplicationCommandOption {
            kind: ApplicationCommandOptionType::STRING,
            name: "query".to_owned(),
            name_localizations: None,
            description: "Search text".to_owned(),
            description_localizations: None,
            required: None,
            choices: Vec::new(),
            options: Vec::new(),
            channel_types: Vec::new(),
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            autocomplete: None,
            file_types: Vec::new(),
        };
        let value = serde_json::to_value(option).expect("option");

        assert_eq!(
            value,
            serde_json::json!({"type":3,"name":"query","description":"Search text"})
        );
    }
}
