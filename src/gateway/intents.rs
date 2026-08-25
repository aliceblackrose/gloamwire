use bitflags::bitflags;

bitflags! {
    /// Event groups requested when identifying a Discord Gateway session.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GatewayIntents: u64 {
        /// Guild lifecycle, roles, channels, threads, and related events.
        const GUILDS = 1 << 0;
        /// Guild member events. This intent is privileged.
        const GUILD_MEMBERS = 1 << 1;
        /// Guild moderation events.
        const GUILD_MODERATION = 1 << 2;
        /// Guild emoji, sticker, and soundboard expression events.
        const GUILD_EXPRESSIONS = 1 << 3;
        /// Guild integration events.
        const GUILD_INTEGRATIONS = 1 << 4;
        /// Guild webhook events.
        const GUILD_WEBHOOKS = 1 << 5;
        /// Guild invite events.
        const GUILD_INVITES = 1 << 6;
        /// Guild voice state events.
        const GUILD_VOICE_STATES = 1 << 7;
        /// Guild presence events. This intent is privileged.
        const GUILD_PRESENCES = 1 << 8;
        /// Guild message events.
        const GUILD_MESSAGES = 1 << 9;
        /// Guild message reaction events.
        const GUILD_MESSAGE_REACTIONS = 1 << 10;
        /// Guild typing events.
        const GUILD_MESSAGE_TYPING = 1 << 11;
        /// Direct-message events.
        const DIRECT_MESSAGES = 1 << 12;
        /// Direct-message reaction events.
        const DIRECT_MESSAGE_REACTIONS = 1 << 13;
        /// Direct-message typing events.
        const DIRECT_MESSAGE_TYPING = 1 << 14;
        /// Message content fields. This intent is privileged.
        const MESSAGE_CONTENT = 1 << 15;
        /// Guild scheduled event events.
        const GUILD_SCHEDULED_EVENTS = 1 << 16;
        /// Auto Moderation rule configuration events.
        const AUTO_MODERATION_CONFIGURATION = 1 << 20;
        /// Auto Moderation execution events.
        const AUTO_MODERATION_EXECUTION = 1 << 21;
        /// Guild poll vote events.
        const GUILD_MESSAGE_POLLS = 1 << 24;
        /// Direct-message poll vote events.
        const DIRECT_MESSAGE_POLLS = 1 << 25;
    }
}
