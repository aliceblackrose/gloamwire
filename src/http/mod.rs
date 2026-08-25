//! Discord HTTP API support.

mod channel;
mod client;
mod encoding;
mod guild;
mod message;
mod models;
mod pagination;
mod rate_limit;
mod response;
mod route;
mod upload;

pub use channel::{
    ArchivedThreadsQuery, EditChannelPermission, FollowedChannel, ForumTagRequest,
    ForumThreadMessage, GroupDmAddRecipient, JoinedPrivateArchivedThreadsQuery, ModifyChannel,
    PermissionOverwriteRequest, SetVoiceChannelStatus, StartForumThread, StartThread,
    StartThreadFromMessage, ThreadMembersQuery,
};
pub use client::{RestClient, RestClientBuilder};
pub use guild::{CreateGuildChannel, ModifyGuild, ModifyGuildChannelPosition};
pub use message::{
    ChannelPinsQuery, MessageListQuery, MessageSearchIndexing, MessageSearchQuery,
    MessageSearchResponse, MessageSearchResult, ReactionUsersQuery,
};
pub use models::{GatewayBot, SessionStartLimit};
pub use pagination::Pagination;
pub use response::HttpResponse;
pub use upload::{UploadFile, UploadSource};
