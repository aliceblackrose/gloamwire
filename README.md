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
- `/gateway/bot` discovery, guild-to-shard routing, and a multi-shard manager.
- Typed parsing for common Gateway dispatches while preserving unknown events as raw JSON.
- OAuth2 helpers and typed Discord CDN URL construction.
- Optional credential-safe `tracing` instrumentation for REST rate limits and Gateway shard/throttling lifecycle.
- Optional normalized Gateway state cache with direct typed-event synchronization and bounded message retention.
- Capability feature flags for model-only builds, transport, Gateway compression codecs, TLS, cache, tracing, and experimental voice transport.
- Experimental Discord Voice Gateway v8 rendezvous/session primitives, UDP discovery, RTP sequencing, and RTP-size AES-256-GCM/XChaCha20-Poly1305 transport encryption behind the `voice` feature.
- Local HTTP/Gateway protocol integration fixtures covering rate-limit, empty-response, reconnect, and Resume behavior.
- Graceful client and shard-manager shutdown.

Gloamwire does **not** provide a command framework. Voice support is experimental: the low-level Voice Gateway/UDP/RTP transport exists, but Discord DAVE/E2EE media encryption and Opus integration are still being implemented. The `voice` feature is therefore not yet a turnkey implementation for normal non-stage Discord calls. See [ROADMAP.md](ROADMAP.md) for the phased implementation plan.

## Requirements

- Rust 1.98 or newer.
- A Discord application bot token for authenticated API and Gateway examples.

## Cargo features

Gloamwire's default feature set preserves the complete `0.1` non-voice transport surface: REST and Gateway transport, both Gateway compression codecs, and Rustls TLS using native certificate roots. Experimental voice support is opt-in.

| Feature | Purpose |
| --- | --- |
| `model` | Core Discord models, strong IDs, permissions, snowflakes, and CDN helpers without network transport. |
| `transport` | REST, OAuth2, and Gateway transport. Implies `model`. |
| `compression-zlib` | Discord Gateway `zlib-stream` decompression. Implies `transport`. |
| `compression-zstd` | Discord Gateway `zstd-stream` decompression. Implies `transport`. |
| `tls-rustls-native-roots` | HTTPS/WSS through Rustls with native certificate roots. Implies `transport`. |
| `cache` | Optional normalized Gateway state cache. Implies `transport`. |
| `tracing` | Credential-safe structured transport instrumentation. Implies `transport`. |
| `voice` | Experimental Voice Gateway v8, UDP discovery, RTP primitives, and current RTP-size transport AEAD. Implies `transport` and Rustls TLS. DAVE/E2EE is not complete yet. |

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

For the experimental voice transport surface:

```toml
[dependencies]
gloamwire = { version = "0.1", features = ["voice"] }
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

## Experimental voice transport

Enable the `voice` feature to access Voice Gateway v8, UDP discovery, RTP sequencing, and transport encryption primitives:

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

The transport AEAD layer is distinct from DAVE. DAVE encrypts encoded media frames end-to-end between call participants; Discord's RTP-size AEAD remains the authenticated transport layer between the client and Discord's voice SFU. Until Gloamwire's DAVE layer is complete, do not advertise a non-zero DAVE protocol version unless an external DAVE implementation is attached.

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
