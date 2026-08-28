# Gloamwire Roadmap

Gloamwire is developed protocol-first: transport correctness and Discord lifecycle rules take priority over endpoint count.

The parity phases below use Discord API v10 and the current Discord Developer Platform documentation as the reference surface. "Parity" means Gloamwire should expose documented bot/application REST resources, interactions, Gateway events, Voice Gateway/media transport, OAuth2, and Webhook Events without sacrificing forward compatibility when Discord adds new fields, values, events, or endpoints.

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

**Goal:** every documented bot-relevant Gateway send/receive behavior should have a typed representation while retaining raw fallback behavior for future Discord additions.

### Identify and connection configuration

- [ ] Configurable Identify `compress` field, clearly distinguished from Gateway URL transport-compression modes
- [ ] Configurable Identify `large_threshold`
- [ ] Configurable initial Identify presence
- [ ] Forward-compatible Gateway Identify `capabilities` bitfield
- [ ] Typed capability constants for documented Discord capability bits
- [ ] `CHANNEL_OBFUSCATION` opt-in support while Discord exposes it as a testing capability
- [ ] Tests proving omitted optional Identify fields preserve the current wire payload
- [ ] Tests for JSON and ETF serialization of all Identify options

### Gateway rate-limit receive behavior

- [ ] Typed `RATE_LIMITED` receive event
- [ ] Typed `opcode`, `retry_after`, and opcode-specific `meta` payloads
- [ ] Request Guild Members opcode-8 rate-limit metadata (`guild_id`, optional `nonce`)
- [ ] Prevent/reject retry of an affected opcode until Discord's `retry_after` has elapsed
- [ ] Fixtures for Request Guild Members throttling and recovery
- [ ] Keep the existing general outbound 120-events/60-seconds limiter separate from opcode-specific rate limits

### Message and channel dispatches

- [ ] `MESSAGE_UPDATE`
- [ ] `CHANNEL_PINS_UPDATE`
- [ ] `TYPING_START`
- [ ] Typed `CHANNEL_INFO` response for `REQUEST_CHANNEL_INFO`
- [ ] `VOICE_CHANNEL_STATUS_UPDATE`
- [ ] `VOICE_CHANNEL_START_TIME_UPDATE`
- [ ] `VOICE_CHANNEL_EFFECT_SEND`

### Channel obfuscation readiness

- [ ] Model obfuscated Gateway channel payloads without pretending they are complete normal channel objects
- [ ] Ensure guild/channel caches can safely hold or ignore obfuscated channel metadata
- [ ] Ensure permission calculations do not accidentally grant access because an obfuscated channel was observed
- [ ] Tests for capability-opted-in obfuscation before Discord's planned November 16, 2026 rollout to all bots
- [ ] Document the difference between Gateway-obfuscated channels and `GET /guilds/{guild_id}/channels`, which omits invisible channels

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
- [ ] `GUILD_SOUNDBOARD_SOUND_CREATE`
- [ ] `GUILD_SOUNDBOARD_SOUND_UPDATE`
- [ ] `GUILD_SOUNDBOARD_SOUND_DELETE`
- [ ] `GUILD_SOUNDBOARD_SOUNDS_UPDATE`
- [ ] `SOUNDBOARD_SOUNDS`
- [ ] Cache synchronization for stage-instance and soundboard state where caching is useful

### Voice rendezvous completeness

- [ ] Public typed `VOICE_SERVER_UPDATE` dispatch instead of relying only on rendezvous internals
- [ ] Verify all documented main-Gateway voice-related dispatches are typed
- [ ] Fixtures for voice server moves, endpoint changes, and token refresh/reconnect behavior

### Gateway parity verification

- [ ] Maintain a machine-readable inventory of documented Gateway dispatch names
- [ ] Maintain a machine-readable inventory of documented Gateway send events/opcodes
- [ ] Test that every inventory entry either maps to a typed API or is explicitly documented as intentionally raw
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

- [ ] Sticker, Sticker Item, Sticker Pack, and sticker-format models
- [ ] Get Sticker
- [ ] List Sticker Packs
- [ ] Get Sticker Pack
- [ ] List Guild Stickers
- [ ] Get Guild Sticker
- [ ] Create Guild Sticker with multipart upload
- [ ] Modify Guild Sticker
- [ ] Delete Guild Sticker
- [ ] Audit-log reason support

### Soundboard resource (`src/http/soundboard.rs`)

- [ ] Soundboard Sound model
- [ ] List Default Soundboard Sounds
- [ ] List Guild Soundboard Sounds
- [ ] Get Guild Soundboard Sound
- [ ] Create Guild Soundboard Sound
- [ ] Modify Guild Soundboard Sound
- [ ] Delete Guild Soundboard Sound
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

- [ ] Guild Template model, including serialized partial-guild snapshot behavior
- [ ] Get Guild Template
- [ ] Get Guild Templates
- [ ] Create Guild Template
- [ ] Sync Guild Template
- [ ] Modify Guild Template
- [ ] Delete Guild Template

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
- [ ] Install Params and installation-context/configuration models
- [ ] Application flags with forward-compatible unknown-bit retention
- [ ] Get Current Application
- [ ] Edit Current Application
- [ ] Activity Instance model and Activity Location model
- [ ] Get Application Activity Instance
- [ ] Application webhook-event configuration fields exposed through Edit Current Application

### Application Role Connection Metadata (`src/http/role_connection.rs`)

- [ ] Role connection metadata model and metadata type enum/newtype
- [ ] Get application role connection metadata records
- [ ] Update application role connection metadata records
- [ ] User application role connection model
- [ ] Get Current User Application Role Connection
- [ ] Update Current User Application Role Connection
- [ ] Delete Current User Application Role Connection
- [ ] Explicit OAuth2 Bearer authentication/scopes on user-side role-connection calls

### Application Identity Profile (`src/http/identity_profile.rs`)

- [ ] Application Identity model
- [ ] Application Identity Profile model
- [ ] Profile Data, Primary Profile Data, Dynamic Field, and Media models
- [ ] Forward-compatible provider and dynamic-field value handling
- [ ] Update Application Identity Profile
- [ ] Get Application Identity Profile
- [ ] Get Application Identities by User ID
- [ ] Get Application Identities by External ID
- [ ] Delete Application Identity
- [ ] Model full-replacement semantics for the profile `data` field
- [ ] Enforce/document current Discord profile size/count/string limits without preventing forward-compatible reads
- [ ] Explicit bot-token plus target-user OAuth authorization requirements

### User REST completion (`src/http/user.rs`)

- [ ] Complete User model audit
- [ ] Avatar Decoration Data model
- [ ] Collectibles and Nameplate models
- [ ] Connection model
- [ ] Get User by ID
- [ ] Modify Current User
- [ ] Get Current User Guilds
- [ ] Get Current User Guild Member
- [ ] Leave Guild
- [ ] Create DM
- [ ] Create Group DM
- [ ] Get Current User Connections
- [ ] Explicit Bot/Bearer authentication and OAuth2 scope requirements per endpoint

### Voice REST completion (`src/http/voice.rs`)

- [ ] Voice Region model parity
- [ ] List Voice Regions
- [ ] Get Guild Voice Regions through the guild resource
- [ ] Get Current User Voice State
- [ ] Get User Voice State
- [ ] Modify Current User Voice State
- [ ] Modify User Voice State
- [ ] Keep REST voice resources separate from the Voice Gateway/media transport subsystem

### Lobby resource (`src/http/lobby.rs`)

- [ ] Lobby model
- [ ] Lobby Member model, including current `additional_name`, metadata, and flags fields
- [ ] Lobby Message model and moderation metadata
- [ ] Correct optional-vs-nullable update semantics for Lobby and Lobby Member fields
- [ ] Create Lobby
- [ ] Create or Join Lobby
- [ ] Get Lobby
- [ ] Modify Lobby
- [ ] Delete Lobby
- [ ] Add a Member to a Lobby
- [ ] Bulk Update Lobby Members
- [ ] Remove a Member from a Lobby
- [ ] Leave Lobby
- [ ] Link Channel to Lobby
- [ ] Unlink Channel from Lobby
- [ ] Send Lobby Message
- [ ] Get Lobby Messages
- [ ] Update Lobby Message Moderation Metadata
- [ ] Create Lobby Channel Invite for Self
- [ ] Create Lobby Channel Invite for User
- [ ] Model/document Lobby development rate limits
- [ ] Support the Bot/Bearer authentication modes documented per Lobby endpoint

## Phase 9 — Core REST exhaustiveness and model parity

**Goal:** move from "practical REST coverage" to documented endpoint, object, authentication, and request-semantics completeness for resources Gloamwire already supports.

### Guild completeness

- [ ] Audit the Guild model against every current documented field
- [ ] Replace raw guild emoji/sticker values with typed models
- [ ] Guild Preview model parity
- [ ] Integration, Integration Account, and Integration Application models
- [ ] Membership Screening and Incidents Data models
- [ ] Get Guild Role Member Counts
- [ ] Guild widget/settings endpoints, including widget image binary responses
- [ ] Vanity URL endpoints
- [ ] Welcome Screen endpoints
- [ ] Onboarding endpoints/models
- [ ] Get Guild Voice Regions
- [ ] Integration endpoints not covered by Phase 7 dispatch work
- [ ] Modify Guild Incident Actions, including explicit `null` disable semantics
- [ ] Current-user guild management endpoints where authentication permits them

### Channel and thread completeness

- [ ] Audit every channel type and field against current Discord documentation
- [ ] Follow/news-channel endpoint parity
- [ ] Thread-member endpoint parity
- [ ] Forum/media-channel field and tag parity
- [ ] Voice/video channel specialized fields
- [ ] Channel permission-overwrite edge cases
- [ ] Pin APIs and pin metadata parity
- [ ] Partial/obfuscated channel representations where Discord intentionally omits fields

### Message completeness

- [ ] Audit all current Message fields and message types
- [ ] Message snapshots/forwarding/reference fields
- [ ] Role-subscription and monetization message payloads
- [ ] Call/message-call fields
- [ ] Interaction metadata fields on messages
- [ ] Components V2 field parity as Discord evolves
- [ ] Attachment metadata parity
- [ ] Reaction endpoint parity including burst/super-reaction behavior
- [ ] Editing-message attachment retention/removal semantics
- [ ] File type filtering behavior and helpers where useful

### User, presence, activity, and integration models

- [ ] Replace raw Presence `activities` JSON with a forward-compatible typed Activity model
- [ ] Activity timestamps, emoji, party, assets, secrets, buttons, and flags models
- [ ] Preserve unknown Activity types/fields without parse failure
- [ ] Complete Client Status model audit as Discord adds platforms
- [ ] Model full User fields, flags, avatar decoration, collectibles/nameplate, and other stable documented subobjects
- [ ] Use explicit partial-user/partial-member/partial-channel types where Discord's wire payload is intentionally partial

### Member, role, permissions, and moderation completeness

- [ ] Audit Guild Member fields and flags
- [ ] Audit Role fields, tags, colors, icons, and flags
- [ ] Role connection/subscription-related role metadata
- [ ] Ban/prune/bulk-ban endpoint parity
- [ ] Timeout and moderation edge-case validation
- [ ] Auto Moderation action/trigger parity
- [ ] Role-hierarchy comparison helpers where Discord permissions depend on position
- [ ] Effective-permission helpers for implicit permission behavior (`VIEW_CHANNEL`, `SEND_MESSAGES`, `CONNECT`)
- [ ] Thread permission inheritance semantics, including `SEND_MESSAGES_IN_THREADS`
- [ ] Timed-out-member effective permissions (`VIEW_CHANNEL` + `READ_MESSAGE_HISTORY`, except owner/admin)
- [ ] Permission syncing semantics documented separately from permission inheritance

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

### Application commands and interaction payloads

- [ ] Audit all command option/choice/localization fields
- [ ] Entry-point/application command handler parity
- [ ] Command permission endpoint parity
- [ ] Interaction callback type parity, including deprecated/limited callback values without losing unknown values
- [ ] Interaction callback resource/activity-instance parity
- [ ] Components/modal payload parity
- [ ] Attachment support in all applicable interaction callbacks/followups
- [ ] `with_response` query behavior on Create Interaction Response
- [ ] User-install-only followup limit behavior/documentation

### HTTP interaction ingress

**Goal:** support framework-neutral verification and parsing for apps that receive interactions over HTTP instead of `INTERACTION_CREATE`.

- [ ] Ed25519 verification helper for `X-Signature-Ed25519` + `X-Signature-Timestamp` over the raw request body
- [ ] Reject invalid signatures before deserializing/dispatching trusted interaction data
- [ ] PING (`type: 1`) verification and PONG (`type: 1`) response helper
- [ ] Framework-neutral request/response primitives rather than embedding an HTTP server framework
- [ ] Document that Gateway and HTTP interaction delivery modes are mutually exclusive
- [ ] Inline `200` Interaction Response helpers for HTTP ingress
- [ ] Separate-callback `202 No Content` behavior for HTTP-received interactions answered through the callback endpoint
- [ ] Enforce/document the 3-second initial-response deadline and 15-minute interaction-token lifetime
- [ ] Fixtures for valid signatures, invalid signatures, PING/PONG, inline responses, deferred responses, and malformed payloads
- [ ] Share signature-verification primitives with Phase 10 Webhook Events where the verification algorithm is identical

### OAuth2 completeness

- [ ] Enumerated/forward-compatible Discord OAuth2 scopes, including approved-partner/unavailable scopes without pretending they are generally usable
- [ ] Authorization Code Grant parity
- [ ] Implicit Grant parity
- [ ] Client Credentials Grant parity
- [ ] Bot authorization flow parity
- [ ] `webhook.incoming` authorization flow parity
- [ ] Refresh-token parity
- [ ] Token revocation parity
- [ ] Get Current Bot Application Information
- [ ] Get Current Authorization Information
- [ ] OAuth2 current-user endpoints that require Bearer authentication
- [ ] Authentication abstraction that can support Bot and Bearer clients without leaking tokens
- [ ] Correct `application/x-www-form-urlencoded` handling for token and revocation requests

## Phase 10 — HTTP Webhook Events

**Goal:** support Discord's event-delivery-over-HTTP surface separately from ordinary outgoing webhooks and interaction webhooks.

### Endpoint verification and delivery lifecycle

- [ ] Webhook Event envelope/version model
- [ ] PING webhook type (`type: 0`) handling
- [ ] PING acknowledgement helper returning `204 No Content` with an empty body
- [ ] Ed25519 verification of `X-Signature-Ed25519` + `X-Signature-Timestamp`
- [ ] Invalid-signature helper behavior returning/recommending HTTP 401
- [ ] Acknowledge normal events with `204 No Content` within Discord's 3-second deadline
- [ ] Model/document Discord's exponential retries for up to 10 minutes when events are not acknowledged
- [ ] Optional replay-window policy as a defense-in-depth helper, clearly distinguished from Discord's required signature verification
- [ ] Event subscription/configuration fields through Edit Current Application

### Webhook event models

- [ ] Forward-compatible unknown Webhook Event fallback
- [ ] `APPLICATION_AUTHORIZED`
- [ ] `APPLICATION_DEAUTHORIZED`
- [ ] `ENTITLEMENT_CREATE`
- [ ] `ENTITLEMENT_UPDATE`
- [ ] `ENTITLEMENT_DELETE`
- [ ] Known `QUEST_USER_ENROLLMENT` type represented while Discord documents it as currently unavailable
- [ ] `LOBBY_MESSAGE_CREATE`
- [ ] `LOBBY_MESSAGE_UPDATE`
- [ ] `LOBBY_MESSAGE_DELETE`
- [ ] `GAME_DIRECT_MESSAGE_CREATE`
- [ ] `GAME_DIRECT_MESSAGE_UPDATE`
- [ ] `GAME_DIRECT_MESSAGE_DELETE`
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
- [ ] Obfuscated-channel cache policy and transition to/from full channel objects
- [ ] Explicit rules for partial Gateway objects versus complete REST objects
- [ ] Cache invalidation semantics for guild unavailability, reconnect, and resharding
- [ ] Idempotent handling of duplicate/replayed Gateway and Webhook events
- [ ] Tests proving out-of-order, duplicate, missing, or partial events cannot silently corrupt normalized state
- [ ] Document that Discord events are eventually consistent and may be delivered zero, one, or multiple times

## Phase 12 — API parity infrastructure

**Goal:** make future Discord changes discoverable instead of relying on occasional manual audits.

- [ ] Maintain a checked-in Discord resource/endpoint inventory
- [ ] Maintain a checked-in Gateway dispatch inventory
- [ ] Maintain a checked-in Gateway send-event/opcode inventory
- [ ] Maintain a checked-in Webhook Event inventory
- [ ] Maintain a checked-in HTTP interaction-ingress requirements inventory
- [ ] Maintain an authentication matrix for unauthenticated, Bot, Bearer, webhook-token, and application-credential endpoints
- [ ] Track OAuth2 scope requirements per Bearer endpoint
- [ ] Track request content type (`application/json`, form-urlencoded, multipart) per endpoint
- [ ] Track optional versus nullable request/resource fields in the parity manifest
- [ ] Track array-query and boolean-query serialization requirements
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
- [ ] Treat Discord's API reference index as the core parity boundary and separately track partner/early-access APIs

## Phase 13 — API quality, protocol semantics, and ergonomics before 1.0

### Public API consistency

- [ ] Standard naming conventions across REST methods (`get_*`, `list_*`, `create_*`, `modify_*`, `delete_*`)
- [ ] Consistent typed query/request structs instead of ad-hoc argument lists
- [ ] Consistent audit-log reason parameter strategy
- [ ] Consistent pagination primitives across all list endpoints
- [ ] Consistent response wrappers when headers/status metadata matter
- [ ] Avoid public `serde_json::Value` where Discord has a stable documented object
- [ ] Preserve `Value`/extra-field escape hatches where Discord schemas are intentionally open-ended
- [ ] Public API review for unnecessary allocation/cloning in hot Gateway/voice paths
- [ ] Public API review for `#[non_exhaustive]` and forward-compatible numeric/string newtypes

### Permission representation and semantics

- [ ] Replace or redesign the current `u64`-bounded permission representation so unknown future Discord permission bits above bit 63 can be retained
- [ ] Preserve ergonomic constants and bitwise operations for known permissions while using an arbitrary-precision/string-backed or multiword representation internally
- [ ] Serialization/deserialization tests for permission values larger than `u64::MAX`
- [ ] Migration strategy for public APIs that currently expose `Permissions: u64` semantics

### HTTP/reference correctness

- [ ] Emit Discord's required User-Agent shape: `DiscordBot ($url, $versionNumber)`
- [ ] Point User-Agent/package repository metadata at `aliceblackrose/gloamwire`
- [ ] Correct repeated-key array-query encoding (for example `?id=123&id=456`) wherever Discord documents array query parameters
- [ ] Correct accepted boolean-query serialization
- [ ] Request-field abstraction for three-state values where Discord distinguishes omitted, explicit `null`, and concrete values
- [ ] Audit ISO8601 timestamp parsing/serialization strategy across models and request bodies
- [ ] Image-data URI helpers for endpoints that accept image data
- [ ] Signed attachment CDN URL preservation/expiry documentation
- [ ] Editing-attachment semantics and file-type filtering tests
- [ ] Invalid-request/rate-limit observability so callers can detect repeated authorization/validation failures before Discord-level enforcement

### Documentation and examples

- [ ] Complete rustdoc examples for every major resource family
- [ ] Add examples for REST, Gateway, cache, sharding, OAuth2, HTTP interactions, Webhook Events, voice, DAVE, and specialized resources
- [ ] Document which features are core parity, partner-only, early-access, or deliberately out of scope

## Scope boundaries

Gloamwire's core target is Discord's **public documented application/bot HTTP API**, Gateway, interaction HTTP transport, Voice Gateway/media transport, OAuth2, and HTTP Webhook Events.

The following client-integration surfaces are **not required for core API parity** because they are SDK/client products rather than the bot/application transport API Gloamwire is designed around:

- Discord Social SDK client library
- Embedded App SDK / Activities client SDK
- Local Discord RPC
- Certified Device SDK/integration surface

Public server-side HTTP resources that also support Social SDK use cases—such as Lobby, Application Identity Profile, and Webhook Event payloads—remain in core parity when they appear in Discord's normal public API reference.

Partner-only, approved-access, experimental, or early-access server APIs that live outside the normal public API reference should be tracked separately. They should not silently become a `1.0` blocker unless Discord promotes them into the stable public reference or Gloamwire explicitly chooses a partner-API feature tier.

Voice/video/Go Live behavior should only be claimed as parity where Discord publishes a stable application/bot protocol. Undocumented client-only media behavior is not part of the core parity promise.

If Rust support for excluded client/partner surfaces is added later, it should live behind clearly separated feature flags or companion crates so the core `model`/`transport` dependency graph remains suitable for bots and services.

## 1.0 parity gate

A `1.0` release should require all of the following:

- [ ] Stable Gateway lifecycle and reconnection behavior
- [ ] Correct REST and Gateway rate limiting, including opcode-specific Gateway rate-limit responses
- [ ] Typed coverage for all documented bot-relevant Gateway dispatches and send events
- [ ] Channel-obfuscation-safe Gateway models/cache behavior
- [ ] Complete documented REST coverage for core and specialized public bot/application resources
- [ ] HTTP interaction ingress verification/parsing support
- [ ] HTTP Webhook Events support
- [ ] Voice Gateway/media transport and DAVE behavior covered by integration fixtures
- [ ] Permission values remain forward-compatible beyond 64 bits
- [ ] Correct optional-vs-nullable request semantics where omission and explicit `null` differ
- [ ] Correct Discord HTTP User-Agent and request-encoding semantics
- [ ] Forward-compatible unknown event/opcode/value/flag handling
- [ ] Idempotent state/event handling under Discord's eventual-consistency model
- [ ] Strong integration, property, serialization-golden, and protocol-fixture test coverage
- [ ] A reproducible Discord API parity audit with no undocumented omissions

The `0.x` line may continue evolving APIs while these parity phases are completed. Endpoint count alone is not a release criterion: protocol correctness, rate-limit correctness, forward compatibility, authentication correctness, and reliable lifecycle behavior remain higher priority.
