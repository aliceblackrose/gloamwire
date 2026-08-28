use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{Instant, MissedTickBehavior},
};

use crate::{
    RestClient,
    error::{Error, Result},
};

use super::{
    DistributedShardCoordinator, GatewayConfig, GatewayConnection, GatewayCoordinationError,
    GatewayIdentifyCoordinator, GatewayIntents, ShardCount, ShardEvent, ShardId,
};

/// Timing policy for distributed shard leases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistributedShardConfig {
    lease_ttl: Duration,
    renew_interval: Duration,
    acquire_interval: Duration,
}

impl Default for DistributedShardConfig {
    fn default() -> Self {
        Self {
            lease_ttl: Duration::from_secs(30),
            renew_interval: Duration::from_secs(10),
            acquire_interval: Duration::from_secs(1),
        }
    }
}

impl DistributedShardConfig {
    /// Creates the default 30-second lease policy.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            lease_ttl: Duration::from_secs(30),
            renew_interval: Duration::from_secs(10),
            acquire_interval: Duration::from_secs(1),
        }
    }

    /// Sets how long a shard lease remains valid without renewal.
    #[must_use]
    pub const fn with_lease_ttl(mut self, lease_ttl: Duration) -> Self {
        self.lease_ttl = lease_ttl;
        self
    }

    /// Sets how often an owned shard lease is renewed.
    #[must_use]
    pub const fn with_renew_interval(mut self, renew_interval: Duration) -> Self {
        self.renew_interval = renew_interval;
        self
    }

    /// Sets how often an unowned shard is retried for acquisition.
    #[must_use]
    pub const fn with_acquire_interval(mut self, acquire_interval: Duration) -> Self {
        self.acquire_interval = acquire_interval;
        self
    }

    /// Returns the shard lease TTL.
    #[must_use]
    pub const fn lease_ttl(self) -> Duration {
        self.lease_ttl
    }

    /// Returns the lease-renewal interval.
    #[must_use]
    pub const fn renew_interval(self) -> Duration {
        self.renew_interval
    }

    /// Returns the unowned-shard acquisition retry interval.
    #[must_use]
    pub const fn acquire_interval(self) -> Duration {
        self.acquire_interval
    }

    fn validate(self) -> Result<()> {
        if self.lease_ttl.is_zero() {
            return Err(Error::GatewayProtocol(
                "distributed shard lease TTL must be non-zero".to_owned(),
            ));
        }
        if self.renew_interval.is_zero() || self.renew_interval >= self.lease_ttl {
            return Err(Error::GatewayProtocol(
                "distributed shard renewal interval must be non-zero and shorter than the lease TTL"
                    .to_owned(),
            ));
        }
        if self.acquire_interval.is_zero() {
            return Err(Error::GatewayProtocol(
                "distributed shard acquisition interval must be non-zero".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Manages Discord Gateway shards across multiple cooperating processes.
///
/// Every process may be started with the same shard count. The shared
/// [`DistributedShardCoordinator`] decides which process owns each shard and
/// globally coordinates Discord Identify capacity. A worker immediately closes
/// its Gateway connection if lease renewal reports that ownership was lost.
#[derive(Debug)]
pub struct DistributedShardManager<C>
where
    C: DistributedShardCoordinator + 'static,
{
    shard_count: ShardCount,
    events: mpsc::Receiver<Result<ShardEvent>>,
    shutdown: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    _coordinator: std::marker::PhantomData<C>,
}

impl<C> DistributedShardManager<C>
where
    C: DistributedShardCoordinator + 'static,
{
    /// Discovers Discord's recommended shard count and starts distributed shard
    /// supervisors using a shared coordinator backend.
    pub async fn start(
        token: impl Into<String>,
        intents: GatewayIntents,
        rest: &RestClient,
        owner_id: impl Into<String>,
        coordinator: Arc<C>,
        distributed: DistributedShardConfig,
    ) -> Result<Self> {
        let token = token.into();
        let gateway = rest.get_gateway_bot().await?;
        let shard_count = ShardCount::new(gateway.shards).ok_or_else(|| {
            Error::GatewayProtocol("Discord recommended zero Gateway shards".to_owned())
        })?;
        let identify: Arc<dyn GatewayIdentifyCoordinator> = coordinator.clone();
        let base_config = GatewayConfig::from_gateway_bot(token, intents, &gateway)
            .with_identify_coordinator(identify);
        Self::spawn(base_config, shard_count, owner_id, coordinator, distributed)
    }

    /// Starts distributed shard supervisors from explicit Gateway configuration.
    ///
    /// The supplied coordinator replaces any process-local Identify limiter on
    /// `base_config`, ensuring later re-identification is also cross-process
    /// coordinated.
    pub fn spawn(
        base_config: GatewayConfig,
        shard_count: ShardCount,
        owner_id: impl Into<String>,
        coordinator: Arc<C>,
        distributed: DistributedShardConfig,
    ) -> Result<Self> {
        distributed.validate()?;
        let owner_id = Arc::<str>::from(owner_id.into());
        if owner_id.is_empty() {
            return Err(Error::GatewayProtocol(
                "distributed shard owner ID must not be empty".to_owned(),
            ));
        }

        let identify: Arc<dyn GatewayIdentifyCoordinator> = coordinator.clone();
        let base_config = base_config.with_identify_coordinator(identify);
        let (event_tx, events) = mpsc::channel(256);
        let (shutdown, _) = watch::channel(false);
        let mut tasks = Vec::with_capacity(shard_count.get() as usize);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "gloamwire::gateway",
            owner_id = %owner_id,
            shard_count = shard_count.get(),
            "starting distributed Discord Gateway shard manager"
        );

        for raw_shard_id in 0..shard_count.get() {
            let shard_id = ShardId::new(raw_shard_id);
            let config = base_config
                .clone()
                .with_shard(raw_shard_id, shard_count.get());
            let owner_id = Arc::clone(&owner_id);
            let coordinator = Arc::clone(&coordinator);
            let event_tx = event_tx.clone();
            let shutdown_rx = shutdown.subscribe();

            tasks.push(tokio::spawn(run_shard_supervisor(
                config,
                shard_id,
                owner_id,
                coordinator,
                distributed,
                event_tx,
                shutdown_rx,
            )));
        }
        drop(event_tx);

        Ok(Self {
            shard_count,
            events,
            shutdown,
            tasks,
            _coordinator: std::marker::PhantomData,
        })
    }

    /// Returns the total Discord shard count, including shards currently owned
    /// by other distributed workers.
    #[must_use]
    pub const fn shard_count(&self) -> ShardCount {
        self.shard_count
    }

    /// Receives the next event from a shard currently owned by this worker.
    pub async fn next_event(&mut self) -> Option<Result<ShardEvent>> {
        self.events.recv().await
    }

    /// Gracefully stops owned shards, releases their leases, and waits for every
    /// local supervisor to exit.
    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.shutdown.send(true);
        while let Some(task) = self.tasks.pop() {
            task.await?;
        }
        Ok(())
    }
}

async fn run_shard_supervisor<C>(
    gateway_config: GatewayConfig,
    shard_id: ShardId,
    owner_id: Arc<str>,
    coordinator: Arc<C>,
    distributed: DistributedShardConfig,
    event_tx: mpsc::Sender<Result<ShardEvent>>,
    mut shutdown: watch::Receiver<bool>,
) where
    C: DistributedShardCoordinator + 'static,
{
    loop {
        if shutdown_requested(&shutdown) {
            return;
        }

        let owns_shard = match coordinator
            .acquire_shard(&owner_id, shard_id, distributed.lease_ttl)
            .await
        {
            Ok(owns_shard) => owns_shard,
            Err(error) => {
                if !send_coordination_error(&event_tx, shard_id, "acquiring shard lease", error)
                    .await
                {
                    return;
                }
                if wait_or_shutdown(&mut shutdown, distributed.acquire_interval).await {
                    return;
                }
                continue;
            }
        };

        if !owns_shard {
            if wait_or_shutdown(&mut shutdown, distributed.acquire_interval).await {
                return;
            }
            continue;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "gloamwire::gateway",
            owner_id = %owner_id,
            shard_id = shard_id.get(),
            "acquired distributed Gateway shard lease"
        );

        let mut gateway = match GatewayConnection::connect(gateway_config.clone()).await {
            Ok(gateway) => gateway,
            Err(error) => {
                if event_tx.send(Err(error)).await.is_err() {
                    let _ = coordinator.release_shard(&owner_id, shard_id).await;
                    return;
                }
                release_lease(&event_tx, &*coordinator, &owner_id, shard_id).await;
                if wait_or_shutdown(&mut shutdown, distributed.acquire_interval).await {
                    return;
                }
                continue;
            }
        };

        let mut renew = tokio::time::interval_at(
            Instant::now() + distributed.renew_interval,
            distributed.renew_interval,
        );
        renew.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut should_stop = false;

        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || shutdown_requested(&shutdown) {
                        let _ = gateway.shutdown().await;
                        should_stop = true;
                        break;
                    }
                }
                _ = renew.tick() => {
                    match coordinator
                        .renew_shard(&owner_id, shard_id, distributed.lease_ttl)
                        .await
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            let _ = gateway.shutdown().await;
                            let error = GatewayCoordinationError::new(format!(
                                "distributed shard {} lease was lost by owner {}",
                                shard_id.get(), owner_id
                            ));
                            if !send_coordination_error(
                                &event_tx,
                                shard_id,
                                "renewing shard lease",
                                error,
                            )
                            .await
                            {
                                return;
                            }
                            break;
                        }
                        Err(error) => {
                            // A failed renewal is treated as loss of ownership. Continuing to
                            // run would risk two processes serving the same shard.
                            let _ = gateway.shutdown().await;
                            if !send_coordination_error(
                                &event_tx,
                                shard_id,
                                "renewing shard lease",
                                error,
                            )
                            .await
                            {
                                return;
                            }
                            break;
                        }
                    }
                }
                event = gateway.next_event() => {
                    match event {
                        Ok(event) => {
                            if event_tx.send(Ok(ShardEvent { shard_id, event })).await.is_err() {
                                let _ = gateway.shutdown().await;
                                release_lease(&event_tx, &*coordinator, &owner_id, shard_id).await;
                                return;
                            }
                        }
                        Err(error) => {
                            if event_tx.send(Err(error)).await.is_err() {
                                release_lease(&event_tx, &*coordinator, &owner_id, shard_id).await;
                                return;
                            }
                            break;
                        }
                    }
                }
            }
        }

        release_lease(&event_tx, &*coordinator, &owner_id, shard_id).await;
        if should_stop || wait_or_shutdown(&mut shutdown, distributed.acquire_interval).await {
            return;
        }
    }
}

fn shutdown_requested(shutdown: &watch::Receiver<bool>) -> bool {
    *shutdown.borrow()
}

async fn wait_or_shutdown(shutdown: &mut watch::Receiver<bool>, delay: Duration) -> bool {
    if shutdown_requested(shutdown) {
        return true;
    }

    tokio::select! {
        _ = tokio::time::sleep(delay) => false,
        changed = shutdown.changed() => changed.is_err() || shutdown_requested(shutdown),
    }
}

async fn release_lease<C>(
    event_tx: &mpsc::Sender<Result<ShardEvent>>,
    coordinator: &C,
    owner_id: &str,
    shard_id: ShardId,
) where
    C: DistributedShardCoordinator + ?Sized,
{
    if let Err(error) = coordinator.release_shard(owner_id, shard_id).await {
        let _ = send_coordination_error(event_tx, shard_id, "releasing shard lease", error).await;
    }
}

async fn send_coordination_error(
    event_tx: &mpsc::Sender<Result<ShardEvent>>,
    shard_id: ShardId,
    operation: &str,
    error: GatewayCoordinationError,
) -> bool {
    event_tx
        .send(Err(Error::GatewayProtocol(format!(
            "Gateway coordination failed while {operation} for shard {}: {error}",
            shard_id.get()
        ))))
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use tokio::{sync::Mutex, time::Instant};

    use super::DistributedShardConfig;
    use crate::gateway::{
        DistributedShardCoordinator, GatewayCoordinationFuture, GatewayIdentifyCoordinator, ShardId,
    };

    #[derive(Default)]
    struct MemoryCoordinator {
        leases: Mutex<HashMap<ShardId, (String, Instant)>>,
        identifies: Mutex<Vec<ShardId>>,
    }

    impl GatewayIdentifyCoordinator for MemoryCoordinator {
        fn acquire_identify(&self, shard_id: ShardId) -> GatewayCoordinationFuture<'_, ()> {
            Box::pin(async move {
                self.identifies.lock().await.push(shard_id);
                Ok(())
            })
        }
    }

    impl DistributedShardCoordinator for MemoryCoordinator {
        fn acquire_shard<'a>(
            &'a self,
            owner_id: &'a str,
            shard_id: ShardId,
            lease_ttl: Duration,
        ) -> GatewayCoordinationFuture<'a, bool> {
            Box::pin(async move {
                let mut leases = self.leases.lock().await;
                let now = Instant::now();
                if let Some((owner, expires_at)) = leases.get(&shard_id)
                    && owner != owner_id
                    && *expires_at > now
                {
                    return Ok(false);
                }
                leases.insert(shard_id, (owner_id.to_owned(), now + lease_ttl));
                Ok(true)
            })
        }

        fn renew_shard<'a>(
            &'a self,
            owner_id: &'a str,
            shard_id: ShardId,
            lease_ttl: Duration,
        ) -> GatewayCoordinationFuture<'a, bool> {
            Box::pin(async move {
                let mut leases = self.leases.lock().await;
                let Some((owner, expires_at)) = leases.get_mut(&shard_id) else {
                    return Ok(false);
                };
                if owner != owner_id || *expires_at <= Instant::now() {
                    return Ok(false);
                }
                *expires_at = Instant::now() + lease_ttl;
                Ok(true)
            })
        }

        fn release_shard<'a>(
            &'a self,
            owner_id: &'a str,
            shard_id: ShardId,
        ) -> GatewayCoordinationFuture<'a, ()> {
            Box::pin(async move {
                let mut leases = self.leases.lock().await;
                if leases
                    .get(&shard_id)
                    .is_some_and(|(owner, _)| owner == owner_id)
                {
                    leases.remove(&shard_id);
                }
                Ok(())
            })
        }
    }

    #[test]
    fn distributed_timing_policy_rejects_unsafe_values() {
        assert!(
            DistributedShardConfig::new()
                .with_lease_ttl(Duration::ZERO)
                .validate()
                .is_err()
        );
        assert!(
            DistributedShardConfig::new()
                .with_renew_interval(Duration::from_secs(30))
                .validate()
                .is_err()
        );
    }

    #[tokio::test]
    async fn leases_are_exclusive_renewable_and_releasable() {
        let coordinator = Arc::new(MemoryCoordinator::default());
        let shard = ShardId::new(7);
        let ttl = Duration::from_secs(5);

        assert!(
            coordinator
                .acquire_shard("worker-a", shard, ttl)
                .await
                .expect("worker A acquire")
        );
        assert!(
            !coordinator
                .acquire_shard("worker-b", shard, ttl)
                .await
                .expect("worker B blocked")
        );
        assert!(
            coordinator
                .renew_shard("worker-a", shard, ttl)
                .await
                .expect("worker A renew")
        );
        assert!(
            !coordinator
                .renew_shard("worker-b", shard, ttl)
                .await
                .expect("worker B cannot renew")
        );

        coordinator
            .release_shard("worker-a", shard)
            .await
            .expect("worker A release");
        assert!(
            coordinator
                .acquire_shard("worker-b", shard, ttl)
                .await
                .expect("worker B acquire after release")
        );
    }

    #[tokio::test]
    async fn coordinator_receives_identify_reservations() {
        let coordinator = MemoryCoordinator::default();
        coordinator
            .acquire_identify(ShardId::new(3))
            .await
            .expect("Identify reservation");
        assert_eq!(*coordinator.identifies.lock().await, vec![ShardId::new(3)]);
    }
}
