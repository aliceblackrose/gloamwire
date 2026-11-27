//! Optional in-memory state derived from typed Discord Gateway dispatches.
//!
//! The cache is intentionally not internally synchronized. Applications with
//! multiple reader/writer tasks can wrap it in their synchronization primitive
//! of choice without Gloamwire imposing a locking strategy.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    Result,
    gateway::{
        DispatchEvent, GatewayEvent, GuildMemberUpdateEvent, ShardCount, ShardEvent, ShardId,
        ThreadListSyncEvent, TypedDispatchEvent, shard_for_guild,
    },
    model::{
        Channel, ChannelId, ChannelType, Guild, GuildId, GuildMember, GuildMemberFlags,
        GuildScheduledEvent, Message, MessageId, PresenceUpdate, Role, RoleId, ScheduledEventId,
        User, UserId, VoiceState,
    },
};

/// Configuration for Gloamwire's optional in-memory cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheConfig {
    message_capacity: usize,
    user_capacity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self { Self::new() }
}

impl CacheConfig {
    /// Creates cache configuration with message caching disabled.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            message_capacity: 0,
            user_capacity: 100_000,
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

    /// Caps the standalone user lookup cache (at least one, for the current user).
    /// Member and retained message objects still contain their embedded user data.
    #[must_use]
    pub const fn user_capacity(mut self, capacity: usize) -> Self {
        self.user_capacity = if capacity == 0 { 1 } else { capacity };
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
    user_order: VecDeque<UserId>,
    membership: HashMap<(GuildId, UserId), bool>,
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
    shard_sequences: HashMap<ShardId, u64>,
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
            user_order: VecDeque::new(),
            membership: HashMap::new(),
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
            shard_sequences: HashMap::new(),
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
        self.user_order.clear();
        self.membership.clear();
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
        self.shard_sequences.clear();
    }

    /// Applies one already-typed Gateway dispatch to the cache.
    pub fn update(&mut self, event: &TypedDispatchEvent) {
        match event {
            TypedDispatchEvent::Ready(ready) => {
                self.clear();
                self.current_user_id = Some(ready.user.id);
                self.remember_user(ready.user.clone());
                self.unavailable_guilds
                    .extend(ready.guilds.iter().map(|guild| guild.id));
            }
            TypedDispatchEvent::Resumed => {}
            TypedDispatchEvent::GuildCreate(guild) => self.seed_guild(guild),
            TypedDispatchEvent::GuildUnavailable(guild) => {
                self.unavailable_guilds.insert(guild.id);
                if let Some(cached) = self.guilds.get_mut(&guild.id) {
                    cached.unavailable = Some(true);
                }
            }
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
            TypedDispatchEvent::ThreadListSync(event) => self.sync_threads(event),
            TypedDispatchEvent::GuildMemberAdd(event) => {
                if let Some(user) = &event.member.user {
                    let prior = self.membership.get(&(event.guild_id, user.id)).copied();
                    let complete = self.guilds.get(&event.guild_id).is_some_and(|guild| {
                        guild.member_count == Some(guild.members.len() as u64)
                    });
                    if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
                        match prior {
                            Some(true) => {}
                            Some(false) => {
                                if let Some(count) = &mut guild.member_count {
                                    *count = count.saturating_add(1);
                                }
                            }
                            None if complete => {
                                if let Some(count) = &mut guild.member_count {
                                    *count = count.saturating_add(1);
                                }
                            }
                            None => {
                                // Discord may send an add for an existing member.
                                // An incomplete snapshot cannot distinguish this.
                                guild.approximate_member_count = guild.member_count.or(guild.approximate_member_count);
                                guild.member_count = None;
                            }
                        }
                    }
                }
                self.upsert_member(event.guild_id, &event.member);
            }
            TypedDispatchEvent::GuildMemberUpdate(event) => self.merge_member_update(event),
            TypedDispatchEvent::GuildMemberRemove(event) => {
                let prior = self.membership.insert((event.guild_id, event.user.id), false);
                self.members.remove(&(event.guild_id, event.user.id));
                self.remember_user(event.user.clone());
                if let Some(guild) = self.guilds.get_mut(&event.guild_id) {
                    guild
                        .members
                        .retain(|member| member_user_id(member) != Some(event.user.id));
                    guild
                        .voice_states
                        .retain(|state| state.user_id != event.user.id);
                    if prior != Some(false) && let Some(member_count) = &mut guild.member_count {
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
                self.seed_presences(event.guild_id, &event.presences);
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
            TypedDispatchEvent::MessageUpdate(event) => {
                if let Some(existing) = self.messages.get_mut(&event.id)
                    && let Ok(updated) = event.apply(existing)
                {
                    *existing = updated;
                }
            }
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
                self.remember_user(user.clone());
            }
            _ => {}
        }
    }

    /// Parses a raw dispatch, applies it to the cache, and returns the typed event.
    pub fn update_dispatch(&mut self, dispatch: &DispatchEvent) -> Result<TypedDispatchEvent> {
        let event = dispatch.typed()?;
        self.apply_checked(&event)?;
        Ok(event)
    }

    fn apply_checked(&mut self, event: &TypedDispatchEvent) -> Result<()> {
        if let TypedDispatchEvent::MessageUpdate(patch) = event {
            if let Some(existing) = self.messages.get_mut(&patch.id) {
                *existing = patch.apply(existing)?;
            }
        } else {
            self.update(event);
        }
        Ok(())
    }

    /// Applies a dispatch from a shared multi-shard event stream. READY clears
    /// only the issuing shard, and repeated sequence numbers are not applied.
    /// Use the same shard count for the lifetime of this cache.
    pub fn update_shard_event(
        &mut self,
        event: &ShardEvent,
        shard_count: ShardCount,
    ) -> Result<Option<TypedDispatchEvent>> {
        let GatewayEvent::Dispatch(dispatch) = &event.event else {
            return Ok(None);
        };
        let typed = dispatch.typed()?;
        if let TypedDispatchEvent::Ready(ready) = &typed {
            let guilds: HashSet<_> = self
                .guilds
                .keys()
                .chain(self.unavailable_guilds.iter())
                .copied()
                .filter(|id| shard_for_guild(*id, shard_count) == event.shard_id)
                .collect();
            for guild in guilds {
                self.remove_guild(guild);
            }
            if event.shard_id.get() == 0 {
                let channels: Vec<_> = self
                    .channels
                    .values()
                    .filter(|channel| channel.guild_id.is_none())
                    .map(|channel| channel.id)
                    .collect();
                for channel in channels {
                    self.remove_channel(channel);
                }
                let messages: Vec<_> = self
                    .messages
                    .values()
                    .filter(|message| message.guild_id.is_none())
                    .map(|message| message.id)
                    .collect();
                for message in messages {
                    self.remove_message(message);
                }
            }
            self.current_user_id = Some(ready.user.id);
            self.remember_user(ready.user.clone());
            self.unavailable_guilds
                .extend(ready.guilds.iter().map(|guild| guild.id));
        } else {
            if self
                .shard_sequences
                .get(&event.shard_id)
                .is_some_and(|sequence| *sequence >= dispatch.sequence)
            {
                return Ok(None);
            }
            self.apply_checked(&typed)?;
        }
        self.shard_sequences
            .insert(event.shard_id, dispatch.sequence);
        Ok(Some(typed))
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

    fn remember_user(&mut self, user: User) {
        let id = user.id;
        if self.users.insert(id, user).is_none() {
            self.user_order.push_back(id);
        }
        while self.users.len() > self.config.user_capacity {
            let Some(oldest) = self.user_order.pop_front() else { break };
            if Some(oldest) == self.current_user_id {
                self.user_order.push_back(oldest);
            } else {
                self.users.remove(&oldest);
            }
        }
    }

    fn seed_presences(&mut self, guild_id: GuildId, presences: &[serde_json::Value]) {
        for raw in presences {
            let mut raw = raw.clone();
            if let Some(object) = raw.as_object_mut() {
                object.insert("guild_id".into(), serde_json::json!(guild_id));
            }
            if let Ok(presence) = serde_json::from_value::<PresenceUpdate>(raw) {
                self.merge_partial_user(&presence);
                self.presences.insert((guild_id, presence.user.id), presence);
            }
        }
    }

    fn sync_threads(&mut self, event: &ThreadListSyncEvent) {
        let supplied: HashSet<_> = event.threads.iter().map(|thread| thread.id).collect();
        let obsolete: Vec<_> = self.channels.values().filter(|channel| {
            channel.guild_id == Some(event.guild_id) && is_thread(channel.kind)
                && !channel.thread_metadata.as_ref().is_some_and(|metadata| metadata.archived)
                && event.channel_ids.as_ref().is_none_or(|parents| channel.parent_id.is_some_and(|id| parents.contains(&id)))
                && !supplied.contains(&channel.id)
        }).map(|channel| channel.id).collect();
        for id in obsolete {
            self.remove_channel(id);
        }
        for thread in &event.threads {
            let mut thread = thread.clone();
            thread.guild_id = Some(event.guild_id);
            self.upsert_channel(&thread);
        }
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
                self.remember_user(user.clone());
                self.members.insert((guild_id, user.id), member.clone());
                self.membership.insert((guild_id, user.id), true);
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

        self.seed_presences(guild_id, &cached_guild.presences);
        self.guilds.insert(guild_id, cached_guild);
    }

    fn update_guild(&mut self, guild: &Guild) {
        self.unavailable_guilds.remove(&guild.id);
        let mut updated = guild.clone();
        updated.unavailable = Some(false);

        if let Some(existing) = self.guilds.get(&guild.id) {
            // GUILD_UPDATE includes the guild role collection, but not
            // Gateway-only child collections or member counts.
            updated.member_count = guild.member_count.or(existing.member_count);
            updated.channels = existing.channels.clone();
            updated.threads = existing.threads.clone();
            updated.members = existing.members.clone();
            updated.voice_states = existing.voice_states.clone();
            updated.presences = existing.presences.clone();
            updated.guild_scheduled_events = existing.guild_scheduled_events.clone();
        }

        self.roles.retain(|(guild_id, _), _| *guild_id != guild.id);
        for role in &updated.roles {
            self.roles.insert((guild.id, role.id), role.clone());
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
        self.membership.retain(|(cached_guild_id, _), _| *cached_guild_id != guild_id);
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
        self.remember_user(user.clone());
        self.members.insert((guild_id, user.id), member.clone());
        self.membership.insert((guild_id, user.id), true);

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
        self.remember_user(event.user.clone());
        let key = (event.guild_id, event.user.id);
        self.membership.insert(key, true);
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
            if let Some(roles) = &event.roles {
                member.roles.clone_from(roles);
            }
            event.nick.apply_to(&mut member.nick);
            event.avatar.apply_to(&mut member.avatar);
            event.banner.apply_to(&mut member.banner);
            event.joined_at.apply_to(&mut member.joined_at);
            event.premium_since.apply_to(&mut member.premium_since);
            if let Some(deaf) = event.deaf {
                member.deaf = deaf;
            }
            if let Some(mute) = event.mute {
                member.mute = mute;
            }
            if let Some(flags) = event.flags {
                member.flags = flags;
            }
            event.pending.apply_to(&mut member.pending);
            event.communication_disabled_until.apply_to(&mut member.communication_disabled_until);
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
        self.remember_user(message.author.clone());
        for user in &message.mentions {
            self.remember_user(user.clone());
        }
        if let Some(channel) = self.channels.get_mut(&message.channel_id) {
            channel.last_message_id = Some(message.id);
            if let Some(guild_id) = channel.guild_id
                && let Some(guild) = self.guilds.get_mut(&guild_id)
            {
                for cached in guild.channels.iter_mut().chain(guild.threads.iter_mut()) {
                    if cached.id == message.channel_id {
                        cached.last_message_id = Some(message.id);
                    }
                }
            }
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
        for guild in self.guilds.values_mut() {
            for channel in guild.channels.iter_mut().chain(guild.threads.iter_mut()) {
                if channel.last_message_id == Some(message_id) {
                    channel.last_message_id = None;
                }
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
            self.remember_user(User {
                    id: partial.id,
                    username: username.clone(),
                    global_name: partial.global_name.clone(),
                    discriminator: partial.discriminator.clone(),
                    bot: partial.bot,
                    avatar: partial.avatar.clone(),
                });
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
        assert_eq!(cache.user(UserId::new(2)).expect("user").username, "after");
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
