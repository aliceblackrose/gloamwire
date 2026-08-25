use std::time::Duration;

use reqwest::{
    Method,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    error::{Error, Result},
    model::{CreateMessage, Message, Snowflake, User},
};

use super::GatewayBot;

const API_BASE_URL: &str = "https://discord.com/api/v10";
const USER_AGENT: &str = "Gloamwire/0.1 (+https://github.com/cybellereaper/Gloamwire)";
const MAX_RATE_LIMIT_RETRIES: usize = 3;

/// An asynchronous Discord REST API client.
#[derive(Debug, Clone)]
pub struct RestClient {
    http: reqwest::Client,
    base_url: String,
}

impl RestClient {
    /// Creates a client using a raw Discord bot token.
    pub fn new(token: impl AsRef<str>) -> Result<Self> {
        let authorization = HeaderValue::from_str(&format!("Bot {}", token.as_ref()))
            .map_err(|_| Error::InvalidToken)?;

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self {
            http,
            base_url: API_BASE_URL.to_owned(),
        })
    }

    /// Overrides the API base URL.
    ///
    /// This is primarily useful for integration tests and compatible proxies.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Returns the current bot user.
    pub async fn get_current_user(&self) -> Result<User> {
        self.request_json::<User, ()>(Method::GET, "/users/@me", None)
            .await
    }

    /// Returns Discord Gateway connection and sharding information for the bot.
    pub async fn get_gateway_bot(&self) -> Result<GatewayBot> {
        self.request_json::<GatewayBot, ()>(Method::GET, "/gateway/bot", None)
            .await
    }

    /// Creates a message in a channel.
    pub async fn create_message(
        &self,
        channel_id: Snowflake,
        message: &CreateMessage,
    ) -> Result<Message> {
        let route = format!("/channels/{channel_id}/messages");
        self.request_json(Method::POST, &route, Some(message)).await
    }

    async fn request_json<T, B>(
        &self,
        method: Method,
        route: &str,
        body: Option<&B>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url, route);

        for attempt in 0..=MAX_RATE_LIMIT_RETRIES {
            let mut request = self.http.request(method.clone(), &url);
            if let Some(body) = body {
                request = request.json(body);
            }

            let response = request.send().await?;
            let status = response.status();
            let bytes = response.bytes().await?;

            if status.is_success() {
                return Ok(serde_json::from_slice(&bytes)?);
            }

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                && attempt < MAX_RATE_LIMIT_RETRIES
            {
                let retry_after = serde_json::from_slice::<RateLimitResponse>(&bytes)
                    .ok()
                    .map(|body| body.retry_after)
                    .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
                    .unwrap_or(1.0);

                tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;
                continue;
            }

            let api_error = serde_json::from_slice::<ApiErrorResponse>(&bytes).ok();
            let message = api_error
                .as_ref()
                .map(|error| error.message.clone())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());

            return Err(Error::HttpStatus {
                status,
                code: api_error.map(|error| error.code),
                message,
            });
        }

        unreachable!("rate-limit retry loop always returns or continues")
    }
}

#[derive(Debug, serde::Deserialize)]
struct ApiErrorResponse {
    code: i64,
    message: String,
}

#[derive(Debug, serde::Deserialize)]
struct RateLimitResponse {
    retry_after: f64,
}
