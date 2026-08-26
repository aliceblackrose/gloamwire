//! Gloamwire is an asynchronous Discord Gateway and REST API library.
//!
//! The crate intentionally starts small: it provides a reusable HTTP client,
//! core Discord models, snowflake handling, Gateway intents, and a maintained
//! Gateway WebSocket connection without imposing a bot framework or command
//! system on downstream applications.

#![deny(unsafe_code)]

pub mod cdn;
pub mod error;
pub mod gateway;
pub mod http;
pub mod model;
pub mod oauth2;

pub use cdn::{
    Cdn, CdnAnimatedImageFormat, CdnAnimatedImageOptions, CdnImageFormat, CdnImageOptions,
    CdnImageSize, CdnStickerFormat, InvalidCdnImageSize,
};
pub use error::{DiscordApiError, DiscordValidationError, Error, OAuth2ApiError, Result};
pub use http::{Pagination, RestClient, RestClientBuilder, UploadFile, UploadSource};
pub use oauth2::{
    OAuth2Application, OAuth2AuthorizationInfo, OAuth2AuthorizationUrl, OAuth2Client, OAuth2Prompt,
    OAuth2ResponseType, OAuth2TokenResponse, OAuth2TokenTypeHint,
};
