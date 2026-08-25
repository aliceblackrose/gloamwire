use gloamwire::gateway::{GatewayConfig, GatewayConnection, GatewayEvent, GatewayIntents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let config = GatewayConfig::new(
        token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    );
    let mut gateway = GatewayConnection::connect(config).await?;

    loop {
        match gateway.next_event().await? {
            GatewayEvent::Dispatch(event) => println!("{} #{}", event.name, event.sequence),
            GatewayEvent::Reconnect => {
                eprintln!("Discord requested a reconnect; Gloamwire recovered automatically");
            }
            GatewayEvent::InvalidSession { resumable } => {
                eprintln!("Discord invalidated the session; resumable={resumable}");
            }
            GatewayEvent::HeartbeatAck => {
                if let Some(latency) = gateway.latency() {
                    println!("Gateway latency: {latency:?}");
                }
            }
            GatewayEvent::Unknown { .. } => {}
        }
    }
}
