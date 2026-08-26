//! Discord CDN URL construction helpers.

use std::fmt;

use crate::model::{
    ApplicationId, EmojiId, GuildId, RoleId, ScheduledEventId, Snowflake, StickerId, UserId,
};

const CDN_BASE_URL: &str = "https://cdn.discordapp.com";
const MEDIA_BASE_URL: &str = "https://media.discordapp.net";
const STICKER_PACK_APPLICATION_ID: u64 = 710_982_414_301_790_216;

/// Valid Discord CDN image size.
///
/// Discord accepts powers of two from 16 through 4096.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CdnImageSize(u16);

impl CdnImageSize {
    /// Creates a validated CDN image size.
    pub const fn new(size: u16) -> Option<Self> {
        if size >= 16 && size <= 4096 && size.is_power_of_two() {
            Some(Self(size))
        } else {
            None
        }
    }

    /// Returns the size in pixels.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for CdnImageSize {
    type Error = InvalidCdnImageSize;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(InvalidCdnImageSize(value))
    }
}

impl fmt::Display for CdnImageSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Error returned when a CDN image size is outside Discord's accepted set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid Discord CDN image size {0}; expected a power of two from 16 through 4096")]
pub struct InvalidCdnImageSize(pub u16);

/// Format for non-animated Discord CDN images.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CdnImageFormat {
    Jpeg,
    Png,
    #[default]
    WebP,
}

impl CdnImageFormat {
    const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
        }
    }
}

/// Format for CDN endpoints that can contain animated assets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CdnAnimatedImageFormat {
    Jpeg,
    Png,
    #[default]
    WebP,
    Gif,
}

impl CdnAnimatedImageFormat {
    const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Gif => "gif",
        }
    }
}

/// Options for a non-animated CDN image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CdnImageOptions {
    pub format: CdnImageFormat,
    pub size: Option<CdnImageSize>,
}

impl CdnImageOptions {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            format: CdnImageFormat::WebP,
            size: None,
        }
    }

    #[must_use]
    pub const fn format(mut self, format: CdnImageFormat) -> Self {
        self.format = format;
        self
    }

    #[must_use]
    pub const fn size(mut self, size: CdnImageSize) -> Self {
        self.size = Some(size);
        self
    }
}

/// Options for CDN images that may be animated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CdnAnimatedImageOptions {
    pub format: CdnAnimatedImageFormat,
    pub size: Option<CdnImageSize>,
    /// Requests animated WebP when the endpoint has an animated asset.
    pub animated: bool,
}

impl Default for CdnAnimatedImageOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CdnAnimatedImageOptions {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            format: CdnAnimatedImageFormat::WebP,
            size: None,
            animated: true,
        }
    }

    #[must_use]
    pub const fn format(mut self, format: CdnAnimatedImageFormat) -> Self {
        self.format = format;
        self
    }

    #[must_use]
    pub const fn size(mut self, size: CdnImageSize) -> Self {
        self.size = Some(size);
        self
    }

    #[must_use]
    pub const fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
}

/// Format used by Discord sticker CDN endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CdnStickerFormat {
    #[default]
    Png,
    Lottie,
    Gif,
}

impl CdnStickerFormat {
    const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Lottie => "json",
            Self::Gif => "gif",
        }
    }
}

/// Discord CDN URL builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cdn {
    base_url: String,
    media_base_url: String,
}

impl Default for Cdn {
    fn default() -> Self {
        Self::new()
    }
}

impl Cdn {
    /// Creates a builder using Discord's production CDN hosts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_url: CDN_BASE_URL.to_owned(),
            media_base_url: MEDIA_BASE_URL.to_owned(),
        }
    }

    /// Overrides the CDN base URL, primarily for compatible proxies and tests.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Overrides the media-proxy base URL used for GIF stickers.
    #[must_use]
    pub fn with_media_base_url(mut self, media_base_url: impl Into<String>) -> Self {
        self.media_base_url = media_base_url.into().trim_end_matches('/').to_owned();
        self
    }

    #[must_use]
    pub fn custom_emoji(
        &self,
        emoji_id: EmojiId,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(&format!("emojis/{emoji_id}"), None, options)
    }

    #[must_use]
    pub fn guild_icon(
        &self,
        guild_id: GuildId,
        hash: &str,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(&format!("icons/{guild_id}/{hash}"), Some(hash), options)
    }

    #[must_use]
    pub fn guild_splash(
        &self,
        guild_id: GuildId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("splashes/{guild_id}/{hash}"), options)
    }

    #[must_use]
    pub fn guild_discovery_splash(
        &self,
        guild_id: GuildId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(
            &format!("discovery-splashes/{guild_id}/{hash}"),
            options,
        )
    }

    #[must_use]
    pub fn guild_banner(
        &self,
        guild_id: GuildId,
        hash: &str,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(&format!("banners/{guild_id}/{hash}"), Some(hash), options)
    }

    #[must_use]
    pub fn user_banner(
        &self,
        user_id: UserId,
        hash: &str,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(&format!("banners/{user_id}/{hash}"), Some(hash), options)
    }

    #[must_use]
    pub fn user_avatar(
        &self,
        user_id: UserId,
        hash: &str,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(&format!("avatars/{user_id}/{hash}"), Some(hash), options)
    }

    /// Returns the default avatar URL for a user on Discord's current username system.
    #[must_use]
    pub fn default_user_avatar(&self, user_id: UserId) -> String {
        let index = (user_id.get() >> 22) % 6;
        format!("{}/embed/avatars/{index}.png", self.base_url)
    }

    /// Returns the legacy default avatar URL selected by discriminator modulo 5.
    #[must_use]
    pub fn legacy_default_user_avatar(&self, discriminator: u16) -> String {
        let index = discriminator % 5;
        format!("{}/embed/avatars/{index}.png", self.base_url)
    }

    #[must_use]
    pub fn guild_member_avatar(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        hash: &str,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(
            &format!("guilds/{guild_id}/users/{user_id}/avatars/{hash}"),
            Some(hash),
            options,
        )
    }

    #[must_use]
    pub fn guild_member_banner(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        hash: &str,
        options: CdnAnimatedImageOptions,
    ) -> String {
        self.animated_image(
            &format!("guilds/{guild_id}/users/{user_id}/banners/{hash}"),
            Some(hash),
            options,
        )
    }

    #[must_use]
    pub fn avatar_decoration(&self, asset: &str) -> String {
        format!("{}/avatar-decoration-presets/{asset}.png", self.base_url)
    }

    #[must_use]
    pub fn application_icon(
        &self,
        application_id: ApplicationId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("app-icons/{application_id}/{hash}"), options)
    }

    #[must_use]
    pub fn application_cover(
        &self,
        application_id: ApplicationId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("app-icons/{application_id}/{hash}"), options)
    }

    #[must_use]
    pub fn application_asset(
        &self,
        application_id: ApplicationId,
        asset_id: Snowflake,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("app-assets/{application_id}/{asset_id}"), options)
    }

    #[must_use]
    pub fn achievement_icon(
        &self,
        application_id: ApplicationId,
        achievement_id: Snowflake,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(
            &format!(
                "app-assets/{application_id}/achievements/{achievement_id}/icons/{hash}"
            ),
            options,
        )
    }

    #[must_use]
    pub fn sticker_pack_banner(&self, asset_id: Snowflake, options: CdnImageOptions) -> String {
        self.image(
            &format!("app-assets/{STICKER_PACK_APPLICATION_ID}/store/{asset_id}"),
            options,
        )
    }

    #[must_use]
    pub fn team_icon(
        &self,
        team_id: Snowflake,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("team-icons/{team_id}/{hash}"), options)
    }

    #[must_use]
    pub fn sticker(&self, sticker_id: StickerId, format: CdnStickerFormat) -> String {
        let base = if format == CdnStickerFormat::Gif {
            &self.media_base_url
        } else {
            &self.base_url
        };
        format!("{base}/stickers/{sticker_id}.{}", format.extension())
    }

    #[must_use]
    pub fn role_icon(
        &self,
        role_id: RoleId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("role-icons/{role_id}/{hash}"), options)
    }

    #[must_use]
    pub fn scheduled_event_cover(
        &self,
        event_id: ScheduledEventId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("guild-events/{event_id}/{hash}"), options)
    }

    #[must_use]
    pub fn guild_tag_badge(
        &self,
        guild_id: GuildId,
        hash: &str,
        options: CdnImageOptions,
    ) -> String {
        self.image(&format!("guild-tag-badges/{guild_id}/{hash}"), options)
    }

    fn image(&self, path: &str, options: CdnImageOptions) -> String {
        image_url(
            &self.base_url,
            path,
            options.format.extension(),
            options.size,
            false,
        )
    }

    fn animated_image(
        &self,
        path: &str,
        hash: Option<&str>,
        options: CdnAnimatedImageOptions,
    ) -> String {
        let animated_webp = options.animated
            && options.format == CdnAnimatedImageFormat::WebP
            && hash.is_none_or(|hash| hash.starts_with("a_"));
        image_url(
            &self.base_url,
            path,
            options.format.extension(),
            options.size,
            animated_webp,
        )
    }
}

fn image_url(
    base_url: &str,
    path: &str,
    extension: &str,
    size: Option<CdnImageSize>,
    animated: bool,
) -> String {
    let mut url = format!("{base_url}/{path}.{extension}");
    let mut separator = '?';

    if let Some(size) = size {
        url.push(separator);
        separator = '&';
        url.push_str("size=");
        url.push_str(&size.to_string());
    }

    if animated {
        url.push(separator);
        url.push_str("animated=true");
    }

    url
}

#[cfg(test)]
mod tests {
    use crate::model::{EmojiId, GuildId, StickerId, UserId};

    use super::{
        Cdn, CdnAnimatedImageOptions, CdnImageOptions, CdnImageSize, CdnStickerFormat,
    };

    #[test]
    fn validates_discord_image_sizes() {
        assert_eq!(CdnImageSize::new(16).map(CdnImageSize::get), Some(16));
        assert_eq!(CdnImageSize::new(4096).map(CdnImageSize::get), Some(4096));
        assert!(CdnImageSize::new(15).is_none());
        assert!(CdnImageSize::new(100).is_none());
        assert!(CdnImageSize::new(8192).is_none());
    }

    #[test]
    fn animated_hash_uses_animated_webp_query() {
        let url = Cdn::new().user_avatar(
            UserId::new(1),
            "a_hash",
            CdnAnimatedImageOptions::new().size(CdnImageSize::new(256).expect("size")),
        );

        assert_eq!(
            url,
            "https://cdn.discordapp.com/avatars/1/a_hash.webp?size=256&animated=true"
        );
    }

    #[test]
    fn static_hash_does_not_request_animation() {
        let url = Cdn::new().guild_icon(
            GuildId::new(2),
            "hash",
            CdnAnimatedImageOptions::new(),
        );
        assert_eq!(url, "https://cdn.discordapp.com/icons/2/hash.webp");
    }

    #[test]
    fn custom_emoji_defaults_to_animated_webp() {
        let url = Cdn::new().custom_emoji(EmojiId::new(3), CdnAnimatedImageOptions::new());
        assert_eq!(url, "https://cdn.discordapp.com/emojis/3.webp?animated=true");
    }

    #[test]
    fn gif_stickers_use_media_proxy() {
        let url = Cdn::new().sticker(StickerId::new(4), CdnStickerFormat::Gif);
        assert_eq!(url, "https://media.discordapp.net/stickers/4.gif");
    }

    #[test]
    fn default_avatar_uses_current_username_index() {
        let user_id = UserId::new(5 << 22);
        assert_eq!(
            Cdn::new().default_user_avatar(user_id),
            "https://cdn.discordapp.com/embed/avatars/5.png"
        );
    }

    #[test]
    fn static_images_accept_valid_size() {
        let url = Cdn::new().guild_splash(
            GuildId::new(9),
            "splash",
            CdnImageOptions::new().size(CdnImageSize::new(1024).expect("size")),
        );
        assert_eq!(
            url,
            "https://cdn.discordapp.com/splashes/9/splash.webp?size=1024"
        );
    }
}
