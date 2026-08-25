use std::{sync::Arc, time::Duration};

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    error::{DiscordApiError, Error, Result},
    model::{ChannelId, CreateMessage, Message, User},
};

use super::{
    GatewayBot, HttpResponse,
    rate_limit::RateLimiter,
    route::Route,
    upload::{UploadFile, multipart_form},
};

const API_BASE_URL: &str = "https://discord.com/api/v10";
const USER_AGENT: &str = "Gloamwire/0.1 (+https://github.com/cybellereaper/Gloamwire)";
const MAX_RATE_LIMIT_RETRIES: usize = 3;
const DEFAULT_TRANSIENT_RETRIES: usize = 2;
const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Builder for an asynchronous Discord REST client.
#[derive(Clone)]
pub struct RestClientBuilder {
    authorization: HeaderValue,
    request_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    pool_idle_timeout: Option<Duration>,
    max_transient_retries: usize,
    retry_base_delay: Duration,
}

impl std::fmt::Debug for RestClientBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RestClientBuilder")
            .field("request_timeout", &self.request_timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("pool_idle_timeout", &self.pool_idle_timeout)
            .field("max_transient_retries", &self.max_transient_retries)
            .field("retry_base_delay", &self.retry_base_delay)
            .finish_non_exhaustive()
    }
}

impl RestClientBuilder {
    fn new(token: impl AsRef<str>) -> Result<Self> {
        let authorization = HeaderValue::from_str(&format!("Bot {}", token.as_ref()))
            .map_err(|_| Error::InvalidToken)?;

        Ok(Self {
            authorization,
            request_timeout: None,
            connect_timeout: None,
            pool_idle_timeout: None,
            max_transient_retries: DEFAULT_TRANSIENT_RETRIES,
            retry_base_delay: DEFAULT_RETRY_BASE_DELAY,
        })
    }

    /// Sets the total timeout for one HTTP request.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Sets the timeout for establishing a new HTTP connection.
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Sets how long idle pooled connections may remain available for reuse.
    #[must_use]
    pub fn pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_idle_timeout = Some(timeout);
        self
    }

    /// Sets the number of transient retries for explicitly retry-safe routes.
    ///
    /// Discord rate-limit retries are tracked separately and are not affected by
    /// this value. Non-idempotent routes are never retried for transport or 5xx
    /// failures regardless of this setting.
    #[must_use]
    pub fn max_transient_retries(mut self, retries: usize) -> Self {
        self.max_transient_retries = retries;
        self
    }

    /// Sets the initial delay used for exponential transient-retry backoff.
    #[must_use]
    pub fn retry_base_delay(mut self, delay: Duration) -> Self {
        self.retry_base_delay = delay;
        self
    }

    /// Builds the configured REST client.
    pub fn build(self) -> Result<RestClient> {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, self.authorization);

        let mut builder = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT);

        if let Some(timeout) = self.request_timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(timeout) = self.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }
        if let Some(timeout) = self.pool_idle_timeout {
            builder = builder.pool_idle_timeout(timeout);
        }

        Ok(RestClient {
            http: builder.build()?,
            base_url: API_BASE_URL.to_owned(),
            rate_limiter: Arc::new(RateLimiter::default()),
            max_transient_retries: self.max_transient_retries,
            retry_base_delay: self.retry_base_delay,
        })
    }
}

/// An asynchronous Discord REST API client.
#[derive(Clone)]
pub struct RestClient {
    http: reqwest::Client,
    base_url: String,
    rate_limiter: Arc<RateLimiter>,
    max_transient_retries: usize,
    retry_base_delay: Duration,
}

impl std::fmt::Debug for RestClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RestClient")
            .field("base_url", &self.base_url)
            .field("max_transient_retries", &self.max_transient_retries)
            .field("retry_base_delay", &self.retry_base_delay)
            .finish_non_exhaustive()
    }
}

impl RestClient {
    /// Creates a client using a raw Discord bot token.
    pub fn new(token: impl AsRef<str>) -> Result<Self> {
        Self::builder(token)?.build()
    }

    /// Creates a configurable REST client builder using a raw Discord bot token.
    pub fn builder(token: impl AsRef<str>) -> Result<RestClientBuilder> {
        RestClientBuilder::new(token)
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

    /// Creates a message in a channel without file uploads.
    pub async fn create_message(
        &self,
        channel_id: ChannelId,
        message: &CreateMessage,
    ) -> Result<Message> {
        self.request_json(Route::create_message(channel_id), Some(message))
            .await
    }

    pub(crate) async fn request_json<T, B>(&self, route: Route, body: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.request_json_with_headers(route, body, HeaderMap::new())
            .await
    }

    pub(crate) async fn request_json_with_headers<T, B>(
        &self,
        route: Route,
        body: Option<&B>,
        headers: HeaderMap,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let payload = match body {
            Some(body) => RequestPayload::Json(serde_json::to_vec(body)?),
            None => RequestPayload::Empty,
        };
        Ok(self
            .execute(route, payload, headers)
            .await?
            .into_json::<T>()?
            .into_body())
    }

    pub(crate) async fn request_empty<B>(
        &self,
        route: Route,
        body: Option<&B>,
        headers: HeaderMap,
    ) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let payload = match body {
            Some(body) => RequestPayload::Json(serde_json::to_vec(body)?),
            None => RequestPayload::Empty,
        };
        self.execute(route, payload, headers).await?;
        Ok(())
    }

    pub(crate) async fn request_multipart_json<T, B>(
        &self,
        route: Route,
        body: &B,
        files: &[UploadFile],
        headers: HeaderMap,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let payload = RequestPayload::Multipart {
            payload_json: serde_json::to_string(body)?,
            files,
        };
        Ok(self
            .execute(route, payload, headers)
            .await?
            .into_json::<T>()?
            .into_body())
    }

    async fn execute(
        &self,
        route: Route,
        payload: RequestPayload<'_>,
        headers: HeaderMap,
    ) -> Result<HttpResponse<Vec<u8>>> {
        let url = format!("{}{}", self.base_url, route.path);
        let mut rate_limit_retries = 0;
        let mut transient_retries = 0;

        loop {
            self.rate_limiter.acquire(&route).await;

            let mut request = self
                .http
                .request(route.method.clone(), &url)
                .headers(headers.clone());
            request = match &payload {
                RequestPayload::Empty => request,
                RequestPayload::Json(bytes) => request
                    .header(CONTENT_TYPE, "application/json")
                    .body(bytes.clone()),
                RequestPayload::Multipart {
                    payload_json,
                    files,
                } => request.multipart(multipart_form(payload_json.clone(), files).await?),
            };

            let response = match request.send().await {
                Ok(response) => response,
                Err(error)
                    if route.is_retry_safe()
                        && transient_retries < self.max_transient_retries
                        && is_retryable_transport_error(&error) =>
                {
                    let delay = retry_delay(self.retry_base_delay, transient_retries);
                    transient_retries += 1;
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(error) => return Err(error.into()),
            };

            let status = response.status();
            let response_headers = response.headers().clone();
            let bytes = response.bytes().await?.to_vec();
            let rate_limit = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                serde_json::from_slice::<RateLimitResponse>(&bytes).ok()
            } else {
                None
            };
            let retry_after = rate_limit
                .as_ref()
                .and_then(|body| duration_from_seconds(body.retry_after))
                .or_else(|| retry_after_header(&response_headers));
            let global = rate_limit.as_ref().is_some_and(|body| body.global)
                || response_headers
                    .get("x-ratelimit-scope")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|scope| scope == "global");

            self.rate_limiter
                .update(&route, status, &response_headers, retry_after, global)
                .await;

            if status.is_success() {
                return Ok(HttpResponse::new(status, response_headers, bytes));
            }

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                && rate_limit_retries < MAX_RATE_LIMIT_RETRIES
            {
                rate_limit_retries += 1;
                continue;
            }

            if route.is_retry_safe()
                && transient_retries < self.max_transient_retries
                && is_retryable_status(status)
            {
                let delay = retry_delay(self.retry_base_delay, transient_retries);
                transient_retries += 1;
                tokio::time::sleep(delay).await;
                continue;
            }

            return Err(response_error(status, &bytes));
        }
    }
}

enum RequestPayload<'a> {
    Empty,
    Json(Vec<u8>),
    Multipart {
        payload_json: String,
        files: &'a [UploadFile],
    },
}

fn is_retryable_transport_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout()
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::REQUEST_TIMEOUT
            | reqwest::StatusCode::TOO_EARLY
            | reqwest::StatusCode::INTERNAL_SERVER_ERROR
            | reqwest::StatusCode::BAD_GATEWAY
            | reqwest::StatusCode::SERVICE_UNAVAILABLE
            | reqwest::StatusCode::GATEWAY_TIMEOUT
    )
}

fn retry_delay(base: Duration, attempt: usize) -> Duration {
    let exponent = attempt.min(6) as i32;
    let multiplier = 2_f64.powi(exponent);
    let jitter = 0.5 + fastrand::f64();
    Duration::from_secs_f64(
        (base.as_secs_f64() * multiplier * jitter).min(MAX_RETRY_DELAY.as_secs_f64()),
    )
}

fn response_error(status: reqwest::StatusCode, bytes: &[u8]) -> Error {
    if let Ok(api_error) = serde_json::from_slice::<ApiErrorResponse>(bytes) {
        if let Some(raw_errors) = api_error.errors {
            return Error::DiscordApi {
                status,
                error: DiscordApiError::new(api_error.code, api_error.message, raw_errors),
            };
        }

        return Error::HttpStatus {
            status,
            code: Some(api_error.code),
            message: api_error.message,
        };
    }

    Error::HttpStatus {
        status,
        code: None,
        message: String::from_utf8_lossy(bytes).into_owned(),
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::StatusCode;

    use crate::Error;

    use super::{RestClient, is_retryable_status, response_error, retry_delay};

    #[test]
    fn nested_validation_errors_use_structured_variant() {
        let error = response_error(
            StatusCode::BAD_REQUEST,
            br#"{
                "code":50035,
                "message":"Invalid Form Body",
                "errors":{
                    "name":{
                        "_errors":[{
                            "code":"BASE_TYPE_REQUIRED",
                            "message":"This field is required"
                        }]
                    }
                }
            }"#,
        );

        let Error::DiscordApi { status, error } = error else {
            panic!("expected structured Discord API error");
        };
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, 50035);
        assert_eq!(error.validation_errors[0].dotted_path(), "name");
    }

    #[test]
    fn ordinary_discord_errors_keep_http_status_variant() {
        let error = response_error(
            StatusCode::NOT_FOUND,
            br#"{"code":10008,"message":"Unknown Message"}"#,
        );

        let Error::HttpStatus {
            status,
            code,
            message,
        } = error
        else {
            panic!("expected HTTP status error");
        };
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, Some(10008));
        assert_eq!(message, "Unknown Message");
    }

    #[test]
    fn non_json_errors_preserve_response_text() {
        let error = response_error(StatusCode::BAD_GATEWAY, b"upstream unavailable");

        let Error::HttpStatus { code, message, .. } = error else {
            panic!("expected HTTP status error");
        };
        assert_eq!(code, None);
        assert_eq!(message, "upstream unavailable");
    }

    #[test]
    fn retries_only_transient_statuses() {
        assert!(is_retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
    }

    #[test]
    fn retry_backoff_is_bounded() {
        let delay = retry_delay(Duration::from_secs(10), 10);
        assert!(delay <= super::MAX_RETRY_DELAY);
    }

    #[test]
    fn builder_debug_does_not_expose_token() {
        let builder = RestClient::builder("secret-token").expect("builder");
        assert!(!format!("{builder:?}").contains("secret-token"));
    }

    #[test]
    fn builder_accepts_transport_configuration() {
        RestClient::builder("token")
            .expect("builder")
            .request_timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .pool_idle_timeout(Duration::from_secs(30))
            .max_transient_retries(4)
            .retry_base_delay(Duration::from_millis(50))
            .build()
            .expect("client");
    }
}
