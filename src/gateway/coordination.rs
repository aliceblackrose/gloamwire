use std::{
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
    time::Duration,
};

use super::ShardId;

/// Boxed asynchronous operation returned by Gateway coordination backends.
pub type GatewayCoordinationFuture<'a, T> = Pin<
    Box<dyn Future<Output = GatewayCoordinationResult<T>> + Send + 'a>,
>;

/// Result returned by Gateway coordination backends.
pub type GatewayCoordinationResult<T> = Result<T, GatewayCoordinationError>;

/// Error returned by a distributed Gateway coordination backend.
#[derive(Debug)]
pub struct GatewayCoordinationError {
    message: String,
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

impl GatewayCoordinationError {
    /// Creates a coordination error with a human-readable message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a coordination error while retaining the backend error source.
    #[must_use]
    pub fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for GatewayCoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GatewayCoordinationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

/// Coordinates Discord Gateway Identify capacity.
///
/// Discord's `max_concurrency` buckets and session-start allowance are scoped to
/// the application, not a single process. A distributed implementation must
/// therefore serialize Identify calls across every worker that uses the same bot
/// token. [`crate::gateway::GatewayConnection`] invokes this hook for both the
/// initial Identify and every later re-identification.
pub trait GatewayIdentifyCoordinator: Send + Sync {
    /// Reserves capacity for one Gateway Identify on `shard_id`.
    fn acquire_identify(
        &self,
        shard_id: ShardId,
    ) -> GatewayCoordinationFuture<'_, ()>;
}

/// Cross-process shard ownership and Identify coordinator.
///
/// Implement this trait using a shared transactional store such as Redis, etcd,
/// Consul, SQL advisory locks, or another lease-capable service. Shard leases
/// must be exclusive per shard ID and fencing-safe: once `renew_shard` returns
/// `false`, the old owner must no longer be considered authoritative.
pub trait DistributedShardCoordinator: GatewayIdentifyCoordinator + Send + Sync {
    /// Attempts to acquire an exclusive lease for one shard.
    ///
    /// Returns `true` when `owner_id` owns the shard after the call and `false`
    /// when another live owner currently holds it.
    fn acquire_shard<'a>(
        &'a self,
        owner_id: &'a str,
        shard_id: ShardId,
        lease_ttl: Duration,
    ) -> GatewayCoordinationFuture<'a, bool>;

    /// Renews an existing shard lease.
    ///
    /// Returns `false` when the lease no longer belongs to `owner_id`. Callers
    /// must stop that Gateway connection immediately when ownership is lost.
    fn renew_shard<'a>(
        &'a self,
        owner_id: &'a str,
        shard_id: ShardId,
        lease_ttl: Duration,
    ) -> GatewayCoordinationFuture<'a, bool>;

    /// Releases a shard lease owned by `owner_id`.
    fn release_shard<'a>(
        &'a self,
        owner_id: &'a str,
        shard_id: ShardId,
    ) -> GatewayCoordinationFuture<'a, ()>;
}
