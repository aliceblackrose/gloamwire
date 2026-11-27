use zeroize::Zeroize;

/// Internal owned credential that redacts Debug and clears its allocation on drop.
#[derive(Clone)]
pub(crate) struct SecretString(String);

impl SecretString {
    pub(crate) const fn new(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::SecretString;

    #[test]
    fn debug_does_not_expose_credential() {
        let credential = SecretString::new("secret-sentinel".into());
        assert_eq!(format!("{credential:?}"), "[REDACTED]");
        assert_eq!(credential.expose(), "secret-sentinel");
    }
}

/// Require TLS for credential-bearing WebSockets. Plaintext is restricted to
/// explicitly enabled numeric loopback endpoints (no DNS resolution).
pub(crate) fn secure_websocket_endpoint(endpoint: &str, allow_loopback: bool) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else { return false; };
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() || url.host_str().is_none() {
        return false;
    }
    if url.scheme() == "wss" {
        return true;
    }
    url.scheme() == "ws" && allow_loopback
        && url.host_str().is_some_and(|host| host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback()))
}

#[cfg(test)]
mod endpoint_tests {
    use super::secure_websocket_endpoint;

    #[test]
    fn plaintext_requires_explicit_numeric_loopback() {
        assert!(secure_websocket_endpoint("wss://gateway.discord.gg", false));
        assert!(!secure_websocket_endpoint("ws://127.0.0.1:80", false));
        assert!(secure_websocket_endpoint("ws://127.0.0.1:80", true));
        assert!(secure_websocket_endpoint("ws://[::1]:80", true));
        assert!(!secure_websocket_endpoint("ws://example.com", true));
        assert!(!secure_websocket_endpoint("wss://secret@example.com", true));
    }
}
