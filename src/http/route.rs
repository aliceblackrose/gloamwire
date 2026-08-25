use reqwest::Method;

use crate::model::ChannelId;

#[derive(Debug, Clone)]
pub(crate) struct Route {
    pub(crate) method: Method,
    pub(crate) path: String,
    template: &'static str,
    major: Option<String>,
}

impl Route {
    pub(crate) fn current_user() -> Self {
        Self::new(Method::GET, "/users/@me", "/users/@me", None)
    }

    pub(crate) fn gateway_bot() -> Self {
        Self::new(Method::GET, "/gateway/bot", "/gateway/bot", None)
    }

    pub(crate) fn create_message(channel_id: ChannelId) -> Self {
        Self::new(
            Method::POST,
            format!("/channels/{channel_id}/messages"),
            "/channels/{channel_id}/messages",
            Some(channel_id.to_string()),
        )
    }

    fn new(
        method: Method,
        path: impl Into<String>,
        template: &'static str,
        major: Option<String>,
    ) -> Self {
        Self {
            method,
            path: path.into(),
            template,
            major,
        }
    }

    pub(crate) fn identity(&self) -> String {
        format!("{} {}", self.method, self.template)
    }

    pub(crate) fn major(&self) -> &str {
        self.major.as_deref().unwrap_or("")
    }
}
