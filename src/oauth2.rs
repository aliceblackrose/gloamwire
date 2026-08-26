//! Discord OAuth2 authorization and token flows.

use std::{collections::BTreeMap, fmt};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    error::{Error, OAuth2ApiError, Result},
    http::encoding::QueryBuilder,
    model::{ApplicationId, ApplicationIntegrationType, GuildId, Permissions, User},
};

const API_BASE_URL: &str = "https://discord.com/api/v10";
const AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize";
const USER_AGENT: &str = "Gloamwire/0.1 (+https://github.com/cybellereaper/Gloamwire)";

/// OAuth2 response type used by Discord's authorization endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OAuth2ResponseType {
    /// Authorization code grant.
    Code,
    /// Implicit access-token grant.
    Token,
}

impl OAuth2ResponseType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Token => "token",
        }
    }
}

/// Prompt behavior for an OAuth2 authorization request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OAuth2Prompt {
    Consent,
    None,
}

impl OAuth2Prompt {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Consent => "consent",
            Self::None => "none",
        }
    }
}

/// Token type hint accepted by Discord's revocation endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OAuth2TokenTypeHint {
    AccessToken,
    RefreshToken,
}

impl OAuth2TokenTypeHint {
    const fn as_str(self) -> &'static str {
        match self {
            Self::AccessToken => "access_token",
            Self::RefreshToken => "refresh_token",
        }
    }
}

/// Builder for Discord OAuth2 authorization URLs.
///
/// The builder does not generate or persist `state`; callers should generate an
/// unpredictable value, include it with [`Self::state`], and verify the returned
/// value before exchanging an authorization code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuth2AuthorizationUrl {
    client_id: ApplicationId,
    response_type: Option<OAuth2ResponseType>,
    scopes: Vec<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    prompt: Option<OAuth2Prompt>,
    integration_type: Option<ApplicationIntegrationType>,
    permissions: Option<Permissions>,
    guild_id: Option<GuildId>,
    disable_guild_select: Option<bool>,
}

impl OAuth2AuthorizationUrl {
    /// Starts an authorization URL for an application.
    #[must_use]
    pub const fn new(client_id: ApplicationId) -> Self {
        Self {
            client_id,
            response_type: None,
            scopes: Vec::new(),
            redirect_uri: None,
            state: None,
            prompt: None,
            integration_type: None,
            permissions: None,
            guild_id: None,
            disable_guild_select: None,
        }
    }

    /// Sets the OAuth2 response type.
    #[must_use]
    pub const fn response_type(mut self, response_type: OAuth2ResponseType) -> Self {
        self.response_type = Some(response_type);
        self
    }

    /// Adds one OAuth2 scope.
    #[must_use]
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Replaces the OAuth2 scopes.
    #[must_use]
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the registered redirect URI.
    #[must_use]
    pub fn redirect_uri(mut self, redirect_uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(redirect_uri.into());
        self
    }

    /// Sets the caller-generated OAuth2 state value.
    #[must_use]
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets authorization prompt behavior.
    #[must_use]
    pub const fn prompt(mut self, prompt: OAuth2Prompt) -> Self {
        self.prompt = Some(prompt);
        self
    }

    /// Sets the application installation context.
    #[must_use]
    pub const fn integration_type(mut self, integration_type: ApplicationIntegrationType) -> Self {
        self.integration_type = Some(integration_type);
        self
    }

    /// Sets the bot permissions requested by an installation flow.
    #[must_use]
    pub const fn permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Preselects a guild in a bot authorization flow.
    #[must_use]
    pub const fn guild_id(mut self, guild_id: GuildId) -> Self {
        self.guild_id = Some(guild_id);
        self
    }

    /// Controls whether the preselected guild may be changed.
    #[must_use]
    pub const fn disable_guild_select(mut self, disabled: bool) -> Self {
        self.disable_guild_select = Some(disabled);
        self
    }

    /// Builds the Discord authorization URL.
    #[must_use]
    pub fn build(&self) -> String {
        let mut query = QueryBuilder::default();
        query.push("client_id", self.client_id);

        if let Some(response_type) = self.response_type {
            query.push_str("response_type", response_type.as_str());
        }
        if !self.scopes.is_empty() {
            query.push_str("scope", &self.scopes.join(" "));
        }
        if let Some(redirect_uri) = &self.redirect_uri {
            query.push_str("redirect_uri", redirect_uri);
        }
        if let Some(state) = &self.state {
            query.push_str("state", state);
        }
        if let Some(prompt) = self.prompt {
            query.push_str("prompt", prompt.as_str());
        }
        if let Some(integration_type) = self.integration_type {
            query.push("integration_type", integration_type.0);
        }
        if let Some(permissions) = self.permissions {
            query.push("permissions", permissions.bits());
        }
        if let Some(guild_id) = self.guild_id {
            query.push("guild_id", guild_id);
        }
        if let Some(disabled) = self.disable_guild_select {
            query.push("disable_guild_select", disabled);
        }

        format!("{AUTHORIZE_URL}{}", query.finish())
    }
}

/// Access-token response returned by Discord OAuth2 token exchanges.
#[derive(Clone, PartialEq, Deserialize)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub scope: String,
    /// Additional flow-specific response fields such as `guild` or `webhook`.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl OAuth2TokenResponse {
    /// Returns the granted scopes in Discord's space-delimited order.
    pub fn scopes(&self) -> impl Iterator<Item = &str> {
        self.scope.split_whitespace()
    }
}

impl fmt::Debug for OAuth2TokenResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuth2TokenResponse")
            .field("access_token", &"<redacted>")
            .field("token_type", &self.token_type)
            .field("expires_in", &self.expires_in)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("scope", &self.scope)
            .field("extra", &self.extra)
            .finish()
    }
}

/// Partial application object returned by Discord's authorization-info endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OAuth2Application {
    pub id: ApplicationId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub bot_public: Option<bool>,
    #[serde(default)]
    pub bot_require_code_grant: Option<bool>,
    #[serde(default)]
    pub verify_key: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Information about the authorization represented by a Bearer access token.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OAuth2AuthorizationInfo {
    pub application: OAuth2Application,
    pub scopes: Vec<String>,
    pub expires: String,
    #[serde(default)]
    pub user: Option<User>,
}

/// Discord OAuth2 client using confidential-client Basic authentication.
#[derive(Clone)]
pub struct OAuth2Client {
    http: reqwest::Client,
    client_id: ApplicationId,
    client_secret: String,
    api_base_url: String,
}

impl fmt::Debug for OAuth2Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuth2Client")
            .field("client_id", &self.client_id)
            .field("api_base_url", &self.api_base_url)
            .finish_non_exhaustive()
    }
}

impl OAuth2Client {
    /// Creates an OAuth2 client for a Discord application.
    pub fn new(client_id: ApplicationId, client_secret: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder().user_agent(USER_AGENT).build()?,
            client_id,
            client_secret: client_secret.into(),
            api_base_url: API_BASE_URL.to_owned(),
        })
    }

    /// Overrides the Discord API base URL, primarily for integration tests.
    #[must_use]
    pub fn with_api_base_url(mut self, api_base_url: impl Into<String>) -> Self {
        self.api_base_url = api_base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Exchanges an authorization code for an access token.
    pub async fn exchange_code(
        &self,
        code: impl AsRef<str>,
        redirect_uri: impl AsRef<str>,
    ) -> Result<OAuth2TokenResponse> {
        self.post_token_form(&AuthorizationCodeRequest {
            grant_type: "authorization_code",
            code: code.as_ref(),
            redirect_uri: redirect_uri.as_ref(),
        })
        .await
    }

    /// Exchanges a refresh token for a fresh access token.
    pub async fn refresh_token(
        &self,
        refresh_token: impl AsRef<str>,
    ) -> Result<OAuth2TokenResponse> {
        self.post_token_form(&RefreshTokenRequest {
            grant_type: "refresh_token",
            refresh_token: refresh_token.as_ref(),
        })
        .await
    }

    /// Requests a client-credentials access token for the supplied scopes.
    pub async fn client_credentials<I, S>(&self, scopes: I) -> Result<OAuth2TokenResponse>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let scope = scopes
            .into_iter()
            .map(|scope| scope.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(" ");

        self.post_token_form(&ClientCredentialsRequest {
            grant_type: "client_credentials",
            scope: &scope,
        })
        .await
    }

    /// Revokes an access or refresh token.
    pub async fn revoke_token(
        &self,
        token: impl AsRef<str>,
        token_type_hint: Option<OAuth2TokenTypeHint>,
    ) -> Result<()> {
        let response = self
            .http
            .post(format!("{}/oauth2/token/revoke", self.api_base_url))
            .basic_auth(self.client_id, Some(&self.client_secret))
            .form(&RevokeTokenRequest {
                token: token.as_ref(),
                token_type_hint: token_type_hint.map(OAuth2TokenTypeHint::as_str),
            })
            .send()
            .await?;

        self.expect_empty_success(response).await
    }

    /// Returns information about a Bearer authorization.
    pub async fn get_current_authorization(
        &self,
        access_token: impl AsRef<str>,
    ) -> Result<OAuth2AuthorizationInfo> {
        let response = self
            .http
            .get(format!("{}/oauth2/@me", self.api_base_url))
            .bearer_auth(access_token.as_ref())
            .send()
            .await?;

        self.decode_json(response).await
    }

    async fn post_token_form<F>(&self, form: &F) -> Result<OAuth2TokenResponse>
    where
        F: Serialize + ?Sized,
    {
        let response = self
            .http
            .post(format!("{}/oauth2/token", self.api_base_url))
            .basic_auth(self.client_id, Some(&self.client_secret))
            .form(form)
            .send()
            .await?;

        self.decode_json(response).await
    }

    async fn decode_json<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let status = response.status();
        let bytes = response.bytes().await?;

        if status.is_success() {
            return Ok(serde_json::from_slice(&bytes)?);
        }

        Err(oauth2_response_error(status, &bytes))
    }

    async fn expect_empty_success(&self, response: reqwest::Response) -> Result<()> {
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        let bytes = response.bytes().await?;
        Err(oauth2_response_error(status, &bytes))
    }
}

#[derive(Serialize)]
struct AuthorizationCodeRequest<'a> {
    grant_type: &'static str,
    code: &'a str,
    redirect_uri: &'a str,
}

#[derive(Serialize)]
struct RefreshTokenRequest<'a> {
    grant_type: &'static str,
    refresh_token: &'a str,
}

#[derive(Serialize)]
struct ClientCredentialsRequest<'a> {
    grant_type: &'static str,
    scope: &'a str,
}

#[derive(Serialize)]
struct RevokeTokenRequest<'a> {
    token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type_hint: Option<&'static str>,
}

fn oauth2_response_error(status: StatusCode, bytes: &[u8]) -> Error {
    if let Ok(error) = serde_json::from_slice::<OAuth2ApiError>(bytes) {
        return Error::OAuth2 { status, error };
    }

    Error::HttpStatus {
        status,
        code: None,
        message: String::from_utf8_lossy(bytes).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{ApplicationId, ApplicationIntegrationType, GuildId, Permissions};

    use super::{
        OAuth2AuthorizationUrl, OAuth2Client, OAuth2Prompt, OAuth2ResponseType, OAuth2TokenResponse,
    };

    #[test]
    fn builds_authorization_code_url_with_encoded_state_and_redirect() {
        let url = OAuth2AuthorizationUrl::new(ApplicationId::new(123))
            .response_type(OAuth2ResponseType::Code)
            .scopes(["identify", "guilds.join"])
            .redirect_uri("https://example.com/callback?source=test")
            .state("random state")
            .prompt(OAuth2Prompt::Consent)
            .integration_type(ApplicationIntegrationType::GUILD_INSTALL)
            .permissions(Permissions::SEND_MESSAGES)
            .guild_id(GuildId::new(456))
            .disable_guild_select(true)
            .build();

        assert!(url.starts_with("https://discord.com/oauth2/authorize?client_id=123"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=identify%20guilds.join"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fexample.com%2Fcallback%3Fsource%3Dtest"));
        assert!(url.contains("state=random%20state"));
        assert!(url.contains("integration_type=0"));
        assert!(url.contains("guild_id=456"));
    }

    #[test]
    fn token_debug_redacts_credentials() {
        let token: OAuth2TokenResponse = serde_json::from_str(
            r#"{
                "access_token":"access-secret",
                "token_type":"Bearer",
                "expires_in":604800,
                "refresh_token":"refresh-secret",
                "scope":"identify connections"
            }"#,
        )
        .expect("token response");

        let debug = format!("{token:?}");
        assert!(!debug.contains("access-secret"));
        assert!(!debug.contains("refresh-secret"));
        assert_eq!(
            token.scopes().collect::<Vec<_>>(),
            ["identify", "connections"]
        );
    }

    #[test]
    fn client_debug_redacts_secret() {
        let client = OAuth2Client::new(ApplicationId::new(1), "client-secret").expect("client");
        assert!(!format!("{client:?}").contains("client-secret"));
    }
}
