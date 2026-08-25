//! Discord HTTP API support.

mod client;
mod encoding;
mod message;
mod models;
mod pagination;
mod rate_limit;
mod response;
mod route;
mod upload;

pub use client::{RestClient, RestClientBuilder};
pub use message::{
    ChannelPinsQuery, MessageListQuery, MessageSearchIndexing, MessageSearchQuery,
    MessageSearchResponse, MessageSearchResult, ReactionUsersQuery,
};
pub use models::{GatewayBot, SessionStartLimit};
pub use pagination::Pagination;
pub use response::HttpResponse;
pub use upload::{UploadFile, UploadSource};
