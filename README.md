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
- Local HTTP/Gateway protocol integration fixtures covering rate-limit, empty-response, reconnect, and Resume behavior.
- Graceful client and shard-manager shutdown.

Gloamwire does **not** provide a command framework or voice media transport. See [ROADMAP.md](ROADMAP.md) for the phased implementation plan.

## Requirements

- Rust 1.98 or newer.
- A Discord application bot token for authenticated API and Gateway examples.

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
