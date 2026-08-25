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
- [ ] Protocol-fixture integration tests for reconnect/resume sequences

## Phase 2 — Rate limiting, Gateway discovery, and sharding

- [ ] Route-aware REST rate-limit buckets
- [ ] Global/shared rate-limit handling
- [ ] Gateway outbound 120-events/60-seconds limiter
- [ ] Gateway URL discovery through `/gateway/bot`
- [ ] Session-start-limit and Identify concurrency enforcement
- [ ] Shard ID/count types and guild-to-shard routing
- [ ] Multi-shard manager with isolated restart/shutdown
- [ ] Gateway presence, voice-state, member, soundboard, and channel-info commands
- [ ] Gateway zlib/zstd compression
- [ ] Configurable JSON/ETF encoding

## Phase 3 — Typed protocol and core Discord models

- [ ] Strong ID types (`GuildId`, `ChannelId`, `MessageId`, `UserId`, and others)
- [ ] Typed READY/RESUMED and core dispatch events
- [ ] Forward-compatible unknown-event and unknown-enum handling
- [ ] Guild, channel, thread, member, role, permission, presence, reaction, and voice-state models
- [ ] Interaction, command, component, modal, webhook, invite, audit-log, automod, scheduled-event, poll, entitlement, SKU, and subscription models
- [ ] Discord permission calculation

## Phase 4 — REST breadth, uploads, and interactions

- [ ] Central REST route abstraction with major-parameter bucket identity
- [ ] JSON, empty, binary, and header-aware response handling
- [ ] Structured Discord validation errors
- [ ] Retry classification and safe idempotent retry policy
- [ ] Configurable request/connect/pool timeouts
- [ ] Full message API
- [ ] Multipart attachments and streaming uploads
- [ ] Guild, channel, thread, role, member, moderation, webhook, audit-log, invite, and scheduled-event APIs
- [ ] Application commands and interaction responses/followups
- [ ] Pagination primitives
- [ ] OAuth2 support
- [ ] CDN URL helpers

## Phase 5 — Reliability, observability, and optional state

- [ ] HTTP and Gateway mock integration servers
- [ ] Captured/synthetic protocol fixtures
- [ ] Optional `tracing` instrumentation without credential leakage
- [ ] Optional cache layer
- [ ] Event-to-cache synchronization
- [ ] Feature flags for transport/model/cache/compression/TLS capabilities
- [ ] Fuzz/property tests for protocol parsing and route/rate-limit behavior

## Phase 6 — Advanced subsystems

- [ ] Voice Gateway and UDP/RTP transport
- [ ] Opus integration boundaries
- [ ] Discord DAVE/E2EE support for voice where required
- [ ] Distributed shard ownership/coordination
- [ ] Optional high-level command framework kept separate from the low-level protocol core

## Release direction

The `0.x` line may evolve APIs while protocol foundations are being completed. A `1.0` release should require stable Gateway lifecycle behavior, correct REST/Gateway rate limiting, typed core models/events, practical REST coverage, and strong integration-test coverage.
