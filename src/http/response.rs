use reqwest::{StatusCode, header::HeaderMap};
use serde::de::DeserializeOwned;

use crate::Result;

/// A successful Discord HTTP response with transport metadata preserved.
///
/// Endpoint methods can return the decoded body directly for convenience while
/// retaining this type internally when response status or headers are relevant.
#[derive(Debug, Clone)]
pub struct HttpResponse<T> {
    status: StatusCode,
    headers: HeaderMap,
    body: T,
}

impl<T> HttpResponse<T> {
    pub(crate) fn new(status: StatusCode, headers: HeaderMap, body: T) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Returns the successful HTTP status code returned by Discord.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the HTTP response headers returned by Discord.
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns a shared reference to the decoded response body.
    #[must_use]
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Consumes the response and returns its decoded body.
    #[must_use]
    pub fn into_body(self) -> T {
        self.body
    }

    /// Maps the response body while preserving status and headers.
    #[must_use]
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> HttpResponse<U> {
        HttpResponse {
            status: self.status,
            headers: self.headers,
            body: map(self.body),
        }
    }
}

impl HttpResponse<Vec<u8>> {
    /// Decodes a raw successful response body as JSON while preserving metadata.
    pub fn into_json<T>(self) -> Result<HttpResponse<T>>
    where
        T: DeserializeOwned,
    {
        let body = serde_json::from_slice(&self.body)?;
        Ok(HttpResponse {
            status: self.status,
            headers: self.headers,
            body,
        })
    }

    /// Decodes a JSON body or returns `None` for a successful empty response.
    pub fn into_optional_json<T>(self) -> Result<HttpResponse<Option<T>>>
    where
        T: DeserializeOwned,
    {
        if self.body.is_empty() {
            return Ok(self.map(|_| None));
        }

        self.into_json::<T>().map(|response| response.map(Some))
    }

    /// Discards a successful response body while preserving status and headers.
    #[must_use]
    pub fn into_empty(self) -> HttpResponse<()> {
        self.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use reqwest::{StatusCode, header::HeaderValue};
    use serde::Deserialize;

    use super::HttpResponse;

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Payload {
        ok: bool,
    }

    #[test]
    fn json_decoding_preserves_response_metadata() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-test", HeaderValue::from_static("value"));
        let response = HttpResponse::new(StatusCode::OK, headers, br#"{"ok":true}"#.to_vec())
            .into_json::<Payload>()
            .expect("json response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-test"], "value");
        assert_eq!(response.body(), &Payload { ok: true });
    }

    #[test]
    fn empty_response_preserves_status_and_headers() {
        let response = HttpResponse::new(
            StatusCode::NO_CONTENT,
            reqwest::header::HeaderMap::new(),
            Vec::new(),
        )
        .into_empty();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.body(), &());
    }

    #[test]
    fn optional_json_accepts_no_content() {
        let response = HttpResponse::new(
            StatusCode::NO_CONTENT,
            reqwest::header::HeaderMap::new(),
            Vec::new(),
        )
        .into_optional_json::<Payload>()
        .expect("optional response");

        assert_eq!(response.into_body(), None);
    }

    #[test]
    fn raw_binary_body_is_not_forced_through_json() {
        let bytes = vec![0, 159, 146, 150, 255];
        let response = HttpResponse::new(
            StatusCode::OK,
            reqwest::header::HeaderMap::new(),
            bytes.clone(),
        );

        assert_eq!(response.into_body(), bytes);
    }
}
