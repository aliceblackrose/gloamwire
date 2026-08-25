//! Core Discord data models.

mod channel;
mod guild;
mod id;
mod member;
mod message;
mod permissions;
mod presence;
mod reaction;
mod role;
mod snowflake;
mod user;
mod voice;

pub use channel::{Channel, ChannelFlags, ChannelType, DefaultReaction, ForumTag, ThreadMetadata};
pub use guild::{Guild, UnavailableGuild};
pub use id::{
    ApplicationId, AttachmentId, ChannelId, CommandId, EmojiId, EntitlementId, GuildId,
    InteractionId, MessageId, RoleId, ScheduledEventId, SkuId, SoundboardSoundId, StickerId,
    UserId, WebhookId,
};
pub use member::{GuildMember, GuildMemberFlags};
pub use message::{CreateMessage, Message};
pub use permissions::{
    PermissionOverwrite, PermissionOverwriteType, Permissions, compute_base_permissions,
    compute_channel_permissions,
};
pub use presence::{ClientStatus, PresenceStatus, PresenceUpdate};
pub use reaction::{PartialEmoji, Reaction, ReactionCountDetails, ReactionType};
pub use role::{Role, RoleColors};
pub use snowflake::{DISCORD_EPOCH_MILLIS, Snowflake};
pub use user::{PartialUser, User};
pub use voice::VoiceState;
