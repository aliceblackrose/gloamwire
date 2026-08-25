use std::{sync::Arc, time::Duration};

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    error::{DiscordApiError, Error, Result},
    model::{ChannelId, CreateMessage, Message, User},
};

use super::{GatewayBot, HttpResponse, rate_limit::RateLimiter, route::Route};

const API_BASE_URL: &str = "https://discord.com/api/v10";
const USER_AGENT: &str = "Gloamwire/0.1 (+https://github.com/cybellereaper/Gloamwire)";
const MAX_RATE_LIMIT_RETRIES: usize = 3;

/// An asynchronous Discord REST API client.
#[derive(Clone)]
pub struct RestClient {
    http: reqwest::Client,
    base_url: String,
    rate_limiter: Arc<RateLimiter>,
}

impl std::fmt::Debug for RestClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RestClient")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
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
            rate_limiter: Arc::new(RateLimiter::default()),
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
        self.request_json::<User, ()>(Route::current_user(), None)
            .await
    }

    /// Returns Discord Gateway connection and sharding information for the bot.
    pub async fn get_gateway_bot(&self) -> Result<GatewayBot> {
        self.request_json::<GatewayBot, ()>(Route::gateway_bot(), None)
            .await
    }

    /// Creates a message in a channel.
    pub async fn create_message(
        &self,
        channel_id: ChannelId,
        message: &CreateMessage,
    ) -> Result<Message> {
        self.request_json(Route::create_message(channel_id), Some(message))
            .await
    }

    async fn request_json<T, B>(&self, route: Route, body: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        Ok(self
            .request_raw(route, body)
            .await?
            .into_json::<T>()?
            .into_body())
    }

    async fn request_raw<B>(&self, route: Route, body: Option<&B>) -> Result<HttpResponse<Vec<u8>>>
    where
        B: Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url, route.path);

        for attempt in 0..=MAX_RATE_LIMIT_RETRIES {
            self.rate_limiter.acquire(&route).await;

            let mut request = self.http.request(route.method.clone(), &url);
            if let Some(body) = body {
                request = request.json(body);
            }

            let response = request.send().await?;
            let status = response.status();
            let headers = response.headers().clone();
            let bytes = response.bytes().await?.to_vec();
            let rate_limit = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                serde_json::from_slice::<RateLimitResponse>(&bytes).ok()
            } else {
                None
            };
            let retry_after = rate_limit
                .as_ref()
                .and_then(|body| duration_from_seconds(body.retry_after))
                .or_else(|| retry_after_header(&headers));
            let global = rate_limit.as_ref().is_some_and(|body| body.global)
                || headers
                    .get("x-ratelimit-scope")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|scope| scope == "global");

            self.rate_limiter
                .update(&route, status, &headers, retry_after, global)
                .await;

            if status.is_success() {
                return Ok(HttpResponse::new(status, headers, bytes));
            }

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < MAX_RATE_LIMIT_RETRIES
            {
                continue;
            }

            if let Ok(api_error) = serde_json::from_slice::<ApiErrorResponse>(&bytes) {
                if let Some(raw_errors) = api_error.errors {
                    return Err(Error::DiscordApi {
                        status,
                        error: DiscordApiError::new(api_error.code, api_error.message, raw_errors),
                    });
                }

                return Err(Error::HttpStatus {
                    status,
                    code: Some(api_error.code),
                    message: api_error.message,
                });
            }

            return Err(Error::HttpStatus {
                status,
                code: None,
                message: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }

        unreachable!("rate-limit retry loop always returns or continues")
    }
}

fn retry_after_header(headers: &HeaderMap) -> Option<Duration> {
    let seconds = headers
        .get("retry-after")?
        .to_str()
        .ok()?
        .parse::<f64>()
        .ok()?;
    duration_from_seconds(seconds)
}

fn duration_from_seconds(seconds: f64) -> Option<Duration> {
    (seconds.is_finite() && seconds >= 0.0).then(|| Duration::from_secs_f64(seconds))
}

#[derive(Debug, serde::Deserialize)]
struct ApiErrorResponse {
    code: i64,
    message: String,
    #[serde(default)]
    errors: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct RateLimitResponse {
    retry_after: f64,
    #[serde(default)]
    global: bool,
}
