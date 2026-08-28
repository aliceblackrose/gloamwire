//! Gloamwire is an asynchronous Discord Gateway and REST API library.
//!
//! The crate intentionally starts small: it provides reusable Discord models,
//! HTTP and Gateway transports, snowflake handling, and Gateway lifecycle
//! primitives without imposing a bot framework or command system on downstream
//! applications.
//!
//! The default feature set preserves the complete transport surface. Consumers
//! that only need Discord data types can disable default features and enable the
//! `model` feature.

#![deny(unsafe_code)]

#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "model")]
pub mod cdn;
#[cfg(feature = "transport")]
pub mod error;
#[cfg(feature = "transport")]
pub mod gateway;
#[cfg(feature = "transport")]
pub mod http;
#[cfg(feature = "model")]
pub mod model;
#[cfg(feature = "transport")]
pub mod oauth2;

#[cfg(feature = "cache")]
pub use cache::{Cache, CacheConfig};
#[cfg(feature = "model")]
pub use cdn::{
    Cdn, CdnAnimatedImageFormat, CdnAnimatedImageOptions, CdnImageFormat, CdnImageOptions,
    CdnImageSize, CdnStickerFormat, InvalidCdnImageSize,
};
#[cfg(feature = "transport")]
pub use error::{DiscordApiError, DiscordValidationError, Error, OAuth2ApiError, Result};
#[cfg(feature = "transport")]
pub use http::{Pagination, RestClient, RestClientBuilder, UploadFile, UploadSource};
#[cfg(feature = "transport")]
pub use oauth2::{
    OAuth2Application, OAuth2AuthorizationInfo, OAuth2AuthorizationUrl, OAuth2Client, OAuth2Prompt,
    OAuth2ResponseType, OAuth2TokenResponse, OAuth2TokenTypeHint,
};
