# Gloamwire

Gloamwire is an asynchronous Discord Gateway and REST API library for Rust. It provides the transport and protocol primitives needed to build bots and services without imposing a command framework or application architecture.

## Current scope

The `0.1` foundation includes:

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
- Backend-neutral DAVE/E2EE lifecycle and media-frame APIs plus an optional pure-Rust `davey` MLS backend through `dave-davey`.
- Local HTTP/Gateway/voice protocol fixtures covering rate limits, reconnect/Resume, UDP negotiation, and voice recovery behavior.
- Graceful client, voice-session, local shard-manager, and distributed shard-manager shutdown.

Gloamwire deliberately does **not** embed a command framework. The separate [Gloam Macro Commands](https://github.com/aliceblackrose/gloam-macro-commands) project provides a slash-command-only framework on top of Gloamwire while keeping protocol and transport concerns in this crate.

Voice is opt-in. The `voice` feature exposes the low-level provider-neutral transport and DAVE integration boundary; `dave-davey` adds Gloamwire's managed pure-Rust DAVE/MLS backend for normal E2EE-eligible Discord voice sessions.

## Requirements

- Rust 1.98 or newer.
- A Discord application bot token for authenticated API and Gateway examples.

## Cargo features

Gloamwire's default feature set preserves the complete `0.1` non-voice transport surface: REST and Gateway transport, both Gateway compression codecs, and Rustls TLS using native certificate roots. Voice and DAVE remain explicit opt-ins.

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
| `dave-davey` | Managed pure-Rust DAVE/MLS provider backed by `davey`. Implies `voice`. |

For a model-only dependency with no HTTP, WebSocket, TLS, or compression stack:

```toml
[dependencies]
gloamwire = { version = "0.1", default-features = false, features = ["model"] }
```

For transport without Gateway compression, explicitly select transport and TLS:

```toml
[dependencies]
gloamwire = { version = "0.1", default-features = false, features = ["transport", "tls-rustls-native-roots"] }
```

For low-level voice transport with an application-supplied DAVE provider:

```toml
[dependencies]
gloamwire = { version = "0.1", features = ["voice"] }
```

For Gloamwire's managed pure-Rust DAVE backend:

```toml
[dependencies]
gloamwire = { version = "0.1", features = ["dave-davey"] }
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

## Voice transport and DAVE

Enable `voice` to access the low-level Voice Gateway v8, UDP discovery, RTP sequencing, Opus boundaries, and transport-encryption primitives:

```rust,no_run
use gloamwire::voice::{
    VoiceEncryptionMode, VoiceGatewayConfig, VoiceGatewayConnection, VoiceTransportCrypto,
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
let transport_crypto = VoiceTransportCrypto::from_session_description(&description)?;
# let _ = transport_crypto;
# Ok(())
# }
```

The transport AEAD layer is distinct from DAVE. DAVE transforms complete encoded Opus frames end-to-end between call participants; RTP-size AEAD remains the authenticated transport layer between the client and Discord's voice SFU.

With `dave-davey`, `DaveVoiceSession::connect_davey(...)` creates the managed pure-Rust DAVE provider, advertises its supported protocol version, and composes the media pipeline as encoded Opus → DAVE → RTP → transport AEAD → UDP. `DaveVoiceSession::next_event()` applies participant and MLS transition events, `send_opus_frame()` and `recv_opus()` apply both encryption layers, and `finish_speaking()` flushes Discord's five canonical silence frames before clearing the Voice Gateway speaking state.

Applications should continuously drive the managed session's Voice Gateway events while connected so participant changes and DAVE epoch transitions are processed promptly. The backend-neutral `DaveProviderLifecycle` interface remains available when an application wants to supply a different DAVE implementation.

## Distributed sharding

`ShardManager` remains the simple single-process manager. For multiple workers, `DistributedShardManager` combines Gateway connections with a user-supplied `DistributedShardCoordinator`.

A coordinator backend is responsible for two application-wide invariants: exclusive renewable ownership of each shard and coordinated Discord Identify reservations. Gloamwire intentionally does not hardwire Redis, etcd, Consul, or a database driver into core; applications can implement the coordinator against whichever shared transactional store fits their deployment.

Lease-renewal failure or loss of ownership immediately stops the affected Gateway connection, while unowned shards are retried so ownership can rebalance between workers. `DistributedShardConfig` controls lease TTL, renewal cadence, and acquisition retry cadence.

## Optional state cache

Enable the cache with the `cache` Cargo feature:

```toml
[dependencies]
gloamwire = { version = "0.1", features = ["cache"] }
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
gloamwire = { version = "0.1", features = ["tracing"] }
```

Gloamwire emits structured events through the standard `tracing` facade and does not install a subscriber. Applications remain responsible for choosing and configuring their subscriber/exporter.

Telemetry intentionally excludes authorization headers, bot/OAuth tokens, message bodies, Gateway payload contents, and concrete REST paths. REST records use normalized route templates so webhook, interaction, and invite tokens embedded in URLs are never emitted.

## Design principles

Gloamwire aims to remain a library rather than a framework. Public APIs should expose Discord concepts directly, protocol correctness takes priority over convenience, and higher-level features should be built only when their requirements are clear.

## License

MIT