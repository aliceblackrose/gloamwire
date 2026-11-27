//! Discord Gateway WebSocket support.

mod close;
mod compression;
mod connection;
mod coordination;
mod dispatch;
mod distributed;
mod driver;
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
pub use coordination::{
    DistributedShardCoordinator, GatewayCoordinationError, GatewayCoordinationFuture,
    GatewayCoordinationResult, GatewayIdentifyCoordinator, ShardLease,
};
pub use dispatch::{
    AutoModerationActionExecutionEvent, GuildAuditLogEntryCreateEvent, GuildMemberAddEvent,
    GuildMemberRemoveEvent, GuildMemberUpdateEvent, GuildMembersChunkEvent, GuildRoleDeleteEvent,
    GuildRoleEvent, GuildScheduledEventUserEvent, InviteCreateEvent, InviteDeleteEvent,
    MessageDeleteBulkEvent, MessageDeleteEvent, MessagePollVoteEvent, MessageReactionAddEvent,
    MessageReactionRemoveAllEvent, MessageReactionRemoveEmojiEvent, MessageReactionRemoveEvent,
    MessageUpdateEvent, ReadyApplication, ReadyEvent, ThreadListSyncEvent, TypedDispatchEvent,
    WebhooksUpdateEvent,
};
pub use distributed::{
    DistributedShardConfig, DistributedShardEvent, DistributedShardManager, LeaseIdentity,
};
pub use driver::{GatewayCommand, GatewayCommandSender, GatewayDriver};
pub use encoding::GatewayEncoding;
pub use event::{DispatchEvent, GatewayEvent};
pub use intents::GatewayIntents;
pub use send::{
    ChannelInfoField, GatewayActivity, GatewayActivityType, GatewayStatus, RequestChannelInfo,
    RequestGuildMembers, RequestSoundboardSounds, UpdatePresence, UpdateVoiceState,
};
pub use session::GatewaySession;
pub use shard::{ShardCount, ShardEvent, ShardId, ShardManager, shard_for_guild};

/// Unstable parser hooks for cargo-fuzz; not part of the supported API.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing {
    use super::{GatewayCompression, GatewayEncoding, compression::GatewayDecoder};

    pub fn json(bytes: &[u8]) {
        super::connection::fuzz_envelope(GatewayEncoding::Json, bytes);
    }

    pub fn etf(bytes: &[u8]) {
        super::connection::fuzz_envelope(GatewayEncoding::Etf, bytes);
    }

    pub fn zlib(bytes: &[u8]) {
        let mut decoder = GatewayDecoder::new(GatewayCompression::ZlibStream).unwrap();
        let split = bytes.len() / 2;
        for chunk in [&bytes[..split], &bytes[split..]] {
            match decoder.decode(chunk) {
                Ok(Some(decoded)) => {
                    assert!(decoded.len() <= 32 * 1024 * 1024);
                    json(&decoded);
                }
                Ok(None) => {}
                Err(_) => break,
            }
        }
    }

    pub fn zstd(bytes: &[u8]) {
        let mut decoder = GatewayDecoder::new(GatewayCompression::ZstdStream).unwrap();
        if let Ok(Some(decoded)) = decoder.decode(bytes) {
            assert!(decoded.len() <= 32 * 1024 * 1024);
            json(&decoded);
        }
    }
}
