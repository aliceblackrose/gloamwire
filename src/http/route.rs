use std::{
    hash::{BuildHasher, RandomState},
    sync::OnceLock,
};

use reqwest::Method;

use crate::model::ChannelId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetrySafety {
    Safe,
    Unsafe,
}

#[derive(Clone)]
pub(crate) struct Route {
    pub(crate) method: Method,
    pub(crate) path: String,
    template: &'static str,
    major: Option<String>,
    retry_safety: RetrySafety,
    token_authenticated: bool,
    global_exempt: bool,
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("method", &self.method)
            .field("template", &self.template)
            .field("token_authenticated", &self.token_authenticated)
            .finish_non_exhaustive()
    }
}

impl Route {
    pub(crate) fn current_user() -> Self {
        Self::new(
            Method::GET,
            "/users/@me",
            "/users/@me",
            None,
            RetrySafety::Safe,
        )
    }

    pub(crate) fn gateway_bot() -> Self {
        Self::new(
            Method::GET,
            "/gateway/bot",
            "/gateway/bot",
            None,
            RetrySafety::Safe,
        )
    }

    pub(crate) fn create_message(channel_id: ChannelId) -> Self {
        Self::new(
            Method::POST,
            format!("/channels/{channel_id}/messages"),
            "/channels/{channel_id}/messages",
            Some(channel_id.to_string()),
            RetrySafety::Unsafe,
        )
    }

    pub(crate) fn new(
        method: Method,
        path: impl Into<String>,
        template: &'static str,
        major: Option<String>,
        retry_safety: RetrySafety,
    ) -> Self {
        Self {
            method,
            path: path.into(),
            template,
            major,
            retry_safety,
            token_authenticated: false,
            global_exempt: false,
        }
    }

    pub(crate) fn token_authenticated(mut self, token: &str, global_exempt: bool) -> Self {
        // A keyed, process-local fingerprint avoids retaining plaintext tokens
        // in rate-limit maps or debug output.
        static HASHER: OnceLock<RandomState> = OnceLock::new();
        let fingerprint = HASHER.get_or_init(RandomState::new).hash_one(token);
        self.major = Some(format!("{}:{fingerprint:016x}", self.major()));
        self.token_authenticated = true;
        self.global_exempt = global_exempt;
        self
    }

    pub(crate) const fn uses_token_auth(&self) -> bool {
        self.token_authenticated
    }

    pub(crate) const fn global_exempt(&self) -> bool {
        self.global_exempt
    }

    pub(crate) fn identity(&self) -> String {
        format!("{} {}", self.method, self.template)
    }

    pub(crate) fn major(&self) -> &str {
        self.major.as_deref().unwrap_or("")
    }

    pub(crate) fn is_retry_safe(&self) -> bool {
        self.retry_safety == RetrySafety::Safe
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::model::ChannelId;

    use super::Route;

    #[test]
    fn read_routes_are_retry_safe() {
        assert!(Route::current_user().is_retry_safe());
        assert!(Route::gateway_bot().is_retry_safe());
    }

    #[test]
    fn create_message_is_not_implicitly_retried() {
        assert!(!Route::create_message(ChannelId::new(1)).is_retry_safe());
    }

    proptest! {
        #[test]
        fn message_route_identity_ignores_major_parameter(
            channel_a in any::<u64>(),
            channel_b in any::<u64>(),
        ) {
            let route_a = Route::create_message(ChannelId::new(channel_a));
            let route_b = Route::create_message(ChannelId::new(channel_b));

            prop_assert_eq!(route_a.identity(), route_b.identity());
            prop_assert_eq!(route_a.major(), channel_a.to_string());
            prop_assert_eq!(route_b.major(), channel_b.to_string());
            prop_assert_eq!(route_a.major() == route_b.major(), channel_a == channel_b);
            prop_assert_eq!(route_a.path == route_b.path, channel_a == channel_b);
        }
    }
}
