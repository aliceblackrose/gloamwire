use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::Instant,
};

use crate::{
    RestClient,
    error::{Error, Result},
};

use super::{
    DistributedShardCoordinator, GatewayConfig, GatewayConnection, GatewayCoordinationError,
    GatewayIdentifyCoordinator, GatewayIntents, GatewayReconnectStrategy, ShardCount, ShardEvent,
    ShardId, shard::shard_error,
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
        if self.lease_ttl.is_zero() || self.lease_ttl > Duration::from_secs(86_400) {
            return Err(Error::GatewayProtocol(
                "distributed shard lease TTL must be non-zero and at most 24 hours".to_owned(),
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

/// Identity attached to each distributed event. The generation must increase
/// on every successful acquisition of the same shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseIdentity {
    pub owner_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistributedShardEvent {
    pub shard: ShardEvent,
    pub lease: LeaseIdentity,
}

#[derive(Debug)]
struct LeaseState {
    deadline: Mutex<Option<Instant>>,
    identity: LeaseIdentity,
}

impl LeaseState {
    fn valid(&self) -> bool {
        self.deadline
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|deadline| Instant::now() < deadline)
    }

    fn set_deadline(&self, deadline: Option<Instant>) {
        *self.deadline.lock().unwrap_or_else(|e| e.into_inner()) = deadline;
    }
}

#[derive(Debug)]
struct OwnedEvent {
    event: ShardEvent,
    lease: Arc<LeaseState>,
}

type QueuedEvent = Result<crate::queue_budget::Budgeted<OwnedEvent>>;

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
    events: mpsc::Receiver<QueuedEvent>,
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
        let budget = crate::queue_budget::Budget::default();
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
            let budget = budget.clone();
            let shutdown_rx = shutdown.subscribe();

            tasks.push(tokio::spawn(run_shard_supervisor(ShardSupervisor {
                gateway_config: config,
                shard_id,
                owner_id,
                coordinator,
                distributed,
                event_tx,
                shutdown: shutdown_rx,
                budget,
            })));
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
    pub async fn next_event(&mut self) -> Option<Result<DistributedShardEvent>> {
        self.next_fenced_event().await
    }

    /// Receives an event together with its lease identity for downstream
    /// fencing. Expired or retired queued events are rejected explicitly.
    pub async fn next_fenced_event(&mut self) -> Option<Result<DistributedShardEvent>> {
        let queued = self.events.recv().await?;
        Some(queued.and_then(|queued| {
            let owned = queued.into_inner();
            if !owned.lease.valid() {
                return Err(shard_error(
                    owned.event.shard_id,
                    Error::GatewayProtocol("queued event belongs to a retired shard lease".into()),
                ));
            }
            Ok(DistributedShardEvent {
                shard: owned.event,
                lease: owned.lease.identity.clone(),
            })
        }))
    }

    /// Gracefully stops owned shards, releases their leases, and waits for every
    /// local supervisor to exit.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.events.close();
        let _ = self.shutdown.send(true);
        while let Some(task) = self.tasks.last_mut() {
            let result = task.await;
            self.tasks.pop();
            result?;
        }
        Ok(())
    }
}

struct ShardSupervisor<C> {
    gateway_config: GatewayConfig,
    shard_id: ShardId,
    owner_id: Arc<str>,
    coordinator: Arc<C>,
    distributed: DistributedShardConfig,
    event_tx: mpsc::Sender<QueuedEvent>,
    shutdown: watch::Receiver<bool>,
    budget: crate::queue_budget::Budget,
}

async fn run_shard_supervisor<C>(supervisor: ShardSupervisor<C>)
where
    C: DistributedShardCoordinator + 'static,
{
    let ShardSupervisor {
        gateway_config,
        shard_id,
        owner_id,
        coordinator,
        distributed,
        event_tx,
        mut shutdown,
        budget,
    } = supervisor;
    loop {
        if shutdown_requested(&shutdown) {
            return;
        }

        // Each acquisition has a unique ownership identity. A delayed release
        // from a previous acquisition must not release a newer lease.
        let lease_owner = format!("{owner_id}:{}:{:032x}", shard_id.get(), fastrand::u128(..));
        let started = Instant::now();
        let deadline = started + distributed.lease_ttl;
        let acquired = tokio::select! {
            _ = shutdown.changed() => return,
            _ = event_tx.closed() => return,
            result = tokio::time::timeout_at(
                deadline,
                coordinator.acquire_fenced_shard(&lease_owner, shard_id, distributed.lease_ttl),
            ) => result,
        };
        let acquired_lease = match acquired
            .map_err(|_| GatewayCoordinationError::new("shard lease acquisition timed out"))
            .and_then(|result| result)
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

        let Some(acquired_lease) = acquired_lease else {
            if wait_or_shutdown(&mut shutdown, distributed.acquire_interval).await {
                return;
            }
            continue;
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "gloamwire::gateway",
            owner_id = %owner_id,
            shard_id = shard_id.get(),
            "acquired distributed Gateway shard lease"
        );

        if Instant::now() >= deadline {
            release_lease(&event_tx, &*coordinator, &lease_owner, shard_id).await;
            continue;
        }

        // Keep renewal alive even when connecting, identifying, reconnecting,
        // or waiting for an application consumer to make room in its queue.
        let lease = Arc::new(LeaseState {
            deadline: Mutex::new(Some(deadline)),
            identity: LeaseIdentity {
                owner_id: lease_owner.clone(),
                generation: acquired_lease.generation,
            },
        });
        let result = tokio::select! {
            biased;
            result = renew_owned_shard(&*coordinator, &lease_owner, shard_id, distributed, deadline, &lease) => result,
            result = run_owned_shard(
                gateway_config.clone(),
                shard_id,
                &event_tx,
                &mut shutdown,
                &lease,
                &budget,
            ) => result,
        };
        // Invalidate before releasing or reporting errors. Queued events will
        // not be delivered as authoritative after this point.
        lease.set_deadline(None);

        release_lease(&event_tx, &*coordinator, &lease_owner, shard_id).await;
        if let Err(error) = result {
            let terminal = matches!(
                &error,
                Error::GatewayClosed { code: Some(code), .. }
                    if code.reconnect_strategy() == GatewayReconnectStrategy::Stop
            );
            if event_tx
                .send(Err(shard_error(shard_id, error)))
                .await
                .is_err()
                || terminal
            {
                return;
            }
        }
        if shutdown_requested(&shutdown)
            || event_tx.is_closed()
            || wait_or_shutdown(&mut shutdown, distributed.acquire_interval).await
        {
            return;
        }
    }
}

async fn run_owned_shard(
    config: GatewayConfig,
    shard_id: ShardId,
    event_tx: &mpsc::Sender<QueuedEvent>,
    shutdown: &mut watch::Receiver<bool>,
    lease: &Arc<LeaseState>,
    budget: &crate::queue_budget::Budget,
) -> Result<()> {
    let mut gateway = tokio::select! {
        result = GatewayConnection::connect(config) => result?,
        _ = shutdown.changed() => return Ok(()),
        _ = event_tx.closed() => return Ok(()),
    };

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                let _ = gateway.shutdown().await;
                return Ok(());
            }
            event = gateway.next_event() => {
                let event = event?;
                if !lease.valid() {
                    return Err(Error::GatewayProtocol("shard lease expired before event publication".into()));
                }
                let queued = budget.wrap(event.estimated_bytes(), OwnedEvent {
                    event: ShardEvent { shard_id, event },
                    lease: lease.clone(),
                }).ok_or_else(|| Error::GatewayProtocol("distributed event byte budget exceeded".into()))?;
                match event_tx.try_send(Ok(queued)) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        let _ = gateway.shutdown().await;
                        return Ok(());
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Fail closed before publishing the error or releasing the lease.
                        return Err(Error::GatewayProtocol(format!(
                            "shard {} event queue overflow; connection stopped",
                            shard_id.get()
                        )));
                    }
                }
            }
        }
    }
}

async fn renew_owned_shard<C>(
    coordinator: &C,
    owner_id: &str,
    shard_id: ShardId,
    distributed: DistributedShardConfig,
    mut deadline: Instant,
    lease: &LeaseState,
) -> Result<()>
where
    C: DistributedShardCoordinator + ?Sized,
{
    loop {
        let renew_at = deadline - (distributed.lease_ttl - distributed.renew_interval);
        tokio::time::sleep_until(renew_at).await;
        let started = Instant::now();
        if started >= deadline {
            return Err(Error::GatewayProtocol(
                "distributed shard lease expired".into(),
            ));
        }
        let renewed = tokio::time::timeout_at(
            deadline,
            coordinator.renew_shard(owner_id, shard_id, distributed.lease_ttl),
        )
        .await
        .map_err(|_| Error::GatewayProtocol("distributed shard lease renewal timed out".into()))?
        .map_err(|error| {
            Error::GatewayProtocol(format!("distributed shard lease renewal failed: {error}"))
        })?;
        if !renewed || Instant::now() >= deadline {
            return Err(Error::GatewayProtocol(format!(
                "distributed shard {} lease was lost by owner {owner_id}",
                shard_id.get()
            )));
        }
        // Conservatively measure the next lease from the request start.
        deadline = started + distributed.lease_ttl;
        lease.set_deadline(Some(deadline));
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
    event_tx: &mpsc::Sender<QueuedEvent>,
    coordinator: &C,
    owner_id: &str,
    shard_id: ShardId,
) where
    C: DistributedShardCoordinator + ?Sized,
{
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        coordinator.release_shard(owner_id, shard_id),
    )
    .await
    .map_err(|_| GatewayCoordinationError::new("shard lease release timed out"))
    .and_then(|result| result);
    if let Err(error) = result {
        let _ = send_coordination_error(event_tx, shard_id, "releasing shard lease", error).await;
    }
}

async fn send_coordination_error(
    event_tx: &mpsc::Sender<QueuedEvent>,
    shard_id: ShardId,
    operation: &str,
    error: GatewayCoordinationError,
) -> bool {
    event_tx
        .send(Err(shard_error(
            shard_id,
            Error::GatewayProtocol(format!(
                "Gateway coordination failed while {operation}: {error}"
            )),
        )))
        .await
        .is_ok()
}

impl<C: DistributedShardCoordinator + 'static> Drop for DistributedShardManager<C> {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use tokio::{sync::Mutex, time::Instant};

    use super::DistributedShardConfig;
    use crate::gateway::{
        DistributedShardCoordinator, GatewayCoordinationFuture, GatewayIdentifyCoordinator, ShardId,
        ShardLease,
    };

    #[derive(Default)]
    struct MemoryCoordinator {
        leases: Mutex<HashMap<ShardId, (String, Instant)>>,
        generations: Mutex<HashMap<ShardId, u64>>,
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
        fn acquire_fenced_shard<'a>(
            &'a self,
            owner_id: &'a str,
            shard_id: ShardId,
            lease_ttl: Duration,
        ) -> GatewayCoordinationFuture<'a, Option<ShardLease>> {
            Box::pin(async move {
                let mut leases = self.leases.lock().await;
                let now = Instant::now();
                if let Some((owner, expires_at)) = leases.get(&shard_id)
                    && owner != owner_id
                    && *expires_at > now
                {
                    return Ok(None);
                }
                let mut generations = self.generations.lock().await;
                let generation = generations.entry(shard_id).or_default();
                *generation = generation.checked_add(1).expect("test generation overflow");
                leases.insert(shard_id, (owner_id.to_owned(), now + lease_ttl));
                Ok(Some(ShardLease {
                    generation: *generation,
                }))
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
    async fn fencing_generation_increases_after_release() {
        let coordinator = MemoryCoordinator::default();
        let shard = ShardId::new(7);
        let ttl = Duration::from_secs(5);
        let first = coordinator
            .acquire_fenced_shard("worker-a", shard, ttl)
            .await
            .unwrap()
            .unwrap();
        coordinator.release_shard("worker-a", shard).await.unwrap();
        let second = coordinator
            .acquire_fenced_shard("worker-b", shard, ttl)
            .await
            .unwrap()
            .unwrap();
        assert!(second.generation > first.generation);
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
