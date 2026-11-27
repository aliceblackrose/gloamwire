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
const USER_AGENT: &str = concat!(
    "Gloamwire/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/aliceblackrose/gloamwire)"
);

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
/// Use [`Self::begin_code_flow`] for authorization codes. Store the returned
/// one-time pending authorization in the initiating browser session.
#[derive(Clone, PartialEq, Eq)]
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

impl fmt::Debug for OAuth2AuthorizationUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuth2AuthorizationUrl")
            .field("client_id", &self.client_id)
            .field("response_type", &self.response_type)
            .field("state_set", &self.state.is_some())
            .finish_non_exhaustive()
    }
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

    /// Creates an authorization-code URL and a one-time, ten-minute state guard.
    /// Store the guard server-side in the initiating browser session, never in a
    /// global shared slot. State is generated by the operating system CSPRNG.
    pub fn begin_code_flow(mut self) -> Result<(String, OAuth2PendingAuthorization)> {
        use std::fmt::Write as _;

        let redirect_uri = self
            .redirect_uri
            .clone()
            .filter(|uri| !uri.is_empty())
            .ok_or_else(|| {
                Error::InvalidRestRequest("OAuth2 code flow requires a redirect URI".into())
            })?;
        let mut bytes = zeroize::Zeroizing::new([0_u8; 32]);
        getrandom::fill(&mut bytes[..]).map_err(|_| {
            Error::InvalidRestRequest("OAuth2 state generation failed".into())
        })?;
        let mut state = String::with_capacity(64);
        for byte in bytes.iter() {
            write!(&mut state, "{byte:02x}").expect("writing to String cannot fail");
        }
        self.response_type = Some(OAuth2ResponseType::Code);
        self.state = Some(state.clone());
        let url = self.build();
        Ok((
            url,
            OAuth2PendingAuthorization {
                client_id: self.client_id,
                redirect_uri,
                state: crate::SecretString::new(state),
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(600),
            },
        ))
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

/// A non-cloneable, one-time authorization-code CSRF guard. Keep it in the
/// initiating browser's server-side session and consume it on callback.
#[must_use]
pub struct OAuth2PendingAuthorization {
    client_id: ApplicationId,
    redirect_uri: String,
    state: crate::SecretString,
    expires_at: std::time::Instant,
}

impl fmt::Debug for OAuth2PendingAuthorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuth2PendingAuthorization").finish_non_exhaustive()
    }
}

impl OAuth2PendingAuthorization {
    fn validate(&self, client_id: ApplicationId, returned_state: &str) -> Result<()> {
        use subtle::ConstantTimeEq;
        if self.client_id != client_id
            || std::time::Instant::now() >= self.expires_at
            || !bool::from(self.state.expose().as_bytes().ct_eq(returned_state.as_bytes()))
        {
            return Err(Error::InvalidRestRequest(
                "invalid or expired OAuth2 state".into(),
            ));
        }
        Ok(())
    }
}

/// Access-token response returned by Discord OAuth2 token exchanges.
#[derive(Clone, PartialEq, Deserialize)]
pub struct OAuth2TokenResponse {
    pub access_token: crate::SecretString,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<crate::SecretString>,
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
            .field("extra_keys", &self.extra.keys().collect::<Vec<_>>())
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
    client_secret: crate::secret::SecretString,
    api_base_url: String,
    allow_insecure_loopback: bool,
}

impl fmt::Debug for OAuth2Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuth2Client")
            .field("client_id", &self.client_id)
            .field("api_base_url", &"[OMITTED]")
            .finish_non_exhaustive()
    }
}

impl OAuth2Client {
    /// Creates an OAuth2 client for a Discord application.
    pub fn new(client_id: ApplicationId, client_secret: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()?,
            client_id,
            client_secret: crate::secret::SecretString::new(client_secret.into()),
            api_base_url: API_BASE_URL.to_owned(),
            allow_insecure_loopback: false,
        })
    }

    /// Overrides the Discord API base URL with a trusted HTTPS endpoint.
    #[must_use]
    pub fn with_api_base_url(mut self, api_base_url: impl Into<String>) -> Self {
        self.api_base_url = api_base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Enables plaintext HTTP only for numeric loopback fixtures.
    /// Never use production credentials with this option.
    #[must_use]
    pub const fn with_insecure_loopback(mut self, allow: bool) -> Self {
        self.allow_insecure_loopback = allow;
        self
    }

    fn validate_endpoint(&self) -> Result<()> {
        if crate::secret::secure_http_endpoint(&self.api_base_url, self.allow_insecure_loopback) {
            Ok(())
        } else {
            Err(Error::InvalidRestRequest(
                "insecure or invalid OAuth2 base URL".into(),
            ))
        }
    }

    /// Consumes the initiating session's state guard before exchanging a code.
    /// A mismatch, expiration, or different application fails before any request.
    pub async fn exchange_code(
        &self,
        pending: OAuth2PendingAuthorization,
        code: impl AsRef<str>,
        returned_state: impl AsRef<str>,
    ) -> Result<OAuth2TokenResponse> {
        pending.validate(self.client_id, returned_state.as_ref())?;
        self.post_token_form(&AuthorizationCodeRequest {
            grant_type: "authorization_code",
            code: code.as_ref(),
            redirect_uri: &pending.redirect_uri,
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
        self.validate_endpoint()?;
        let response = self
            .http
            .post(format!("{}/oauth2/token/revoke", self.api_base_url))
            .basic_auth(self.client_id, Some(self.client_secret.expose()))
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
        self.validate_endpoint()?;
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
        self.validate_endpoint()?;
        let response = self
            .http
            .post(format!("{}/oauth2/token", self.api_base_url))
            .basic_auth(self.client_id, Some(self.client_secret.expose()))
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
        let bytes = crate::error::read_limited_body(response).await?;

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

        let bytes = crate::error::read_limited_body(response).await?;
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

#[cfg(test)]
mod authorization_debug_tests {
    use super::*;

    #[test]
    fn authorization_debug_omits_state_and_redirect() {
        let url = OAuth2AuthorizationUrl::new(ApplicationId::new(1))
            .state("state-sentinel")
            .redirect_uri("https://example.com/redirect-sentinel");
        let debug = format!("{url:?}");
        assert!(!debug.contains("sentinel"));
    }
}

#[cfg(test)]
mod code_flow_security_tests {
    use super::*;

    #[test]
    fn generated_state_is_unique_bound_and_expiring() {
        let builder = OAuth2AuthorizationUrl::new(ApplicationId::new(1))
            .redirect_uri("https://example.com/callback");
        let (url, mut pending) = builder.clone().begin_code_flow().unwrap();
        let (_, other) = builder.begin_code_flow().unwrap();
        assert_eq!(pending.state.expose().len(), 64);
        assert!(url.contains(pending.state.expose()));
        assert_ne!(pending.state, other.state);
        assert!(
            pending
                .validate(ApplicationId::new(1), pending.state.expose())
                .is_ok()
        );
        assert!(
            pending
                .validate(ApplicationId::new(2), pending.state.expose())
                .is_err()
        );
        assert!(pending.validate(ApplicationId::new(1), "wrong").is_err());
        pending.expires_at = std::time::Instant::now();
        assert!(
            pending
                .validate(ApplicationId::new(1), pending.state.expose())
                .is_err()
        );
    }
}
