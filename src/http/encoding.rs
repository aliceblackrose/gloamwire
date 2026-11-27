use std::fmt::Display;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

const AUDIT_LOG_REASON: HeaderName = HeaderName::from_static("x-audit-log-reason");

/// Percent-encodes a UTF-8 value using RFC 3986 unreserved characters.
pub(crate) fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }

    encoded
}

/// Reject path segments that URL parsers would normalize or collapse.
pub(crate) fn valid_route_path(path: &str) -> bool {
    let path = path.split('?').next().unwrap_or_default();
    path.strip_prefix('/').is_some_and(|path| {
        path.split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
    })
}

pub(crate) fn audit_reason_headers(reason: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(reason) = reason {
        let encoded = percent_encode(reason);
        if let Ok(value) = HeaderValue::from_str(&encoded) {
            headers.insert(AUDIT_LOG_REASON, value);
        }
    }
    headers
}

#[derive(Debug, Default)]
pub(crate) struct QueryBuilder {
    query: String,
}

impl QueryBuilder {
    pub(crate) fn push(&mut self, name: &str, value: impl Display) {
        self.push_str(name, &value.to_string());
    }

    pub(crate) fn push_str(&mut self, name: &str, value: &str) {
        self.query
            .push(if self.query.is_empty() { '?' } else { '&' });
        self.query.push_str(&percent_encode(name));
        self.query.push('=');
        self.query.push_str(&percent_encode(value));
    }

    pub(crate) fn finish(self) -> String {
        self.query
    }
}

#[cfg(test)]
mod tests {
    use super::{QueryBuilder, audit_reason_headers, percent_encode};

    #[test]
    fn percent_encodes_unicode_and_reserved_characters() {
        assert_eq!(percent_encode("🔥 custom:1"), "%F0%9F%94%A5%20custom%3A1");
    }

    #[test]
    fn query_builder_encodes_names_and_values() {
        let mut query = QueryBuilder::default();
        query.push("limit", 50);
        query.push_str("author_id", "123");
        assert_eq!(query.finish(), "?limit=50&author_id=123");
    }

    #[test]
    fn audit_log_reasons_are_header_safe() {
        let headers = audit_reason_headers(Some("cleanup / spam"));

        assert_eq!(headers["x-audit-log-reason"], "cleanup%20%2F%20spam");
    }
}

#[cfg(test)]
mod path_security_tests {
    #[test]
    fn rejects_dot_and_empty_segments() {
        use super::valid_route_path;
        assert!(valid_route_path("/invites/code?with_counts=true"));
        assert!(!valid_route_path("/invites/.."));
        assert!(!valid_route_path("/webhooks/1/./messages/2"));
        assert!(!valid_route_path("/webhooks/1//messages/2"));
    }
}
