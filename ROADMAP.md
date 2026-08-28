# Gloamwire Roadmap

Gloamwire is developed protocol-first: transport correctness and Discord lifecycle rules take priority over endpoint count.

The parity phases below use Discord API v10 and the current Discord Developer Platform documentation as the reference surface. "Parity" means Gloamwire should expose documented bot/application REST resources and Gateway/Webhook Events without sacrificing forward compatibility when Discord adds new fields, values, events, or endpoints.

## Phase 1 — Gateway lifecycle correctness

- [x] Rust 1.98 / Edition 2024 baseline
- [x] Typed Gateway close-code handling
- [x] READY session-state capture (`session_id`, `resume_gateway_url`, sequence)
- [x] Opcode 6 Resume payloads
- [x] Automatic resume/re-identify decisions
- [x] Exponential reconnect backoff with jitter
- [x] Heartbeat ACK enforcement
- [x] Heartbeat latency measurement
- [x] Graceful client shutdown
- [x] Protocol-fixture integration tests for reconnect/resume sequences

## Phase 2 — Rate limiting, Gateway discovery, and sharding

- [x] Route-aware REST rate-limit buckets
- [x] Global/shared rate-limit handling
- [x] Gateway outbound 120-events/60-seconds limiter
- [x] Gateway URL discovery through `/gateway/bot`
- [x] Session-start-limit and Identify concurrency enforcement
- [x] Shard ID/count types and guild-to-shard routing
- [x] Multi-shard manager with isolated shard recovery/shutdown
- [x] Gateway presence, voice-state, member, soundboard, and channel-info send events
- [x] Gateway zlib/zstd transport compression
- [x] Configurable JSON/ETF encoding

## Phase 3 — Typed protocol and core Discord models

- [x] Strong ID types (`GuildId`, `ChannelId`, `MessageId`, `UserId`, and others)
- [x] Typed READY/RESUMED and core dispatch events
- [x] Forward-compatible unknown-event and unknown-value handling
- [x] Guild, channel, thread, member, role, permission, presence, and voice-state models
- [x] Reaction models and dispatches
- [x] Application command, option, context, and handler models
- [x] Interaction core, application-command interaction data, and `INTERACTION_CREATE`
- [x] Component and modal models
- [x] Webhook and invite models plus Gateway update events
- [x] Audit-log and automod models plus Gateway events
- [x] Scheduled-event and poll models plus Gateway events
- [x] Entitlement, SKU, and subscription models plus Gateway events
- [x] Discord permission calculation

## Phase 4 — REST breadth, uploads, and interactions

- [x] Central REST route abstraction with major-parameter bucket identity
- [x] JSON, empty, binary, and header-aware response handling
- [x] Structured Discord validation errors
- [x] Retry classification and safe idempotent retry policy
- [x] Configurable request/connect/pool timeouts
- [x] Full message API
- [x] Multipart attachments and streaming uploads
- [x] Guild, channel, and thread APIs
- [x] Role, member, and moderation APIs
- [x] Webhook, audit-log, invite, and scheduled-event APIs
- [x] Application commands and interaction responses/followups
- [x] Pagination primitives
- [x] OAuth2 support
- [x] CDN URL helpers

## Phase 5 — Reliability, observability, and optional state

- [x] HTTP and Gateway mock integration servers
- [x] Captured/synthetic protocol fixtures
- [x] Optional `tracing` instrumentation without credential leakage
- [x] Optional cache layer
- [x] Event-to-cache synchronization
- [x] Feature flags for transport/model/cache/compression/TLS capabilities
- [x] Fuzz/property tests for protocol parsing and route/rate-limit behavior

## Phase 6 — Advanced subsystems

**Status: Complete**

### Voice transport

- [x] Main-Gateway voice rendezvous (`VOICE_STATE_UPDATE` + `VOICE_SERVER_UPDATE`)
- [x] Voice Gateway v8 Identify, heartbeat, sequence acknowledgement, and Resume
- [x] UDP IP discovery and Select Protocol negotiation
- [x] RTP audio header and sequence/timestamp primitives
- [x] AES-256-GCM and XChaCha20-Poly1305 RTP-size transport encryption
- [x] Voice Gateway/UDP integration fixtures and reconnect orchestration
- [x] Opus integration boundaries and frame pacing
- [x] Discord DAVE/E2EE MLS session lifecycle and media frame encryption

### Scale and framework layers

- [x] Distributed shard ownership/coordination
- [x] Optional high-level command framework kept separate from the low-level protocol core ([gloam-macro-commands](https://github.com/aliceblackrose/gloam-macro-commands))

## Phase 7 — Gateway API parity

**Goal:** every documented bot-relevant Gateway dispatch should have a typed representation while retaining `Unknown` fallback behavior for future Discord additions.

### Identify and connection configuration

- [ ] Configurable Identify `large_threshold`
- [ ] Configurable initial Identify presence
- [ ] Forward-compatible Gateway Identify `capabilities` bitfield
- [ ] Typed capability constants for documented Discord capability bits
- [ ] Tests proving omitted optional Identify fields preserve the current wire payload
- [ ] Tests for JSON and ETF serialization of all Identify options

### Message and channel dispatches

- [ ] `MESSAGE_UPDATE`
- [ ] `CHANNEL_PINS_UPDATE`
- [ ] `TYPING_START`
- [ ] Channel-info response dispatches for `REQUEST_CHANNEL_INFO`
- [ ] Voice-channel status/start-time update dispatches
- [ ] Voice-channel effect dispatches

### Thread dispatches

- [ ] `THREAD_LIST_SYNC`
- [ ] `THREAD_MEMBER_UPDATE`
- [ ] `THREAD_MEMBERS_UPDATE`
- [ ] Cache synchronization for thread-member and thread-list events

### Guild and integration dispatches

- [ ] `GUILD_BAN_ADD`
- [ ] `GUILD_BAN_REMOVE`
- [ ] `GUILD_EMOJIS_UPDATE`
- [ ] `GUILD_STICKERS_UPDATE`
- [ ] `GUILD_INTEGRATIONS_UPDATE`
- [ ] `INTEGRATION_CREATE`
- [ ] `INTEGRATION_UPDATE`
- [ ] `INTEGRATION_DELETE`
- [ ] `APPLICATION_COMMAND_PERMISSIONS_UPDATE`

### Stage and soundboard dispatches

- [ ] `STAGE_INSTANCE_CREATE`
- [ ] `STAGE_INSTANCE_UPDATE`
- [ ] `STAGE_INSTANCE_DELETE`
- [ ] Typed soundboard-sound create/update/delete dispatches
- [ ] Typed soundboard-sounds response dispatches
- [ ] Cache synchronization for stage-instance and soundboard state where caching is useful

### Voice rendezvous completeness

- [ ] Public typed `VOICE_SERVER_UPDATE` dispatch instead of relying only on rendezvous internals
- [ ] Verify all documented main-Gateway voice-related dispatches are typed
- [ ] Fixtures for voice server moves, endpoint changes, and token refresh/reconnect behavior

### Gateway parity verification

- [ ] Maintain a machine-readable inventory of documented Gateway dispatch names
- [ ] Test that every inventory entry either maps to a typed event or is explicitly documented as intentionally raw
- [ ] Preserve unknown dispatch payloads losslessly
- [ ] Preserve unknown opcode payloads losslessly

## Phase 8 — Specialized REST resource parity

**Goal:** cover Discord's long-tail REST resources using the same resource-per-module organization as the existing HTTP client.

### Emoji resource (`src/http/emoji.rs`)

- [ ] Typed full Emoji model separate from lightweight `PartialEmoji`
- [ ] List application emojis
- [ ] Get application emoji
- [ ] Create application emoji
- [ ] Modify application emoji
- [ ] Delete application emoji
- [ ] List guild emojis
- [ ] Get guild emoji
- [ ] Create guild emoji
- [ ] Modify guild emoji
- [ ] Delete guild emoji
- [ ] Audit-log reason support on guild mutation endpoints

### Sticker resource (`src/http/sticker.rs`)

- [ ] Sticker, sticker item, sticker pack, and sticker-format models
- [ ] Get sticker
- [ ] List Nitro sticker packs where exposed by Discord
- [ ] List guild stickers
- [ ] Get guild sticker
- [ ] Create guild sticker with multipart upload
- [ ] Modify guild sticker
- [ ] Delete guild sticker
- [ ] Audit-log reason support

### Soundboard resource (`src/http/soundboard.rs`)

- [ ] Soundboard sound model
- [ ] List default soundboard sounds
- [ ] List guild soundboard sounds
- [ ] Get guild soundboard sound
- [ ] Create guild soundboard sound
- [ ] Modify guild soundboard sound
- [ ] Delete guild soundboard sound
- [ ] Send/play a soundboard sound where exposed by Discord
- [ ] Audio payload validation helpers without embedding an audio codec
- [ ] Audit-log reason support

### Stage Instance resource (`src/http/stage_instance.rs`)

- [ ] Stage Instance model
- [ ] Create Stage Instance
- [ ] Get Stage Instance
- [ ] Modify Stage Instance
- [ ] Delete Stage Instance
- [ ] Privacy-level and scheduled-event linkage models
- [ ] Audit-log reason support where applicable

### Guild Template resource (`src/http/template.rs`)

- [ ] Guild Template model
- [ ] Get guild template by code
- [ ] Create guild from template
- [ ] List guild templates
- [ ] Create guild template
- [ ] Sync guild template
- [ ] Modify guild template
- [ ] Delete guild template

### Poll REST completion (`src/http/poll.rs` or message integration)

- [ ] Get poll answer voters with pagination
- [ ] Expire/end a poll
- [ ] Typed poll-voter response model
- [ ] Preserve poll creation inside the message API

### Monetization REST (`src/http/monetization.rs`)

#### SKU

- [ ] List application SKUs
- [ ] Verify all current SKU fields/flags are represented

#### Entitlements

- [ ] List entitlements with typed query filters and pagination
- [ ] Get entitlement
- [ ] Consume entitlement
- [ ] Create test entitlement
- [ ] Delete test entitlement
- [ ] Support user, guild, SKU, and entitlement filters documented by Discord

#### Subscriptions

- [ ] List SKU subscriptions with pagination
- [ ] Get SKU subscription
- [ ] Verify subscription status and renewal fields against current Discord schema

### Application resource (`src/http/application.rs`)

- [ ] Full Application model separate from partial READY/OAuth representations
- [ ] Get current application
- [ ] Edit current application where supported
- [ ] Application integration-type configuration models
- [ ] Installation parameter/configuration models
- [ ] Application flags with forward-compatible unknown-bit retention
- [ ] Application asset/metadata endpoints that are part of the public REST API

### Application Role Connection Metadata (`src/http/role_connection.rs`)

- [ ] Role connection metadata model and metadata type enum/newtype
- [ ] Get application role connection metadata records
- [ ] Update application role connection metadata records
- [ ] User application role connection model
- [ ] Get/update current user's application role connection for OAuth2 Bearer clients

### Application Identity Profile (`src/http/identity_profile.rs`)

- [ ] Application Identity Profile models
- [ ] Profile/game-stat update endpoints documented by Discord
- [ ] Authentication mode required by the resource
- [ ] Forward-compatible profile field/value handling

### User REST completion (`src/http/user.rs`)

- [ ] Get user by ID
- [ ] Modify current user
- [ ] Current-user guild endpoints supported by bot/OAuth contexts
- [ ] Current-user guild-member endpoints supported by OAuth contexts
- [ ] User connection models/endpoints for OAuth2 Bearer clients
- [ ] Explicit authentication requirements per endpoint

### Voice REST completion (`src/http/voice.rs`)

- [ ] List/get current voice regions where still exposed by Discord
- [ ] Guild voice-region endpoints where documented
- [ ] Voice-channel status REST helpers not already covered by channel APIs
- [ ] Keep REST voice resources separate from the Voice Gateway/media transport subsystem

### Lobby resource (`src/http/lobby.rs`)

- [ ] Determine bot-token/public-REST applicability versus Social SDK-only applicability
- [ ] Model Lobby and Lobby Member objects if usable through Discord's public REST API
- [ ] Implement documented lobby CRUD/member/linking endpoints that fit Gloamwire's authentication model
- [ ] Feature-gate lobby support if it requires non-bot application credentials or substantially different lifecycle rules

## Phase 9 — Core REST exhaustiveness and model parity

**Goal:** move from "practical REST coverage" to documented endpoint and object completeness for the resources Gloamwire already supports.

### Guild completeness

- [ ] Audit the Guild model against every current documented field
- [ ] Replace raw guild emoji/sticker values with typed models
- [ ] Guild preview model parity
- [ ] Guild widget/settings endpoints
- [ ] Vanity URL endpoints
- [ ] Welcome screen endpoints
- [ ] Onboarding endpoints/models where publicly supported
- [ ] MFA/security-level configuration endpoints where publicly supported
- [ ] Integration endpoints not covered by Phase 7 dispatch work
- [ ] Current-user guild management endpoints where authentication permits them

### Channel and thread completeness

- [ ] Audit every channel type and field against current Discord documentation
- [ ] Follow/news-channel endpoint parity
- [ ] Thread-member endpoint parity
- [ ] Forum/media-channel field and tag parity
- [ ] Voice/video channel specialized fields
- [ ] Channel permission overwrite edge cases
- [ ] Pin APIs and pin metadata parity

### Message completeness

- [ ] Audit all current Message fields and message types
- [ ] Message snapshots/forwarding/reference fields
- [ ] Role-subscription and monetization message payloads
- [ ] Call/message-call fields
- [ ] Interaction metadata fields on messages
- [ ] Components V2 field parity as Discord evolves
- [ ] Attachment metadata parity
- [ ] Reaction endpoint parity including burst/super-reaction behavior

### Member, role, and moderation completeness

- [ ] Audit Guild Member fields and flags
- [ ] Audit Role fields, tags, colors, icons, and flags
- [ ] Role connection/subscription-related role metadata
- [ ] Ban/prune/bulk-ban endpoint parity
- [ ] Timeout and moderation edge-case validation
- [ ] Auto Moderation action/trigger parity

### Invite completeness

- [ ] Audit invite models against all invite types and target modes
- [ ] Scheduled-event invite fields
- [ ] Community/target-user invite behavior
- [ ] Invite flags and expiration fields

### Webhook REST completeness

- [ ] Audit incoming, channel-follower, and application webhook models
- [ ] Execute/edit/delete webhook message endpoint parity
- [ ] Slack/GitHub-compatible webhook endpoints if still publicly documented
- [ ] Thread/forum parameters and attachment behavior

### Application commands and interactions

- [ ] Audit all command option/choice/localization fields
- [ ] Entry-point/application command handler parity
- [ ] Command permission endpoint parity
- [ ] Interaction callback type parity
- [ ] Interaction callback resource/activity-instance parity
- [ ] Components/modal payload parity
- [ ] Attachment support in all applicable interaction callbacks/followups

### OAuth2 completeness

- [ ] Enumerated/forward-compatible Discord OAuth2 scopes
- [ ] Authorization Code + PKCE support
- [ ] Client Credentials grant where Discord supports it
- [ ] Refresh/revoke parity
- [ ] OAuth2 current-user endpoints that require Bearer authentication
- [ ] Webhook/incoming-webhook authorization flow parity
- [ ] Authentication abstraction that can support Bot and Bearer clients without leaking tokens

## Phase 10 — HTTP Webhook Events

**Goal:** support Discord's event-delivery-over-HTTP surface separately from ordinary outgoing webhooks.

- [ ] Webhook Event envelope model
- [ ] Typed Webhook Event payloads for every documented event type
- [ ] Forward-compatible unknown Webhook Event fallback
- [ ] Ed25519 request signature verification helpers
- [ ] Timestamp/replay-window validation helpers
- [ ] Event subscription/configuration REST endpoints if exposed publicly
- [ ] Distinguish outgoing Discord webhooks from incoming Webhook Events in module naming/API design
- [ ] Shared event models with Gateway where wire schemas are identical
- [ ] Separate event models where Gateway and Webhook Event schemas differ
- [ ] Fixtures using captured/synthetic signed HTTP event requests

## Phase 11 — Cache and state parity

- [ ] `MESSAGE_UPDATE` cache mutation/merge semantics
- [ ] Channel pin state where useful
- [ ] Thread sync/member state
- [ ] Emoji and sticker guild state
- [ ] Integration state only if it can be kept correct from available events
- [ ] Stage Instance state
- [ ] Soundboard state
- [ ] Entitlement/subscription state policy
- [ ] Explicit rules for partial Gateway objects versus complete REST objects
- [ ] Cache invalidation semantics for guild unavailability, reconnect, and resharding
- [ ] Tests proving out-of-order or partial events cannot silently corrupt normalized state

## Phase 12 — API parity infrastructure

**Goal:** make future Discord changes discoverable instead of relying on occasional manual audits.

- [ ] Maintain a checked-in Discord resource/endpoint inventory
- [ ] Maintain a checked-in Gateway dispatch inventory
- [ ] Maintain a checked-in Gateway send-event/opcode inventory
- [ ] Maintain a checked-in Webhook Event inventory
- [ ] Map each inventory entry to implementation status and source module
- [ ] Add CI checks that fail when the inventory and parity manifest disagree
- [ ] Add schema fixtures for every typed resource/event family
- [ ] Add serialization golden tests for every mutation/request body
- [ ] Add route identity/rate-limit-major tests for every REST route family
- [ ] Add audit-log reason tests for every endpoint that accepts `X-Audit-Log-Reason`
- [ ] Add pagination tests for every paginated endpoint
- [ ] Add multipart tests for every upload endpoint
- [ ] Add compatibility tests for unknown enum values, flags, fields, events, and opcodes
- [ ] Document the Discord documentation/change-log date used for each parity audit

## Phase 13 — API quality and ergonomics before 1.0

- [ ] Standard naming conventions across REST methods (`get_*`, `list_*`, `create_*`, `modify_*`, `delete_*`)
- [ ] Consistent typed query/request structs instead of ad-hoc argument lists
- [ ] Consistent audit-log reason parameter strategy
- [ ] Consistent pagination primitives across all list endpoints
- [ ] Consistent response wrappers when headers/status metadata matter
- [ ] Avoid public `serde_json::Value` where Discord has a stable documented object
- [ ] Preserve `Value`/extra-field escape hatches where Discord schemas are intentionally open-ended
- [ ] Public API review for unnecessary allocation/cloning in hot Gateway/voice paths
- [ ] Public API review for `#[non_exhaustive]` and forward-compatible numeric/string newtypes
- [ ] Fix package/user-agent repository metadata to point at `aliceblackrose/gloamwire`
- [ ] Complete rustdoc examples for every major resource family
- [ ] Add examples for REST, Gateway, cache, sharding, OAuth2, interactions, voice, DAVE, and specialized resources

## Scope boundaries

Gloamwire's core target is Discord's public bot/application HTTP API, Gateway, Voice Gateway/media transport, OAuth2, and HTTP Webhook Events.

The following Discord platform surfaces are **not required for core API parity** because they are SDK/client-integration products rather than the bot/application transport API Gloamwire is designed around:

- Discord Social SDK
- Embedded App SDK / Activities client SDK
- Local Discord RPC
- Certified Device SDK/integration surface

If Rust support for those surfaces is added later, it should live behind clearly separated feature flags or companion crates so the core `model`/`transport` dependency graph remains suitable for bots and services.

## 1.0 parity gate

A `1.0` release should require all of the following:

- [ ] Stable Gateway lifecycle and reconnection behavior
- [ ] Correct REST and Gateway rate limiting
- [ ] Typed coverage for all documented bot-relevant Gateway dispatches
- [ ] Complete documented REST coverage for core and specialized bot/application resources
- [ ] HTTP Webhook Events support
- [ ] Voice Gateway/media transport and DAVE behavior covered by integration fixtures
- [ ] Forward-compatible unknown event/opcode/value/flag handling
- [ ] Strong integration, property, and protocol-fixture test coverage
- [ ] A reproducible Discord API parity audit with no undocumented omissions

The `0.x` line may continue evolving APIs while these parity phases are completed. Endpoint count alone is not a release criterion: protocol correctness, rate-limit correctness, forward compatibility, and reliable lifecycle behavior remain higher priority.