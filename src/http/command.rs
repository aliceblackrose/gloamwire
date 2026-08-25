use std::collections::BTreeMap;

use reqwest::{
    Method,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::Serialize;

use crate::{
    Error, Result,
    model::{
        ApplicationCommand, ApplicationCommandHandlerType, ApplicationCommandOption,
        ApplicationCommandPermission, ApplicationCommandType, ApplicationId,
        ApplicationIntegrationType, CommandId, GuildApplicationCommandPermissions, GuildId,
        InteractionContextType, Permissions, Snowflake,
    },
};

use super::{
    RestClient,
    route::{RetrySafety, Route},
};

/// Definition accepted when creating an application command.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreateApplicationCommand {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ApplicationCommandOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<Option<Permissions>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<InteractionContextType>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ApplicationCommandType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<ApplicationCommandHandlerType>,
}

impl CreateApplicationCommand {
    /// Creates a chat-input command definition.
    #[must_use]
    pub fn chat_input(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            name_localizations: None,
            description: Some(description.into()),
            description_localizations: None,
            options: Vec::new(),
            default_member_permissions: None,
            dm_permission: None,
            default_permission: None,
            integration_types: None,
            contexts: None,
            kind: Some(ApplicationCommandType::CHAT_INPUT),
            nsfw: None,
            handler: None,
        }
    }

    /// Creates a context-menu or primary-entry-point command definition.
    #[must_use]
    pub fn named(name: impl Into<String>, kind: ApplicationCommandType) -> Self {
        Self {
            name: name.into(),
            name_localizations: None,
            description: None,
            description_localizations: None,
            options: Vec::new(),
            default_member_permissions: None,
            dm_permission: None,
            default_permission: None,
            integration_types: None,
            contexts: None,
            kind: Some(kind),
            nsfw: None,
            handler: None,
        }
    }
}

/// Command definition accepted by bulk-overwrite endpoints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BulkOverwriteApplicationCommand {
    /// Existing command ID, supported by Discord's guild bulk-overwrite endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<CommandId>,
    #[serde(flatten)]
    pub command: CreateApplicationCommand,
}

impl From<CreateApplicationCommand> for BulkOverwriteApplicationCommand {
    fn from(command: CreateApplicationCommand) -> Self {
        Self { id: None, command }
    }
}

/// Fields accepted by application-command edit endpoints.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct EditApplicationCommand {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<Option<BTreeMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<Option<BTreeMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<Option<Permissions>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Option<Vec<InteractionContextType>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<ApplicationCommandHandlerType>,
}

/// Body used to replace one command's permission overwrites.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EditApplicationCommandPermissions {
    pub permissions: Vec<ApplicationCommandPermission>,
}

impl RestClient {
    /// Lists global commands for an application.
    pub async fn get_global_application_commands(
        &self,
        application_id: ApplicationId,
        with_localizations: bool,
    ) -> Result<Vec<ApplicationCommand>> {
        self.request_json::<Vec<ApplicationCommand>, ()>(
            global_commands_route(
                Method::GET,
                application_id,
                localization_query(with_localizations),
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Creates or replaces a same-named global application command.
    pub async fn create_global_application_command(
        &self,
        application_id: ApplicationId,
        command: &CreateApplicationCommand,
    ) -> Result<ApplicationCommand> {
        self.request_json(
            global_commands_route(Method::POST, application_id, "", RetrySafety::Unsafe),
            Some(command),
        )
        .await
    }

    /// Returns one global application command.
    pub async fn get_global_application_command(
        &self,
        application_id: ApplicationId,
        command_id: CommandId,
    ) -> Result<ApplicationCommand> {
        self.request_json::<ApplicationCommand, ()>(
            global_command_route(Method::GET, application_id, command_id, RetrySafety::Safe),
            None,
        )
        .await
    }

    /// Edits one global application command.
    pub async fn edit_global_application_command(
        &self,
        application_id: ApplicationId,
        command_id: CommandId,
        command: &EditApplicationCommand,
    ) -> Result<ApplicationCommand> {
        self.request_json(
            global_command_route(
                Method::PATCH,
                application_id,
                command_id,
                RetrySafety::Unsafe,
            ),
            Some(command),
        )
        .await
    }

    /// Deletes one global application command.
    pub async fn delete_global_application_command(
        &self,
        application_id: ApplicationId,
        command_id: CommandId,
    ) -> Result<()> {
        self.request_empty::<()>(
            global_command_route(
                Method::DELETE,
                application_id,
                command_id,
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Replaces every global command for an application.
    pub async fn bulk_overwrite_global_application_commands(
        &self,
        application_id: ApplicationId,
        commands: &[CreateApplicationCommand],
    ) -> Result<Vec<ApplicationCommand>> {
        self.request_json(
            global_commands_route(Method::PUT, application_id, "", RetrySafety::Safe),
            Some(commands),
        )
        .await
    }

    /// Lists application commands registered in one guild.
    pub async fn get_guild_application_commands(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        with_localizations: bool,
    ) -> Result<Vec<ApplicationCommand>> {
        self.request_json::<Vec<ApplicationCommand>, ()>(
            guild_commands_route(
                Method::GET,
                application_id,
                guild_id,
                localization_query(with_localizations),
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Creates or replaces a same-named guild application command.
    pub async fn create_guild_application_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command: &CreateApplicationCommand,
    ) -> Result<ApplicationCommand> {
        self.request_json(
            guild_commands_route(
                Method::POST,
                application_id,
                guild_id,
                "",
                RetrySafety::Unsafe,
            ),
            Some(command),
        )
        .await
    }

    /// Returns one guild application command.
    pub async fn get_guild_application_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
    ) -> Result<ApplicationCommand> {
        self.request_json::<ApplicationCommand, ()>(
            guild_command_route(
                Method::GET,
                application_id,
                guild_id,
                command_id,
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Edits one guild application command.
    pub async fn edit_guild_application_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
        command: &EditApplicationCommand,
    ) -> Result<ApplicationCommand> {
        self.request_json(
            guild_command_route(
                Method::PATCH,
                application_id,
                guild_id,
                command_id,
                RetrySafety::Unsafe,
            ),
            Some(command),
        )
        .await
    }

    /// Deletes one guild application command.
    pub async fn delete_guild_application_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: CommandId,
    ) -> Result<()> {
        self.request_empty::<()>(
            guild_command_route(
                Method::DELETE,
                application_id,
                guild_id,
                command_id,
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Replaces every command for an application in one guild.
    pub async fn bulk_overwrite_guild_application_commands(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        commands: &[BulkOverwriteApplicationCommand],
    ) -> Result<Vec<ApplicationCommand>> {
        self.request_json(
            guild_commands_route(Method::PUT, application_id, guild_id, "", RetrySafety::Safe),
            Some(commands),
        )
        .await
    }

    /// Lists permission overwrites for every application command in a guild.
    pub async fn get_guild_application_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
    ) -> Result<Vec<GuildApplicationCommandPermissions>> {
        self.request_json::<Vec<GuildApplicationCommandPermissions>, ()>(
            command_permissions_collection_route(
                Method::GET,
                application_id,
                guild_id,
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Returns permission overwrites for one command or application defaults.
    pub async fn get_application_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_or_application_id: Snowflake,
    ) -> Result<GuildApplicationCommandPermissions> {
        self.request_json::<GuildApplicationCommandPermissions, ()>(
            command_permissions_route(
                Method::GET,
                application_id,
                guild_id,
                command_or_application_id,
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Replaces a command's permission overwrites using the required OAuth2 Bearer token.
    pub async fn edit_application_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_or_application_id: Snowflake,
        edit: &EditApplicationCommandPermissions,
        bearer_token: &str,
    ) -> Result<GuildApplicationCommandPermissions> {
        let mut headers = HeaderMap::new();
        let authorization = HeaderValue::from_str(&format!("Bearer {bearer_token}"))
            .map_err(|_| Error::InvalidToken)?;
        headers.insert(AUTHORIZATION, authorization);

        self.request_json_with_headers(
            command_permissions_route(
                Method::PUT,
                application_id,
                guild_id,
                command_or_application_id,
                RetrySafety::Safe,
            ),
            Some(edit),
            headers,
        )
        .await
    }
}

fn localization_query(with_localizations: bool) -> &'static str {
    if with_localizations {
        "?with_localizations=true"
    } else {
        ""
    }
}

fn global_commands_route(
    method: Method,
    application_id: ApplicationId,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/applications/{application_id}/commands{suffix}"),
        "/applications/{application_id}/commands",
        Some(application_id.to_string()),
        safety,
    )
}

fn global_command_route(
    method: Method,
    application_id: ApplicationId,
    command_id: CommandId,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/applications/{application_id}/commands/{command_id}"),
        "/applications/{application_id}/commands/{command_id}",
        Some(application_id.to_string()),
        safety,
    )
}

fn guild_commands_route(
    method: Method,
    application_id: ApplicationId,
    guild_id: GuildId,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/applications/{application_id}/guilds/{guild_id}/commands{suffix}"),
        "/applications/{application_id}/guilds/{guild_id}/commands",
        Some(guild_id.to_string()),
        safety,
    )
}

fn guild_command_route(
    method: Method,
    application_id: ApplicationId,
    guild_id: GuildId,
    command_id: CommandId,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/applications/{application_id}/guilds/{guild_id}/commands/{command_id}"),
        "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}",
        Some(guild_id.to_string()),
        safety,
    )
}

fn command_permissions_collection_route(
    method: Method,
    application_id: ApplicationId,
    guild_id: GuildId,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!("/applications/{application_id}/guilds/{guild_id}/commands/permissions"),
        "/applications/{application_id}/guilds/{guild_id}/commands/permissions",
        Some(guild_id.to_string()),
        safety,
    )
}

fn command_permissions_route(
    method: Method,
    application_id: ApplicationId,
    guild_id: GuildId,
    command_or_application_id: Snowflake,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!(
            "/applications/{application_id}/guilds/{guild_id}/commands/{command_or_application_id}/permissions"
        ),
        "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions",
        Some(guild_id.to_string()),
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::model::{ApplicationCommandType, Permissions};

    use super::{CreateApplicationCommand, EditApplicationCommand};

    #[test]
    fn chat_input_constructor_serializes_required_fields_only() {
        let command = CreateApplicationCommand::chat_input("search", "Search messages");

        assert_eq!(
            serde_json::to_value(command).expect("command"),
            serde_json::json!({
                "name":"search",
                "description":"Search messages",
                "type":ApplicationCommandType::CHAT_INPUT.0
            })
        );
    }

    #[test]
    fn command_edits_preserve_explicit_nulls() {
        let edit = EditApplicationCommand {
            name_localizations: Some(None),
            default_member_permissions: Some(None),
            dm_permission: Some(None),
            ..EditApplicationCommand::default()
        };
        let value = serde_json::to_value(edit).expect("edit command");

        assert!(value["name_localizations"].is_null());
        assert!(value["default_member_permissions"].is_null());
        assert!(value["dm_permission"].is_null());
        assert!(value.get("name").is_none());
    }

    #[test]
    fn command_permissions_serialize_permission_strings() {
        let command = CreateApplicationCommand {
            default_member_permissions: Some(Some(Permissions::MANAGE_MESSAGES)),
            ..CreateApplicationCommand::named("Inspect", ApplicationCommandType::MESSAGE)
        };

        assert_eq!(
            serde_json::to_value(command).expect("command")["default_member_permissions"],
            "8192"
        );
    }
}
