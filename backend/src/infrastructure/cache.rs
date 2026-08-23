use crate::{
    application::{CacheError, SnapshotCache},
    domain::TokenObservation,
};
use async_trait::async_trait;
use std::{
    collections::VecDeque,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

/// Process-local rolling cache. Restarting the backend intentionally clears it.
pub struct InMemorySnapshotCache {
    entries: RwLock<VecDeque<TokenObservation>>,
    retention: Duration,
}

impl InMemorySnapshotCache {
    pub fn new(retention: Duration) -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
            retention,
        }
    }

    fn cutoff_unix_ms(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        now.saturating_sub(self.retention).as_millis() as u64
    }
}

#[async_trait]
impl SnapshotCache for InMemorySnapshotCache {
    async fn write(&self, observation: TokenObservation) -> Result<(), CacheError> {
        let cutoff = self.cutoff_unix_ms();
        let mut entries = self.entries.write().await;
        while entries
            .front()
            .is_some_and(|entry| entry.observed_at_unix_ms < cutoff)
        {
            entries.pop_front();
        }
        entries.push_back(observation);
        Ok(())
    }

    async fn latest(&self) -> Result<Option<TokenObservation>, CacheError> {
        let cutoff = self.cutoff_unix_ms();
        let mut entries = self.entries.write().await;
        while entries
            .front()
            .is_some_and(|entry| entry.observed_at_unix_ms < cutoff)
        {
            entries.pop_front();
        }
        Ok(entries.back().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TokenSnapshot;

    fn observation(observed_at_unix_ms: u64, block_number: u64) -> TokenObservation {
        TokenObservation {
            observed_at_unix_ms,
            snapshot: TokenSnapshot {
                chain_id: 31337,
                block_number,
                contract_address: "0x1234".to_owned(),
                name: "Research Euro EMT".to_owned(),
                symbol: "rUSD".to_owned(),
                decimals: 6,
                total_supply_raw: "0".to_owned(),
            },
        }
    }

    #[tokio::test]
    async fn removes_entries_outside_retention_window() {
        let cache = InMemorySnapshotCache::new(Duration::from_secs(60));
        cache.write(observation(0, 1)).await.unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        cache.write(observation(now, 2)).await.unwrap();
        assert_eq!(
            cache.latest().await.unwrap().unwrap().snapshot.block_number,
            2
        );
        assert_eq!(cache.entries.read().await.len(), 1);
    }
}
