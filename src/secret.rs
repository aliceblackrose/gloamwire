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
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return false;
    }
    if url.scheme() == "wss" {
        return true;
    }
    url.scheme() == "ws"
        && allow_loopback
        && url.host_str().is_some_and(|host| {
            host.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        })
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

pub(crate) fn secure_http_endpoint(endpoint: &str, allow_loopback: bool) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return false;
    }
    url.scheme() == "https"
        || (url.scheme() == "http"
            && allow_loopback
            && url.host_str().is_some_and(|host| {
                host.trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            }))
}

pub(crate) fn discord_websocket_origin(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    url.scheme() == "wss"
        && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|host| {
            ["discord.gg", "discord.com", "discord.media"]
                .iter()
                .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        })
}

pub(crate) fn same_origin(first: &str, second: &str) -> bool {
    match (reqwest::Url::parse(first), reqwest::Url::parse(second)) {
        (Ok(first), Ok(second)) => first.origin() == second.origin(),
        _ => false,
    }
}

#[cfg(test)]
mod http_endpoint_tests {
    use super::{discord_websocket_origin, same_origin, secure_http_endpoint};

    #[test]
    fn http_policy_rejects_plaintext_and_ambiguous_bases() {
        assert!(secure_http_endpoint("https://discord.com/api/v10", false));
        assert!(!secure_http_endpoint("http://example.com", true));
        assert!(!secure_http_endpoint("http://127.0.0.1", false));
        assert!(secure_http_endpoint("http://127.0.0.1:8080", true));
        assert!(!secure_http_endpoint("https://a:b@example.com", false));
        assert!(!secure_http_endpoint("https://example.com/?q=1", false));
        assert!(!secure_http_endpoint("https://example.com/#fragment", false));
    }

    #[test]
    fn discord_origin_policy_uses_domain_boundaries() {
        assert!(discord_websocket_origin("wss://gateway.discord.gg"));
        assert!(discord_websocket_origin("wss://voice.discord.media"));
        assert!(!discord_websocket_origin("wss://discord.gg.example.com"));
        assert!(!discord_websocket_origin("wss://gateway.discord.gg:444"));
        assert!(same_origin("wss://example.com/a", "wss://example.com/b"));
        assert!(!same_origin("wss://example.com", "wss://other.example.com"));
    }
}
