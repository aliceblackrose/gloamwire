# Gloamwire

Gloamwire is an asynchronous Discord Gateway and REST API library for Rust. It provides the transport and protocol primitives needed to build bots and services without imposing a command framework or application architecture.

## Current scope

The `1.0.1` release target includes:

- Rust 1.98 and Edition 2024.
- Discord REST API v10 client with bot authentication.
- Route-aware REST rate-limit buckets plus global/shared 429 handling.
- Strong Discord ID types such as `GuildId`, `ChannelId`, `MessageId`, and `UserId`.
- Core guild, channel/thread, member, role, permission, message, presence, user, and voice-state models.
- Discord Gateway v10 WebSocket connections with JSON or ETF encoding.
- Optional per-connection `zlib-stream` and `zstd-stream` Gateway transport compression.
- Jittered heartbeat scheduling, heartbeat ACK enforcement, latency measurement, and sequence tracking.
- READY session-state capture and opcode 6 Resume support.
- Typed Gateway close-code recovery policy with automatic resume/re-identify backoff.
- Gateway outbound rate limiting and Identify session-start concurrency enforcement.
- `/gateway/bot` discovery, guild-to-shard routing, a local multi-shard manager, and backend-neutral distributed shard coordination.
- Typed parsing for common Gateway dispatches while preserving unknown events as raw JSON.
- OAuth2 helpers and typed Discord CDN URL construction.
- Optional credential-safe `tracing` instrumentation for REST rate limits and Gateway shard/throttling lifecycle.
- Optional normalized Gateway state cache with direct typed-event synchronization and bounded message retention.
- Capability feature flags for model-only builds, transport, Gateway compression codecs, TLS, cache, tracing, voice, and DAVE.
- Discord Voice Gateway v8 rendezvous/session primitives, UDP discovery, RTP sequencing, Opus frame boundaries/pacing, speaking lifecycle, and RTP-size AES-256-GCM/XChaCha20-Poly1305 transport encryption behind the `voice` feature.
- Backend-neutral DAVE/E2EE lifecycle and media-frame APIs for application-supplied MLS providers.
- Local HTTP/Gateway/voice protocol fixtures covering rate limits, reconnect/Resume, UDP negotiation, and voice recovery behavior.
- Graceful client, voice-session, local shard-manager, and distributed shard-manager shutdown.

Gloamwire deliberately does **not** embed a command framework. The separate [Gloam Macro Commands](https://github.com/aliceblackrose/gloam-macro-commands) project provides a slash-command-only framework on top of Gloamwire while keeping protocol and transport concerns in this crate.

Voice is opt-in. The `voice` feature exposes the low-level provider-neutral transport and DAVE integration boundary. A separately vetted DAVE provider is required for managed end-to-end encrypted voice sessions.

## Requirements

- Rust 1.98 or newer.
- A Discord application bot token for authenticated API and Gateway examples.

## Cargo features

Gloamwire's default feature set preserves the complete non-voice transport surface: REST and Gateway transport, both Gateway compression codecs, and Rustls TLS using native certificate roots. Voice and DAVE remain explicit opt-ins.

| Feature | Purpose |
| --- | --- |
| `model` | Core Discord models, strong IDs, permissions, snowflakes, and CDN helpers without network transport. |
| `transport` | REST, OAuth2, and Gateway transport. Implies `model`. |
| `compression-zlib` | Discord Gateway `zlib-stream` decompression. Implies `transport`. |
| `compression-zstd` | Discord Gateway `zstd-stream` decompression. Implies `transport`. |
| `tls-rustls-native-roots` | HTTPS/WSS through Rustls with native certificate roots. Implies `transport`. |
| `cache` | Optional normalized Gateway state cache. Implies `transport`. |
| `tracing` | Credential-safe structured transport instrumentation. Implies `transport`. |
| `voice` | Voice Gateway v8, UDP discovery, RTP/Opus primitives, RTP-size transport AEAD, and backend-neutral DAVE lifecycle/media boundaries. Implies `transport` and Rustls TLS. |

For a model-only dependency with no HTTP, WebSocket, TLS, or compression stack:

```toml
[dependencies]
gloamwire = { version = "1.0.1", default-features = false, features = ["model"] }
```

For transport without Gateway compression, explicitly select transport and TLS:

```toml
[dependencies]
gloamwire = { version = "1.0.1", default-features = false, features = ["transport", "tls-rustls-native-roots"] }
```

For low-level voice transport with an application-supplied DAVE provider:

```toml
[dependencies]
gloamwire = { version = "1.0.1", features = ["voice"] }
```

`GatewayCompression::ZlibStream` and `GatewayCompression::ZstdStream` remain available in the public API across transport builds. Attempting to construct a Gateway connection with a disabled codec returns a `GatewayCompression` error that names the required Cargo feature.

## REST example

```rust,no_run
use gloamwire::RestClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = RestClient::new(std::env::var("DISCORD_TOKEN")?)?;
let user = client.get_current_user().await?;
println!("{} ({})", user.username, user.id);
# Ok(())
# }
```

## Gateway example

```rust,no_run
use gloamwire::gateway::{
    GatewayCompression, GatewayConfig, GatewayConnection, GatewayEncoding, GatewayEvent,
    GatewayIntents, TypedDispatchEvent,
};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let token = std::env::var("DISCORD_TOKEN")?;
let config = GatewayConfig::new(token, GatewayIntents::GUILDS)
    .with_encoding(GatewayEncoding::Etf)
    .with_compression(GatewayCompression::ZstdStream);
let mut gateway = GatewayConnection::connect(config).await?;

loop {
    if let GatewayEvent::Dispatch(dispatch) = gateway.next_event().await? {
        match dispatch.typed()? {
            TypedDispatchEvent::Ready(ready) => println!("ready as {}", ready.user.username),
            TypedDispatchEvent::GuildCreate(guild) => println!("guild: {}", guild.name),
            TypedDispatchEvent::Unknown { name, .. } => println!("unmodeled event: {name}"),
            _ => {}
        }
    }
}
# }
```

`GatewayConnection::next_event` must be continuously polled because it drives heartbeats and recoverable reconnects.

## Message buttons

Construct buttons with the existing `Component` model and place them in an
action row. Interactive styles (primary, secondary, success, and danger)
send a message-component interaction with the specified custom ID.

```rust
use gloamwire::model::{Component, CreateMessage, InteractiveButtonStyle};

let message = CreateMessage {
    content: Some("Continue?".into()),
    components: vec![Component::action_row([
        Component::button(InteractiveButtonStyle::Success, "confirm").with_label("Confirm"),
        Component::button(InteractiveButtonStyle::Danger, "cancel").with_label("Cancel"),
        Component::link_button("https://example.com").with_label("Help"),
    ])],
    ..CreateMessage::default()
};
```

The same components can be used in message edits, interaction responses,
followups, and webhook messages. Use `with_emoji` for an emoji-only or
label-and-emoji button, `with_disabled(true)` to disable a button, and
`Component::premium_button(sku_id)` for a purchase button. Link and premium
buttons do not dispatch click interactions. Premium buttons must not have
custom labels or emojis. For ordinary messages, each action row may contain
up to five buttons; keep interactive custom IDs unique within the message
and use at most five rows. Discord enforces component field and layout limits.

Handle an incoming `TypedDispatchEvent::InteractionCreate(interaction)` with
the button-specific accessor and acknowledge the interaction promptly:

```rust,no_run
use gloamwire::{
    RestClient,
    http::CreateInteractionResponseQuery,
    model::{Interaction, InteractionMessageData, InteractionResponse},
};

async fn handle_button(
    client: &RestClient,
    interaction: &Interaction,
) -> gloamwire::Result<()> {
    if let Some(button) = interaction.button_data()? {
        let content = match button.custom_id.as_str() {
            "confirm" => "Confirmed!",
            "cancel" => "Cancelled!",
            _ => return Ok(()),
        };
        let response = InteractionResponse::message(InteractionMessageData::content(content));
        client
            .create_interaction_response(
                interaction.id,
                interaction.token.expose(),
                &response,
                &CreateInteractionResponseQuery::default(),
            )
            .await?;
    }
    Ok(())
}
```

Acknowledge interactions within three seconds. For an in-place update, use
the existing `InteractionCallbackType::UPDATE_MESSAGE` callback with message
data; use `DEFERRED_UPDATE_MESSAGE` to acknowledge first and edit later.
Applications remain responsible for routing custom IDs and authorizing
actions for the user who clicked.

## Voice transport and DAVE

**Security and migration notice (2026-11-27):** Gloamwire 1.0.1 removes the bundled `dave-davey` feature, `DaveyProvider`, and `DaveVoiceSession::connect_davey`. The previous backend (`davey 0.2.0`) pulled in nine RustSec vulnerabilities (RUSTSEC-2026-0387, -0209, -0211, -0124, -0330, -0331, -0212, -0208, -0207), including two with no fixed release in the affected crate family. Updating the other direct dependencies did not resolve these advisories. The old implementation and its backend-specific tests remain in Git history at [30aba01](https://github.com/aliceblackrose/gloamwire/tree/30aba01404e5b8a9a0fcd4d994e25b47cbf6c5fd); do not deploy that backend until its dependencies are remediated. This is a breaking change to the unreleased feature, not a patch to the vulnerable upstream libraries. No advisories are ignored and CI audits the complete remaining dependency graph.


Enable `voice` to access the low-level Voice Gateway v8, UDP discovery, RTP sequencing, Opus boundaries, and transport-encryption primitives:

```rust,no_run
use gloamwire::voice::{
    VoiceEncryptionMode, VoiceGatewayConfig, VoiceGatewayConnection, VoiceTransportDecryptor,
    VoiceUdpSocket,
};

# async fn example(info: gloamwire::voice::VoiceConnectionInfo) -> Result<(), Box<dyn std::error::Error>> {
let mut gateway = VoiceGatewayConnection::connect(VoiceGatewayConfig::new(info)).await?;
let udp = VoiceUdpSocket::connect(gateway.ready()).await?;
let discovery = udp.discover().await?;
let mode = gateway.ready().preferred_encryption_mode()?;
gateway.select_protocol(&discovery, &mode).await?;

// After receiving VoiceGatewayEvent::SessionDescription:
# let description = gloamwire::voice::VoiceSessionDescription {
#     mode: VoiceEncryptionMode::from(VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE),
#     secret_key: [0; 32],
#     dave_protocol_version: 0,
# };
let transport_crypto = VoiceTransportDecryptor::from_session_description(&description)?;
# let _ = transport_crypto;
# Ok(())
# }
```

The transport AEAD layer is distinct from DAVE. DAVE transforms complete encoded Opus frames end-to-end between call participants; RTP-size AEAD remains the authenticated transport layer between the client and Discord's voice SFU.

To establish a managed session, implement `DaveProvider` and `DaveProviderLifecycle` using a separately vetted DAVE backend and call `DaveVoiceSession::connect(config, provider)`. Managed DAVE sessions require version 1 or higher by default. Explicitly call `VoiceGatewayConfig::with_min_dave_protocol_version(0)` only when transport-only compatibility is acceptable. The provider must implement real DAVE/MLS cryptography; there is no bundled fallback to plaintext. The media pipeline remains encoded Opus → DAVE → RTP → transport AEAD → UDP. `DaveVoiceSession::next_event()` applies participant and MLS transition events, `send_opus_frame()` and `recv_opus()` apply both encryption layers, and `finish_speaking()` flushes Discord's five canonical silence frames before clearing the Voice Gateway speaking state.

Applications should continuously drive the managed session's Voice Gateway events while connected so participant changes and DAVE epoch transitions are processed promptly. The backend-neutral `DaveProviderLifecycle` interface remains available when an application wants to supply a different DAVE implementation.

## Distributed sharding

`ShardManager` remains the simple single-process manager. For multiple workers, `DistributedShardManager` combines Gateway connections with a user-supplied `DistributedShardCoordinator`.

A coordinator backend is responsible for two application-wide invariants: exclusive renewable ownership with a persistent monotonically increasing fencing generation per shard, and coordinated Discord Identify reservations. Implement `acquire_fenced_shard` atomically; both distributed event APIs return a mandatory `lease.generation`. Downstream stores must atomically reject stale generations. Gloamwire intentionally does not hardwire Redis, etcd, Consul, or a database driver into core; applications can implement the coordinator against whichever shared transactional store fits their deployment.

Lease-renewal failure or loss of ownership immediately stops the affected Gateway connection, while unowned shards are retried so ownership can rebalance between workers. `DistributedShardConfig` controls lease TTL, renewal cadence, and acquisition retry cadence.

## Optional state cache

Enable the cache with the `cache` Cargo feature:

```toml
[dependencies]
gloamwire = { version = "1.0.1", features = ["cache"] }
```

The cache is explicit state owned by the application; Gloamwire does not hide a lock or background task inside it. Feed Gateway events into it from the same loop that consumes them:

```rust,no_run
use gloamwire::{Cache, CacheConfig};
use gloamwire::gateway::{GatewayConnection, GatewayEvent};

# async fn example(mut gateway: GatewayConnection) -> gloamwire::Result<()> {
let mut cache = Cache::new(CacheConfig::new().message_capacity(500));

loop {
    let event = gateway.next_event().await?;
    if let GatewayEvent::Dispatch(dispatch) = &event {
        let typed = cache.update_dispatch(dispatch)?;
        // Handle `typed` without parsing the dispatch a second time.
        drop(typed);
    }
}
# }
```

Guilds, channels/threads, roles, members, presences, voice states, scheduled events, and users are normalized in the cache. Message retention is disabled by default and must be assigned an explicit bounded capacity.

## Slash-command framework

Gloamwire remains the low-level protocol/transport library. Applications that want generated slash-command metadata, runtime option extraction, synchronization, contexts, subcommands, choices, autocomplete, checks, hooks, and managed command dispatch can use [Gloam Macro Commands](https://github.com/aliceblackrose/gloam-macro-commands).

Keeping that framework in a separate repository lets Gloamwire expose Discord concepts directly without coupling transport releases to a particular application architecture.

## Optional tracing

Enable Gloamwire's instrumentation with the `tracing` Cargo feature:

```toml
[dependencies]
gloamwire = { version = "1.0.1", features = ["tracing"] }
```

Gloamwire emits structured events through the standard `tracing` facade and does not install a subscriber. Applications remain responsible for choosing and configuring their subscriber/exporter.

Telemetry intentionally excludes authorization headers, bot/OAuth tokens, message bodies, Gateway payload contents, and concrete REST paths. REST records use normalized route templates so webhook, interaction, and invite tokens embedded in URLs are never emitted.

## Design principles

Gloamwire aims to remain a library rather than a framework. Public APIs should expose Discord concepts directly, protocol correctness takes priority over convenience, and higher-level features should be built only when their requirements are clear.

## License

MIT
### Independently driven connections

`GatewayDriver::connect(config)` runs Gateway receive, heartbeat and recovery processing in a Tokio task. Its cloneable `commands()` handle sends typed `GatewayCommand` operations without borrowing the event receiver. Consume `next_event()` continuously; the driver stops with an explicit error on event queue overflow. The low-level `GatewayConnection` remains available for applications that drive it directly.

For voice, transfer a negotiated `DaveVoiceSession` to `VoiceDriver::spawn(session)`. The driver independently processes Gateway heartbeats, DAVE transitions, UDP reception and outbound `VoiceCommand` operations. Control queue overflow stops the session; incoming audio uses a bounded best-effort queue, with drops available via `dropped_audio_packets()`. A terminal control error requires a fresh session. The low-level voice API remains available.

When using a shared cache with a shard manager, feed each `ShardEvent` into `Cache::update_shard_event(&event, shard_count)` rather than calling the single-shard `update_gateway_event` API. This isolates READY handling and sequence deduplication per shard. Feed the complete stream: a sequence gap, parse/apply failure, or reconnect invalidates the cache conservatively. Call `mark_stream_lost()` on driver errors or known dropped events. Clear only when rebuilding fresh authoritative snapshots.

## Security configuration

REST and OAuth2 base URL overrides require HTTPS; local fixtures may explicitly
enable numeric loopback HTTP with `with_insecure_loopback(true)`. Custom
endpoints are trusted with credentials. Gateway discovery and resume accept
Discord WSS origins by default; explicit custom Gateway URLs restrict resume
to the configured origin. Non-Discord Voice Gateway origins require
`with_custom_origin(true)`.

Managed voice sessions own their outbound nonce state: use
`VoiceSession::encrypt_audio` for low-level media instead of replacing the
transport encryptor. Standalone outbound constructors are no longer public.
A provider must be ready before managed
DAVE media operations proceed. No external MLS provider is certified here.

Credential fields use `SecretString`, which redacts debug output and zeroizes
its owned allocation. Use `.expose()` when necessary, and `.into()` to
construct credential fields from strings. `SecretString` no longer implements
`Serialize`; serialized interactions and webhooks omit token and webhook URL fields. Server error details remain available through their public fields
but are omitted from default error formatting. REST rate-limit bookkeeping
has fixed capacity and conservatively applies shared cooldowns at saturation.

### Defensive hardening (cache and voice)

Cache collection and aggregate entry limits are configured with
`CacheConfig::limits(CacheLimits { ..CacheLimits::default() })`. The cache
also uses an approximate retained-payload byte budget. It is not a precise
allocator or process-memory cap; incoming event decoding and temporary
copies may use additional memory. On a capacity violation the entire cache
is cleared and latched invalidated; subsequent events do not repopulate it.
Check `Cache::is_invalidated()`, treat missing state as unknown, and call
`Cache::clear()` only when arranging fresh authoritative snapshots. Never
treat incomplete cache data as authoritative permission information.

Voice Ready UDP addresses must be numeric public unicast addresses by default.
Private or loopback infrastructure requires the separate explicit
`VoiceUdpDestinationPolicy::AllowPrivate` opt-in on
`VoiceGatewayConfig::with_udp_destination_policy` or
`VoiceUdpSocket::connect_with_policy`. Link-local, multicast, unspecified and
other rejected special-use addresses remain blocked. Hostnames are not
resolved.

Outbound transport cryptography is constructed only by `VoiceSession`, which
owns its nonce counter. For receive-only processing use `VoiceTransportDecryptor`;
standalone receivers must implement replay protection. Independent connections
still require distinct keys from the trusted voice service; the library cannot
guarantee cross-process key uniqueness or certify external DAVE/MLS providers.

Default debug formatting omits raw voice/DAVE payloads, encoded or decrypted
audio, upload contents and paths, and OAuth2 state/redirect values. Explicit
access to raw dispatch data or explicit secret exposure can still reveal sensitive data.

### OAuth2 code-flow migration

Use `OAuth2AuthorizationUrl::begin_code_flow()` to obtain the URL and a non-cloneable, expiring pending authorization. Store the pending guard in the initiating browser's server-side session. Remove it from that session on callback and pass it, the returned code, and the returned state to `OAuth2Client::exchange_code`. A state mismatch or expired guard fails before any network request. The redirect URI and client ID are bound to the guard. Prefer authorization-code flows over the legacy implicit grant.
