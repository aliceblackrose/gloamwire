//! Discord Gateway WebSocket support.

mod close;
mod compression;
mod connection;
mod dispatch;
mod encoding;
mod event;
mod identify;
mod intents;
mod rate_limit;
mod send;
mod session;
mod shard;

pub use close::{GatewayCloseCode, GatewayReconnectStrategy};
pub use compression::GatewayCompression;
pub use connection::{GatewayConfig, GatewayConnection};
pub use dispatch::{
    AutoModerationActionExecutionEvent, GuildAuditLogEntryCreateEvent, GuildMemberAddEvent,
    GuildMemberRemoveEvent, GuildMemberUpdateEvent, GuildMembersChunkEvent, GuildRoleDeleteEvent,
    GuildRoleEvent, GuildScheduledEventUserEvent, InviteCreateEvent, InviteDeleteEvent,
    MessageDeleteBulkEvent, MessageDeleteEvent, MessagePollVoteEvent, MessageReactionAddEvent,
    MessageReactionRemoveAllEvent, MessageReactionRemoveEmojiEvent, MessageReactionRemoveEvent,
    ReadyApplication, ReadyEvent, TypedDispatchEvent, WebhooksUpdateEvent,
};
pub use encoding::GatewayEncoding;
pub use event::{DispatchEvent, GatewayEvent};
pub use intents::GatewayIntents;
pub use send::{
    ChannelInfoField, GatewayActivity, GatewayActivityType, GatewayStatus, RequestChannelInfo,
    RequestGuildMembers, RequestSoundboardSounds, UpdatePresence, UpdateVoiceState,
};
pub use session::GatewaySession;
pub use shard::{ShardCount, ShardEvent, ShardId, ShardManager, shard_for_guild};
