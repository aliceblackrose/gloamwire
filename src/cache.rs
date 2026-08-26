//! Optional in-memory state derived from typed Discord Gateway dispatches.
//!
//! The cache is intentionally not internally synchronized. Applications with
//! multiple reader/writer tasks can wrap it in their synchronization primitive
//! of choice without Gloamwire imposing a locking strategy.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    Result,
    gateway::{DispatchEvent, GatewayEvent, GuildMemberUpdateEvent, TypedDispatchEvent},
    model::{
        Channel, ChannelId, ChannelType, Guild, GuildId, GuildMember, GuildMemberFlags,
        GuildScheduledEvent, Message, MessageId, PresenceUpdate, Role, RoleId, ScheduledEventId,
        User, UserId, VoiceState,
    },
};

/// Configuration for Gloamwire's optional in-memory cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheConfig {
    message_capacity: usize,
}

impl CacheConfig {
    /// Creates cache configuration with message caching disabled.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            message_capacity: 0,
        }
    }

    /// Sets the maximum number of `MESSAGE_CREATE` payloads retained.
    ///
    /// A capacity of zero disables message retention while still allowing the
    /// cache to update users and channel last-message metadata from the event.
    #[must_use]
    pub const fn message_capacity(mut self, capacity: usize) -> Self {
        self.message_capacity = capacity;
        self
    }

    /// Returns the configured message capacity.
    #[must_use]
    pub const fn max_messages(&self) -> usize {
        self.message_capacity
    }
}

/// Normalized in-memory Discord state updated from typed Gateway dispatches.
#[derive(Debug, Clone)]
pub struct Cache {
    config: CacheConfig,
    current_user_id: Option<UserId>,
    users: HashMap<UserId, User>,
    guilds: HashMap<GuildId, Guild>,
    unavailable_guilds: HashSet<GuildId>,
    channels: HashMap<ChannelId, Channel>,
    roles: HashMap<(GuildId, RoleId), Role>,
    members: HashMap<(GuildId, UserId), GuildMember>,
    presences: HashMap<(GuildId, UserId), PresenceUpdate>,
    voice_states: HashMap<(GuildId, UserId), VoiceState>,
    scheduled_events: HashMap<ScheduledEventId, GuildScheduledEvent>,
    messages: HashMap<MessageId, Message>,
    message_order: VecDeque<MessageId>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

impl Cache {
    /// Creates an empty cache with the supplied configuration.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            current_user_id: None,
            users: HashMap::new(),
            guilds: HashMap::new(),
            unavailable_guilds: HashSet::new(),
            channels: HashMap::new(),
            roles: HashMap::new(),
            members: HashMap::new(),
            presences: HashMap::new(),
            voice_states: HashMap::new(),
            scheduled_events: HashMap::new(),
            messages: HashMap::new(),
            message_order: VecDeque::new(),
        }
    }

    /// Returns the cache configuration.
    #[must_use]
    pub const fn config(&self) -> CacheConfig {
        self.config
    }

    /// Clears all Gateway-derived state while preserving configuration.
    pub fn clear(&mut self) {
        self.current_user_id = None;
        self.users.clear();
        self.guilds.clear();
        self.unavailable_guilds.clear();
        self.channels.clear();
        self.roles.clear();
        self.members.clear();
        self.presences.clear();
        self.voice_states.clear();
        self.scheduled_events.clear();
        self.messages.clear();
        self.message_order.clear();
    }

    /// Applies one already-typed Gateway dispatch to the cache.
    pub fn update(&mut self, event: &TypedDispatchEvent) {
        match event {
            TypedDispatchEvent::Ready(ready) => {
                self.clear();
                self.current_user_id = Some(ready.user.id);
                self.users.insert(ready.user.id, ready.user.clone());
                self.unavailable_guilds
                    .extend(ready.guilds.iter().map(|guild| guild.id));
            }
            TypedDispatchEvent::Resumed => {}
            TypedDispatchEvent::GuildCreate(guild) => self.seed_guild(guild),
            TypedDispatchEvent::GuildUpdate(guild) => self.update_guild(guild),
            TypedDispatchEvent::GuildDelete(guild) => {
                if guild.unavailable {
                    self.unavailable_guilds.insert(guild.id);
                    if let Some(cached) = self.guilds.get_mut(&guild.id) {
                        cached.unavailable = Some(true);
                    }
                } else {
                    self.remove_guild(guild.id);
                }
            }
            TypedDispatchEvent::ChannelCreate(channel)
            | TypedDispatchEvent::ChannelUpdate(channel)
            | TypedDispatchEvent::ThreadCreate(channel)
            | TypedDispatchEvent::ThreadUpdate(channel) => self.upsert_channel(channel),
            TypedDispatchEvent::ChannelDelete(channel)
            | TypedDispatchEvent::ThreadDelete(channel) => {
                self.remove_channel(channel.id);
            }
            TypedDispatchEvent::GuildMemberAdd(event) => {
                let inserted = event
                    .member
                    .user
                    .as_ref()
                    .is_some_and(|user| !self.members.contains_key(&(event.guild_id, user.id)));
                self.upsert_member(event.guild_id, &event.member);
                if inserted
                    && let Some(guild) = self.guilds.get_mut(&event.guild_id)
                    && let Some(member_count) = &mut guild.member_count
                {
                    *member_count = member_count.saturating_add(1);
                }
            }
            TypedDispatchEvent::GuildMemberUpdate(event) => self.merge_member_update(event),
            TypedDispatchEvent::GuildMemberRemove(event) => {
                let removed = self
                    .members
                    .remove(&(event.guild_id, event.user.id))
                    .is_some();
                self.users.insert(event.user.id, event.user.clone());
                if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
                    guild
                        .members
                        .retain(|member| member_user_id(member) != Some(event.user.id));
                    if removed && let Some(member_count) = &mut guild.member_count {
                        *member_count = member_count.saturating_sub(1);
                    }
                }
                self.presences.remove(&(event.guild_id, event.user.id));
                self.voice_states.remove(&(event.guild_id, event.user.id));
            }
            TypedDispatchEvent::GuildMembersChunk(event) => {
                for member in &event.members {
                    self.upsert_member(event.guild_id, member);
                }
            }
            TypedDispatchEvent::GuildRoleCreate(event)
            | TypedDispatchEvent::GuildRoleUpdate(event) => {
                self.upsert_role(event.guild_id, &event.role);
            }
            TypedDispatchEvent::GuildRoleDelete(event) => {
                self.roles.remove(&(event.guild_id, event.role_id));
                if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
                    guild.roles.retain(|role| role.id != event.role_id);
                }
            }
            TypedDispatchEvent::GuildScheduledEventCreate(event)
            | TypedDispatchEvent::GuildScheduledEventUpdate(event) => {
                self.upsert_scheduled_event(event);
            }
            TypedDispatchEvent::GuildScheduledEventDelete(event) => {
                self.scheduled_events.remove(&event.id);
                if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
                    guild
                        .guild_scheduled_events
                        .retain(|cached| cached.id != event.id);
                }
            }
            TypedDispatchEvent::MessageCreate(message) => self.cache_message(message),
            TypedDispatchEvent::MessageDelete(event) => self.remove_message(event.id),
            TypedDispatchEvent::MessageDeleteBulk(event) => {
                for message_id in &event.ids {
                    self.remove_message(*message_id);
                }
            }
            TypedDispatchEvent::PresenceUpdate(presence) => {
                self.merge_partial_user(presence);
                self.presences
                    .insert((presence.guild_id, presence.user.id), presence.clone());
            }
            TypedDispatchEvent::VoiceStateUpdate(voice_state) => {
                self.update_voice_state(voice_state)
            }
            TypedDispatchEvent::UserUpdate(user) => {
                self.current_user_id = Some(user.id);
                self.users.insert(user.id, user.clone());
            }
            _ => {}
        }
    }

    /// Parses a raw dispatch, applies it to the cache, and returns the typed event.
    pub fn update_dispatch(&mut self, dispatch: &DispatchEvent) -> Result<TypedDispatchEvent> {
        let event = dispatch.typed()?;
        self.update(&event);
        Ok(event)
    }

    /// Applies a Gateway event when it is a dispatch and returns its typed form.
    ///
    /// Heartbeat/reconnect lifecycle events do not mutate cache state and return
    /// `Ok(None)`.
    pub fn update_gateway_event(
        &mut self,
        event: &GatewayEvent,
    ) -> Result<Option<TypedDispatchEvent>> {
        match event {
            GatewayEvent::Dispatch(dispatch) => self.update_dispatch(dispatch).map(Some),
            _ => Ok(None),
        }
    }

    /// Returns the current bot/OAuth user after READY or USER_UPDATE.
    #[must_use]
    pub fn current_user(&self) -> Option<&User> {
        self.current_user_id.and_then(|id| self.users.get(&id))
    }

    #[must_use]
    pub fn user(&self, user_id: UserId) -> Option<&User> {
        self.users.get(&user_id)
    }

    /// Returns the latest guild payload. Child collections are also available
    /// through the normalized cache getters below.
    #[must_use]
    pub fn guild(&self, guild_id: GuildId) -> Option<&Guild> {
        self.guilds.get(&guild_id)
    }

    #[must_use]
    pub fn is_guild_unavailable(&self, guild_id: GuildId) -> bool {
        self.unavailable_guilds.contains(&guild_id)
    }

    #[must_use]
    pub fn channel(&self, channel_id: ChannelId) -> Option<&Channel> {
        self.channels.get(&channel_id)
    }

    #[must_use]
    pub fn role(&self, guild_id: GuildId, role_id: RoleId) -> Option<&Role> {
        self.roles.get(&(guild_id, role_id))
    }

    #[must_use]
    pub fn member(&self, guild_id: GuildId, user_id: UserId) -> Option<&GuildMember> {
        self.members.get(&(guild_id, user_id))
    }

    #[must_use]
    pub fn presence(&self, guild_id: GuildId, user_id: UserId) -> Option<&PresenceUpdate> {
        self.presences.get(&(guild_id, user_id))
    }

    #[must_use]
    pub fn voice_state(&self, guild_id: GuildId, user_id: UserId) -> Option<&VoiceState> {
        self.voice_states.get(&(guild_id, user_id))
    }

    #[must_use]
    pub fn scheduled_event(&self, event_id: ScheduledEventId) -> Option<&GuildScheduledEvent> {
        self.scheduled_events.get(&event_id)
    }

    #[must_use]
    pub fn message(&self, message_id: MessageId) -> Option<&Message> {
        self.messages.get(&message_id)
    }

    /// Iterates channels currently associated with one guild.
    pub fn guild_channels(&self, guild_id: GuildId) -> impl Iterator<Item = &Channel> {
        self.channels
            .values()
            .filter(move |channel| channel.guild_id == Some(guild_id))
    }

    /// Iterates cached members for one guild.
    pub fn guild_members(&self, guild_id: GuildId) -> impl Iterator<Item = &GuildMember> {
        self.members
            .iter()
            .filter_map(move |(&(cached_guild_id, _), member)| {
                (cached_guild_id == guild_id).then_some(member)
            })
    }

    fn seed_guild(&mut self, guild: &Guild) {
        let guild_id = guild.id;
        self.remove_guild_children(guild_id);
        self.unavailable_guilds.remove(&guild_id);

        let mut cached_guild = guild.clone();
        cached_guild.unavailable = Some(false);

        for channel in cached_guild
            .channels
            .iter_mut()
            .chain(cached_guild.threads.iter_mut())
        {
            if channel.guild_id.is_none() {
                channel.guild_id = Some(guild_id);
            }
            self.channels.insert(channel.id, channel.clone());
        }

        for role in &cached_guild.roles {
            self.roles.insert((guild_id, role.id), role.clone());
        }

        for member in &cached_guild.members {
            if let Some(user) = &member.user {
                self.users.insert(user.id, user.clone());
                self.members.insert((guild_id, user.id), member.clone());
            }
        }

        for voice_state in &mut cached_guild.voice_states {
            if voice_state.guild_id.is_none() {
                voice_state.guild_id = Some(guild_id);
            }
            if voice_state.channel_id.is_some() {
                self.voice_states
                    .insert((guild_id, voice_state.user_id), voice_state.clone());
            }
        }

        for event in &cached_guild.guild_scheduled_events {
            self.scheduled_events.insert(event.id, event.clone());
        }

        self.guilds.insert(guild_id, cached_guild);
    }

    fn update_guild(&mut self, guild: &Guild) {
        self.unavailable_guilds.remove(&guild.id);
        let mut updated = guild.clone();
        updated.unavailable = Some(false);

        if let Some(existing) = self.guilds.get(&guild.id) {
            updated.roles = existing.roles.clone();
            updated.channels = existing.channels.clone();
            updated.threads = existing.threads.clone();
            updated.members = existing.members.clone();
            updated.voice_states = existing.voice_states.clone();
            updated.guild_scheduled_events = existing.guild_scheduled_events.clone();
        }

        self.guilds.insert(guild.id, updated);
    }

    fn remove_guild(&mut self, guild_id: GuildId) {
        self.guilds.remove(&guild_id);
        self.unavailable_guilds.remove(&guild_id);
        self.remove_guild_children(guild_id);
        let message_ids = self
            .messages
            .iter()
            .filter_map(|(&id, message)| (message.guild_id == Some(guild_id)).then_some(id))
            .collect::<Vec<_>>();
        for message_id in message_ids {
            self.remove_message(message_id);
        }
    }

    fn remove_guild_children(&mut self, guild_id: GuildId) {
        self.channels
            .retain(|_, channel| channel.guild_id != Some(guild_id));
        self.roles
            .retain(|(cached_guild_id, _), _| *cached_guild_id != guild_id);
        self.members
            .retain(|(cached_guild_id, _), _| *cached_guild_id != guild_id);
        self.presences
            .retain(|(cached_guild_id, _), _| *cached_guild_id != guild_id);
        self.voice_states
            .retain(|(cached_guild_id, _), _| *cached_guild_id != guild_id);
        self.scheduled_events
            .retain(|_, event| event.guild_id != guild_id);
    }

    fn upsert_channel(&mut self, channel: &Channel) {
        let mut channel = channel.clone();
        if channel.guild_id.is_none()
            && let Some(existing) = self.channels.get(&channel.id)
        {
            channel.guild_id = existing.guild_id;
        }

        let guild_id = channel.guild_id;
        self.channels.insert(channel.id, channel.clone());

        if let Some(guild_id) = guild_id
            && let Some(guild) = self.guilds.get_mut(&guild_id)
        {
            guild.channels.retain(|cached| cached.id != channel.id);
            guild.threads.retain(|cached| cached.id != channel.id);
            if is_thread(channel.kind) {
                guild.threads.push(channel);
            } else {
                guild.channels.push(channel);
            }
        }
    }

    fn remove_channel(&mut self, channel_id: ChannelId) {
        let guild_id = self
            .channels
            .remove(&channel_id)
            .and_then(|channel| channel.guild_id);
        if let Some(guild_id) = guild_id
            && let Some(guild) = self.guilds.get_mut(&guild_id)
        {
            guild.channels.retain(|channel| channel.id != channel_id);
            guild.threads.retain(|channel| channel.id != channel_id);
        }

        let message_ids = self
            .messages
            .iter()
            .filter_map(|(&id, message)| (message.channel_id == channel_id).then_some(id))
            .collect::<Vec<_>>();
        for message_id in message_ids {
            self.remove_message(message_id);
        }
    }

    fn upsert_role(&mut self, guild_id: GuildId, role: &Role) {
        self.roles.insert((guild_id, role.id), role.clone());
        if let Some(guild) = self.guilds.get_mut(&guild_id) {
            if let Some(existing) = guild.roles.iter_mut().find(|cached| cached.id == role.id) {
                *existing = role.clone();
            } else {
                guild.roles.push(role.clone());
            }
        }
    }

    fn upsert_member(&mut self, guild_id: GuildId, member: &GuildMember) {
        let Some(user) = &member.user else {
            return;
        };
        self.users.insert(user.id, user.clone());
        self.members.insert((guild_id, user.id), member.clone());

        if let Some(guild) = self.guilds.get_mut(&guild_id) {
            if let Some(existing) = guild
                .members
                .iter_mut()
                .find(|cached| member_user_id(cached) == Some(user.id))
            {
                *existing = member.clone();
            } else {
                guild.members.push(member.clone());
            }
        }
    }

    fn merge_member_update(&mut self, event: &GuildMemberUpdateEvent) {
        self.users.insert(event.user.id, event.user.clone());
        let key = (event.guild_id, event.user.id);
        let updated = {
            let member = self.members.entry(key).or_insert_with(|| GuildMember {
                user: Some(event.user.clone()),
                nick: None,
                avatar: None,
                banner: None,
                bio: None,
                roles: Vec::new(),
                joined_at: None,
                premium_since: None,
                deaf: false,
                mute: false,
                flags: GuildMemberFlags::empty(),
                pending: None,
                permissions: None,
                communication_disabled_until: None,
            });

            member.user = Some(event.user.clone());
            member.roles.clone_from(&event.roles);
            member.nick.clone_from(&event.nick);
            member.avatar.clone_from(&event.avatar);
            member.banner.clone_from(&event.banner);
            member.joined_at.clone_from(&event.joined_at);
            member.premium_since.clone_from(&event.premium_since);
            if let Some(deaf) = event.deaf {
                member.deaf = deaf;
            }
            if let Some(mute) = event.mute {
                member.mute = mute;
            }
            if let Some(flags) = event.flags {
                member.flags = flags;
            }
            if event.pending.is_some() {
                member.pending = event.pending;
            }
            member
                .communication_disabled_until
                .clone_from(&event.communication_disabled_until);
            member.clone()
        };

        if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
            if let Some(existing) = guild
                .members
                .iter_mut()
                .find(|cached| member_user_id(cached) == Some(event.user.id))
            {
                *existing = updated;
            } else {
                guild.members.push(updated);
            }
        }
    }

    fn update_voice_state(&mut self, voice_state: &VoiceState) {
        let Some(guild_id) = voice_state.guild_id else {
            return;
        };

        if let Some(member) = &voice_state.member {
            self.upsert_member(guild_id, member);
        }

        if voice_state.channel_id.is_none() {
            self.voice_states.remove(&(guild_id, voice_state.user_id));
            if let Some(guild) = self.guilds.get_mut(&guild_id) {
                guild
                    .voice_states
                    .retain(|state| state.user_id != voice_state.user_id);
            }
            return;
        }

        self.voice_states
            .insert((guild_id, voice_state.user_id), voice_state.clone());
        if let Some(guild) = self.guilds.get_mut(&guild_id) {
            if let Some(existing) = guild
                .voice_states
                .iter_mut()
                .find(|state| state.user_id == voice_state.user_id)
            {
                *existing = voice_state.clone();
            } else {
                guild.voice_states.push(voice_state.clone());
            }
        }
    }

    fn upsert_scheduled_event(&mut self, event: &GuildScheduledEvent) {
        self.scheduled_events.insert(event.id, event.clone());
        if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
            if let Some(existing) = guild
                .guild_scheduled_events
                .iter_mut()
                .find(|cached| cached.id == event.id)
            {
                *existing = event.clone();
            } else {
                guild.guild_scheduled_events.push(event.clone());
            }
        }
    }

    fn cache_message(&mut self, message: &Message) {
        self.users.insert(message.author.id, message.author.clone());
        for user in &message.mentions {
            self.users.insert(user.id, user.clone());
        }
        if let Some(channel) = self.channels.get_mut(&message.channel_id) {
            channel.last_message_id = Some(message.id);
        }

        let capacity = self.config.message_capacity;
        if capacity == 0 {
            return;
        }

        let is_new = !self.messages.contains_key(&message.id);
        self.messages.insert(message.id, message.clone());
        if is_new {
            self.message_order.push_back(message.id);
        }

        while self.messages.len() > capacity {
            let Some(oldest) = self.message_order.pop_front() else {
                break;
            };
            self.messages.remove(&oldest);
        }
    }

    fn remove_message(&mut self, message_id: MessageId) {
        self.messages.remove(&message_id);
        for channel in self.channels.values_mut() {
            if channel.last_message_id == Some(message_id) {
                channel.last_message_id = None;
            }
        }
        self.message_order.retain(|cached| *cached != message_id);
    }

    fn merge_partial_user(&mut self, presence: &PresenceUpdate) {
        let partial = &presence.user;
        if let Some(user) = self.users.get_mut(&partial.id) {
            if let Some(username) = &partial.username {
                user.username.clone_from(username);
            }
            if partial.global_name.is_some() {
                user.global_name.clone_from(&partial.global_name);
            }
            if partial.discriminator.is_some() {
                user.discriminator.clone_from(&partial.discriminator);
            }
            if partial.bot.is_some() {
                user.bot = partial.bot;
            }
            if partial.avatar.is_some() {
                user.avatar.clone_from(&partial.avatar);
            }
        } else if let Some(username) = &partial.username {
            self.users.insert(
                partial.id,
                User {
                    id: partial.id,
                    username: username.clone(),
                    global_name: partial.global_name.clone(),
                    discriminator: partial.discriminator.clone(),
                    bot: partial.bot,
                    avatar: partial.avatar.clone(),
                },
            );
        }
    }
}

fn member_user_id(member: &GuildMember) -> Option<UserId> {
    member.user.as_ref().map(|user| user.id)
}

fn is_thread(kind: ChannelType) -> bool {
    matches!(
        kind,
        ChannelType::ANNOUNCEMENT_THREAD | ChannelType::PUBLIC_THREAD | ChannelType::PRIVATE_THREAD
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        gateway::{DispatchEvent, GuildMemberUpdateEvent, TypedDispatchEvent},
        model::{Guild, GuildId, Message, MessageId, UnavailableGuild, UserId},
    };

    use super::{Cache, CacheConfig};

    #[test]
    fn guild_create_seeds_normalized_state_and_delete_cleans_it() {
        let guild: Guild = serde_json::from_value(json!({
            "id":"1",
            "name":"Gloamwire",
            "owner_id":"2",
            "channels":[{"id":"10","type":0,"name":"general"}],
            "members":[{"user":{"id":"2","username":"owner"},"roles":[]}]
        }))
        .expect("guild");
        let mut cache = Cache::default();

        cache.update(&TypedDispatchEvent::GuildCreate(guild));

        assert!(cache.guild(GuildId::new(1)).is_some());
        assert_eq!(
            cache
                .channel(crate::model::ChannelId::new(10))
                .expect("channel")
                .guild_id,
            Some(GuildId::new(1))
        );
        assert!(cache.member(GuildId::new(1), UserId::new(2)).is_some());

        cache.update(&TypedDispatchEvent::GuildDelete(UnavailableGuild {
            id: GuildId::new(1),
            unavailable: false,
        }));

        assert!(cache.guild(GuildId::new(1)).is_none());
        assert!(cache.channel(crate::model::ChannelId::new(10)).is_none());
        assert!(cache.member(GuildId::new(1), UserId::new(2)).is_none());
    }

    #[test]
    fn temporary_guild_unavailability_retains_state() {
        let guild: Guild = serde_json::from_value(json!({
            "id":"1",
            "name":"Gloamwire",
            "owner_id":"2"
        }))
        .expect("guild");
        let mut cache = Cache::default();
        cache.update(&TypedDispatchEvent::GuildCreate(guild));

        cache.update(&TypedDispatchEvent::GuildDelete(UnavailableGuild {
            id: GuildId::new(1),
            unavailable: true,
        }));

        assert!(cache.guild(GuildId::new(1)).is_some());
        assert!(cache.is_guild_unavailable(GuildId::new(1)));
    }

    #[test]
    fn member_update_merges_into_seeded_member() {
        let guild: Guild = serde_json::from_value(json!({
            "id":"1",
            "name":"Gloamwire",
            "owner_id":"2",
            "members":[{"user":{"id":"2","username":"before"},"roles":[]}]
        }))
        .expect("guild");
        let update: GuildMemberUpdateEvent = serde_json::from_value(json!({
            "guild_id":"1",
            "roles":["9"],
            "user":{"id":"2","username":"after"},
            "nick":"wire"
        }))
        .expect("member update");
        let mut cache = Cache::default();
        cache.update(&TypedDispatchEvent::GuildCreate(guild));
        cache.update(&TypedDispatchEvent::GuildMemberUpdate(update));

        let member = cache
            .member(GuildId::new(1), UserId::new(2))
            .expect("member");
        assert_eq!(member.nick.as_deref(), Some("wire"));
        assert_eq!(member.roles[0].get(), 9);
        assert_eq!(
            cache.user(UserId::new(2)).expect("user").username,
            "after"
        );
    }

    #[test]
    fn message_cache_is_bounded() {
        let mut cache = Cache::new(CacheConfig::new().message_capacity(1));
        let first: Message = serde_json::from_value(json!({
            "id":"1",
            "channel_id":"10",
            "author":{"id":"2","username":"user"},
            "content":"first"
        }))
        .expect("first message");
        let second: Message = serde_json::from_value(json!({
            "id":"2",
            "channel_id":"10",
            "author":{"id":"2","username":"user"},
            "content":"second"
        }))
        .expect("second message");

        cache.update(&TypedDispatchEvent::MessageCreate(Box::new(first)));
        cache.update(&TypedDispatchEvent::MessageCreate(Box::new(second)));

        assert!(cache.message(MessageId::new(1)).is_none());
        assert_eq!(
            cache
                .message(MessageId::new(2))
                .expect("newest message")
                .content,
            "second"
        );
    }

    #[test]
    fn raw_dispatch_can_be_typed_and_applied_in_one_step() {
        let mut cache = Cache::new(CacheConfig::new().message_capacity(1));
        let dispatch = DispatchEvent {
            name: "MESSAGE_CREATE".to_owned(),
            sequence: 1,
            data: json!({
                "id":"50",
                "channel_id":"10",
                "author":{"id":"2","username":"user"},
                "content":"cached"
            }),
        };

        let event = cache.update_dispatch(&dispatch).expect("typed dispatch");
        assert!(matches!(event, TypedDispatchEvent::MessageCreate(_)));
        assert_eq!(
            cache
                .message(MessageId::new(50))
                .expect("cached message")
                .content,
            "cached"
        );
    }
}
