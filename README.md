# Gloamwire

Gloamwire is an asynchronous Discord Gateway and REST API library for Rust. It provides the transport and protocol primitives needed to build bots and services without imposing a command framework, cache, or application architecture.

## Current scope

The `0.1` foundation includes:

- Rust 1.98 and Edition 2024.
- Discord REST API v10 client with bot authentication.
- Basic 429 handling using Discord's `retry_after` response.
- Typed current-user, Gateway-bot, message, and snowflake models.
- Discord Gateway v10 JSON WebSocket connections.
- Jittered heartbeat scheduling, heartbeat ACK enforcement, latency measurement, and sequence tracking.
- Identify payloads with current Gateway intents and optional shard coordinates.
- READY session-state capture with `session_id` and `resume_gateway_url`.
- Opcode 6 session Resume support.
- Typed Gateway close-code recovery policy.
- Automatic resume/re-identify handling with reconnect backoff and jitter.
- Graceful client shutdown.
- Raw dispatch payloads so new Discord events do not require an immediate Gloamwire release.

Gloamwire does **not** yet provide a state cache, command framework, voice transport, complete REST endpoint/model coverage, route-aware REST rate limiting, or a multi-shard manager. See [ROADMAP.md](ROADMAP.md) for the phased implementation plan.

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
use gloamwire::gateway::{GatewayConfig, GatewayConnection, GatewayEvent, GatewayIntents};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let token = std::env::var("DISCORD_TOKEN")?;
let config = GatewayConfig::new(token, GatewayIntents::GUILDS);
let mut gateway = GatewayConnection::connect(config).await?;

loop {
    if let GatewayEvent::Dispatch(event) = gateway.next_event().await? {
        println!("{}", event.name);
    }
}
# }
```

`GatewayConnection::next_event` must be continuously polled because it drives heartbeats and recoverable reconnects.

## Design principles

Gloamwire aims to remain a library rather than a framework. Public APIs should expose Discord concepts directly, protocol correctness takes priority over convenience, and higher-level features should be built only when their requirements are clear.

## License

MIT
