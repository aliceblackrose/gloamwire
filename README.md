# Gloamwire

Gloamwire is an asynchronous Discord Gateway and REST API library for Rust. It provides the transport and protocol primitives needed to build bots and services without imposing a command framework, cache, or application architecture.

## Current scope

The initial `0.1` foundation includes:

- Discord REST API v10 client with bot authentication.
- Basic 429 handling using Discord's `retry_after` response.
- Typed current-user, Gateway-bot, message, and snowflake models.
- Discord Gateway v10 JSON WebSocket connection.
- Jittered heartbeat scheduling, heartbeat ACK enforcement, and sequence tracking.
- Identify payloads with current Gateway intents and optional shard coordinates.
- Raw dispatch payloads so new Discord events do not require an immediate Gloamwire release.

Gloamwire does **not** currently provide automatic session resume/reconnect, a state cache, command framework, voice transport, or complete endpoint/model coverage. Those should be added incrementally rather than hidden behind unstable abstractions.

## Requirements

- Rust 1.85 or newer.
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

`GatewayConnection::next_event` must be continuously polled because it also drives the heartbeat loop.

## Design principles

Gloamwire aims to remain a library rather than a framework. Public APIs should expose Discord concepts directly, protocol correctness takes priority over convenience, and higher-level features should be built only when their requirements are clear.

## License

MIT
