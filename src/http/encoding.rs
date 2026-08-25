use std::fmt::Display;

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

#[derive(Debug, Default)]
pub(crate) struct QueryBuilder {
    query: String,
}

impl QueryBuilder {
    pub(crate) fn push(&mut self, name: &str, value: impl Display) {
        self.push_str(name, &value.to_string());
    }

    pub(crate) fn push_str(&mut self, name: &str, value: &str) {
        self.query.push(if self.query.is_empty() { '?' } else { '&' });
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
    use super::{QueryBuilder, percent_encode};

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
}
