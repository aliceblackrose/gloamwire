# Gloamwire Roadmap

Gloamwire is developed protocol-first: transport correctness and Discord lifecycle rules take priority over endpoint count.

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

## Release direction

The `0.x` line may evolve APIs while protocol foundations are being completed. A `1.0` release should require stable Gateway lifecycle behavior, correct REST/Gateway rate limiting, typed core models/events, practical REST coverage, and strong integration-test coverage.
