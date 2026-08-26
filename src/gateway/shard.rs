use std::num::NonZeroU32;

use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::{RestClient, error::Result, model::GuildId};

use super::{GatewayConfig, GatewayConnection, GatewayEvent, GatewayIntents};

/// A Discord Gateway shard identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShardId(u32);

impl ShardId {
    /// Creates a shard identifier.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw shard identifier.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A non-zero number of Discord Gateway shards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShardCount(NonZeroU32);

impl ShardCount {
    /// Creates a shard count, returning `None` for zero.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the number of shards.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// Returns the shard responsible for a guild according to Discord's shard formula.
#[must_use]
pub const fn shard_for_guild(guild_id: GuildId, shard_count: ShardCount) -> ShardId {
    ShardId::new(((guild_id.get() >> 22) % shard_count.get() as u64) as u32)
}

/// An event emitted by one managed Gateway shard.
#[derive(Debug, Clone, PartialEq)]
pub struct ShardEvent {
    /// Shard that produced the event.
    pub shard_id: ShardId,
    /// Gateway event produced by the shard.
    pub event: GatewayEvent,
}

/// Manages a set of Discord Gateway shards and exposes a unified event stream.
#[derive(Debug)]
pub struct ShardManager {
    shard_count: ShardCount,
    events: mpsc::Receiver<Result<ShardEvent>>,
    shutdown: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

impl ShardManager {
    /// Discovers Discord's Gateway configuration and starts the recommended shards.
    pub async fn start(
        token: impl Into<String>,
        intents: GatewayIntents,
        rest: &RestClient,
    ) -> Result<Self> {
        let token = token.into();
        let gateway = rest.get_gateway_bot().await?;
        let shard_count = ShardCount::new(gateway.shards).ok_or_else(|| {
            crate::Error::GatewayProtocol("Discord recommended zero Gateway shards".to_owned())
        })?;
        let base_config = GatewayConfig::from_gateway_bot(token, intents, &gateway);
        Ok(Self::spawn(base_config, shard_count))
    }

    /// Starts a specific number of shards from shared Gateway configuration.
    #[must_use]
    pub fn spawn(base_config: GatewayConfig, shard_count: ShardCount) -> Self {
        let (event_tx, events) = mpsc::channel(256);
        let (shutdown, _) = watch::channel(false);
        let mut tasks = Vec::with_capacity(shard_count.get() as usize);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "gloamwire::gateway",
            shard_count = shard_count.get(),
            "starting Discord Gateway shard manager"
        );

        for raw_shard_id in 0..shard_count.get() {
            let shard_id = ShardId::new(raw_shard_id);
            let config = base_config
                .clone()
                .with_shard(raw_shard_id, shard_count.get());
            let event_tx = event_tx.clone();
            let mut shutdown_rx = shutdown.subscribe();

            tasks.push(tokio::spawn(async move {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    target: "gloamwire::gateway",
                    shard_id = raw_shard_id,
                    "connecting Discord Gateway shard"
                );

                let mut gateway = match GatewayConnection::connect(config).await {
                    Ok(gateway) => {
                        #[cfg(feature = "tracing")]
                        tracing::debug!(
                            target: "gloamwire::gateway",
                            shard_id = raw_shard_id,
                            "Discord Gateway shard connected"
                        );
                        gateway
                    }
                    Err(error) => {
                        #[cfg(feature = "tracing")]
                        tracing::debug!(
                            target: "gloamwire::gateway",
                            shard_id = raw_shard_id,
                            "Discord Gateway shard connection failed"
                        );
                        let _ = event_tx.send(Err(error)).await;
                        return;
                    }
                };

                loop {
                    tokio::select! {
                        changed = shutdown_rx.changed() => {
                            if changed.is_err() || *shutdown_rx.borrow() {
                                #[cfg(feature = "tracing")]
                                tracing::debug!(
                                    target: "gloamwire::gateway",
                                    shard_id = raw_shard_id,
                                    "shutting down Discord Gateway shard"
                                );
                                let _ = gateway.shutdown().await;
                                break;
                            }
                        }
                        event = gateway.next_event() => {
                            match event {
                                Ok(event) => {
                                    if event_tx.send(Ok(ShardEvent { shard_id, event })).await.is_err() {
                                        #[cfg(feature = "tracing")]
                                        tracing::debug!(
                                            target: "gloamwire::gateway",
                                            shard_id = raw_shard_id,
                                            "Gateway shard event receiver dropped"
                                        );
                                        let _ = gateway.shutdown().await;
                                        break;
                                    }
                                }
                                Err(error) => {
                                    #[cfg(feature = "tracing")]
                                    tracing::debug!(
                                        target: "gloamwire::gateway",
                                        shard_id = raw_shard_id,
                                        "Discord Gateway shard stopped with an error"
                                    );
                                    let _ = event_tx.send(Err(error)).await;
                                    break;
                                }
                            }
                        }
                    }
                }
            }));
        }

        drop(event_tx);

        Self {
            shard_count,
            events,
            shutdown,
            tasks,
        }
    }

    /// Returns the number of shards managed by this instance.
    #[must_use]
    pub const fn shard_count(&self) -> ShardCount {
        self.shard_count
    }

    /// Receives the next event from any managed shard.
    pub async fn next_event(&mut self) -> Option<Result<ShardEvent>> {
        self.events.recv().await
    }

    /// Requests graceful shutdown of every shard and waits for their tasks to exit.
    pub async fn shutdown(&mut self) -> Result<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "gloamwire::gateway",
            shard_count = self.shard_count.get(),
            "requesting Discord Gateway shard-manager shutdown"
        );

        let _ = self.shutdown.send(true);

        while let Some(task) = self.tasks.pop() {
            task.await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ShardCount, ShardId, shard_for_guild};
    use crate::model::GuildId;

    #[test]
    fn rejects_zero_shards() {
        assert!(ShardCount::new(0).is_none());
    }

    #[test]
    fn computes_discord_shard_formula() {
        let count = ShardCount::new(16).expect("non-zero shard count");
        let guild = GuildId::new(81384788765712384);
        let expected = ShardId::new(((guild.get() >> 22) % 16) as u32);
        assert_eq!(shard_for_guild(guild, count), expected);
    }
}
