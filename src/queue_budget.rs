use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Conservative byte accounting for queued decoded payloads.
#[derive(Debug, Clone)]
pub(crate) struct Budget(Arc<Semaphore>);

impl Default for Budget {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(32 * 1024 * 1024)))
    }
}

#[derive(Debug)]
pub(crate) struct Budgeted<T> {
    value: T,
    _permit: OwnedSemaphorePermit,
}

impl<T> Budgeted<T> {
    pub(crate) fn into_inner(self) -> T {
        self.value
    }
}

impl Budget {
    pub(crate) fn wrap<T>(&self, bytes: usize, value: T) -> Option<Budgeted<T>> {
        let count = u32::try_from(bytes.max(128)).ok()?;
        let permit = self.0.clone().try_acquire_many_owned(count).ok()?;
        Some(Budgeted {
            value,
            _permit: permit,
        })
    }
}

pub(crate) fn json_bytes(value: &serde_json::Value) -> usize {
    use serde_json::Value;
    let overhead = 64;
    match value {
        Value::String(value) => overhead + value.capacity(),
        Value::Array(values) => values
            .iter()
            .fold(overhead, |sum, value| sum.saturating_add(json_bytes(value))),
        Value::Object(values) => values.iter().fold(overhead, |sum, (key, value)| {
            sum.saturating_add(key.capacity())
                .saturating_add(json_bytes(value))
        }),
        _ => overhead,
    }
}

#[cfg(test)]
mod tests {
    use super::Budget;

    #[test]
    fn byte_budget_released_when_event_drained() {
        let budget = Budget::default();
        let full = budget.wrap(32 * 1024 * 1024, 1).expect("fits");
        assert!(budget.wrap(1, 2).is_none());
        assert_eq!(full.into_inner(), 1);
        assert!(budget.wrap(1, 2).is_some());
    }
}
