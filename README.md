# Gloamwire

Gloamwire is an asynchronous Discord Gateway and REST API library for Rust. It provides the transport and protocol primitives needed to build bots and services without imposing a command framework, cache, or application architecture.

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
- Graceful client and shard-manager shutdown.

Gloamwire does **not** yet provide a state cache, command framework, voice media transport, complete REST endpoint/model coverage, or every Discord dispatch model. See [ROADMAP.md](ROADMAP.md) for the phased implementation plan.

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

## Design principles

Gloamwire aims to remain a library rather than a framework. Public APIs should expose Discord concepts directly, protocol correctness takes priority over convenience, and higher-level features should be built only when their requirements are clear.

## License

MIT
