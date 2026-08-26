//! Discord HTTP API support.

mod audit_log;
mod channel;
mod client;
mod command;
pub(crate) mod encoding;
mod guild;
mod interaction;
mod invite;
mod member;
mod message;
mod models;
mod moderation;
mod pagination;
mod rate_limit;
mod response;
mod role;
mod route;
mod scheduled_event;
mod upload;
mod webhook;

pub use audit_log::AuditLogQuery;
pub use channel::{
    ArchivedThreadsQuery, EditChannelPermission, FollowedChannel, ForumTagRequest,
    ForumThreadMessage, GroupDmAddRecipient, JoinedPrivateArchivedThreadsQuery, ModifyChannel,
    PermissionOverwriteRequest, SetVoiceChannelStatus, StartForumThread, StartThread,
    StartThreadFromMessage, ThreadMembersQuery,
};
pub use client::{RestClient, RestClientBuilder};
pub use command::{
    BulkOverwriteApplicationCommand, CreateApplicationCommand, EditApplicationCommand,
    EditApplicationCommandPermissions,
};
pub use guild::{CreateGuildChannel, ModifyGuild, ModifyGuildChannelPosition};
pub use interaction::{CreateInteractionResponseQuery, EditInteractionMessageQuery};
pub use invite::{CreateChannelInvite, GetInviteQuery};
pub use member::{
    AddGuildMember, BeginGuildPrune, BulkGuildBan, BulkGuildBanResponse, CreateGuildBan,
    GuildBansQuery, GuildMembersQuery, GuildPruneQuery, GuildPruneResult, ModifyCurrentMember,
    ModifyGuildMember, SearchGuildMembersQuery,
};
pub use message::{
    ChannelPinsQuery, MessageListQuery, MessageSearchIndexing, MessageSearchQuery,
    MessageSearchResponse, MessageSearchResult, ReactionUsersQuery,
};
pub use models::{GatewayBot, SessionStartLimit};
pub use moderation::{CreateAutoModerationRule, ModifyAutoModerationRule};
pub use pagination::Pagination;
pub use response::HttpResponse;
pub use role::{CreateGuildRole, ModifyGuildRole, ModifyGuildRolePosition};
pub use scheduled_event::{
    CreateGuildScheduledEvent, GuildScheduledEventRecurrenceRuleRequest,
    GuildScheduledEventUsersQuery, ModifyGuildScheduledEvent,
};
pub use upload::{UploadFile, UploadSource};
pub use webhook::{
    CreateWebhook, EditWebhookMessage, EditWebhookMessageQuery, ExecuteCompatibleWebhookQuery,
    ExecuteWebhook, ExecuteWebhookQuery, ModifyWebhook, ModifyWebhookWithToken,
    WebhookMessageQuery,
};
