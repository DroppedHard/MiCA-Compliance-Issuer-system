use super::{CacheError, EsgStore, EsgStoreError, SnapshotCache, TokenReadError, TokenReader};
use crate::domain::{EsgObservation, TokenObservation};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollingStatus {
    pub is_healthy: bool,
    pub last_success_at_unix_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Default)]
struct PollingState {
    last_success_instant: Option<Instant>,
    last_success_at_unix_ms: Option<u64>,
    last_error: Option<String>,
}

/// Shared liveness state. A successful read remains healthy only for a bounded time.
pub struct PollingMonitor {
    state: RwLock<PollingState>,
    max_staleness: Duration,
}

impl PollingMonitor {
    pub fn new(max_staleness: Duration) -> Self {
        Self {
            state: RwLock::new(PollingState::default()),
            max_staleness,
        }
    }

    pub async fn status(&self) -> PollingStatus {
        let state = self.state.read().await;
        let is_fresh = state
            .last_success_instant
            .is_some_and(|last_success| last_success.elapsed() <= self.max_staleness);
        PollingStatus {
            is_healthy: is_fresh && state.last_error.is_none(),
            last_success_at_unix_ms: state.last_success_at_unix_ms,
            last_error: state.last_error.clone(),
        }
    }

    async fn record_success(&self, observed_at_unix_ms: u64) {
        let mut state = self.state.write().await;
        state.last_success_instant = Some(Instant::now());
        state.last_success_at_unix_ms = Some(observed_at_unix_ms);
        state.last_error = None;
    }

    async fn record_failure(&self, error: String) {
        self.state.write().await.last_error = Some(error);
    }
}

pub struct ChainPollingService {
    reader: Arc<dyn TokenReader>,
    cache: Arc<dyn SnapshotCache>,
    monitor: Arc<PollingMonitor>,
    poll_interval: Duration,
    observations: ObservationBroadcaster,
    esg_store: Arc<dyn EsgStore>,
    esg_observations: EsgBroadcaster,
}

#[derive(Clone)]
pub struct EsgBroadcaster {
    sender: broadcast::Sender<EsgObservation>,
    latest: Arc<RwLock<Option<EsgObservation>>>,
}
impl EsgBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            latest: Arc::new(RwLock::new(None)),
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<EsgObservation> {
        self.sender.subscribe()
    }
    pub async fn latest(&self) -> Option<EsgObservation> {
        self.latest.read().await.clone()
    }
    pub(crate) async fn publish(&self, value: EsgObservation) {
        *self.latest.write().await = Some(value.clone());
        let _ = self.sender.send(value);
    }
}

#[derive(Clone)]
pub struct ObservationBroadcaster {
    sender: broadcast::Sender<TokenObservation>,
}

impl ObservationBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TokenObservation> {
        self.sender.subscribe()
    }

    fn publish(&self, observation: TokenObservation) {
        // Zero active SSE clients is valid, so a send error is intentionally ignored.
        let _ = self.sender.send(observation);
    }
}

impl ChainPollingService {
    pub fn new(
        reader: Arc<dyn TokenReader>,
        cache: Arc<dyn SnapshotCache>,
        monitor: Arc<PollingMonitor>,
        poll_interval: Duration,
        observations: ObservationBroadcaster,
        esg_store: Arc<dyn EsgStore>,
        esg_observations: EsgBroadcaster,
    ) -> Self {
        Self {
            reader,
            cache,
            monitor,
            poll_interval,
            observations,
            esg_store,
            esg_observations,
        }
    }

    pub async fn poll_once(&self) -> Result<TokenObservation, PollingError> {
        let snapshot = self.reader.read_snapshot().await?;
        let contract = snapshot.contract_address.clone();
        let previous = self
            .esg_store
            .last_processed_block(snapshot.chain_id, &contract)?;
        let from_block = previous.map_or(0, |block| block.saturating_add(1));
        let transfer_count = self
            .reader
            .count_transfer_transactions(from_block, snapshot.block_number)
            .await?;
        let date = time::OffsetDateTime::now_utc().date().to_string();
        let esg = self.esg_store.record_observation(
            snapshot.chain_id,
            &contract,
            snapshot.block_number,
            &date,
            transfer_count,
        )?;
        let observation = TokenObservation {
            observed_at_unix_ms: unix_time_ms(),
            snapshot,
        };
        self.cache.write(observation.clone()).await?;
        self.monitor
            .record_success(observation.observed_at_unix_ms)
            .await;
        self.observations.publish(observation.clone());
        self.esg_observations.publish(esg).await;
        Ok(observation)
    }

    pub async fn run(self) {
        let mut ticker = interval(self.poll_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match self.poll_once().await {
                Ok(observation) => info!(
                    block_number = observation.snapshot.block_number,
                    "blockchain state cached"
                ),
                Err(error) => {
                    let message = error.to_string();
                    self.monitor.record_failure(message.clone()).await;
                    error!(error = %message, "blockchain polling failed");
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum PollingError {
    #[error(transparent)]
    Read(#[from] TokenReadError),
    #[error(transparent)]
    Cache(#[from] CacheError),
    #[error(transparent)]
    Esg(#[from] EsgStoreError),
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::CachedTokenQueryService, domain::TokenSnapshot,
        infrastructure::cache::InMemorySnapshotCache,
    };
    use async_trait::async_trait;

    struct StubTokenReader;

    struct RecordingTokenReader {
        calls: std::sync::Mutex<Vec<(u64, u64)>>,
        fail_transfer_read: bool,
    }

    #[async_trait]
    impl TokenReader for StubTokenReader {
        async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError> {
            Ok(TokenSnapshot {
                chain_id: 31337,
                block_number: 42,
                contract_address: "0x1234".to_owned(),
                name: "Research Euro EMT".to_owned(),
                symbol: "rEUR".to_owned(),
                decimals: 6,
                total_supply_raw: "1000000".to_owned(),
            })
        }
        async fn count_transfer_transactions(
            &self,
            _from_block: u64,
            _to_block: u64,
        ) -> Result<u64, TokenReadError> {
            Ok(2)
        }
    }

    #[async_trait]
    impl TokenReader for RecordingTokenReader {
        async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError> {
            Ok(TokenSnapshot {
                chain_id: 31337,
                block_number: 42,
                contract_address: "0x1234".to_owned(),
                name: "Research Euro EMT".to_owned(),
                symbol: "rEUR".to_owned(),
                decimals: 6,
                total_supply_raw: "1000000".to_owned(),
            })
        }
        async fn count_transfer_transactions(
            &self,
            from_block: u64,
            to_block: u64,
        ) -> Result<u64, TokenReadError> {
            self.calls.lock().unwrap().push((from_block, to_block));
            if self.fail_transfer_read {
                Err(TokenReadError::Rpc("log read failed".to_owned()))
            } else {
                Ok(2)
            }
        }
    }

    #[tokio::test]
    async fn successful_poll_populates_cache_and_makes_queries_healthy() {
        let cache: Arc<dyn SnapshotCache> =
            Arc::new(InMemorySnapshotCache::new(Duration::from_secs(86_400)));
        let monitor = Arc::new(PollingMonitor::new(Duration::from_secs(30)));
        let database = std::env::temp_dir().join(format!("esg-test-{}.sqlite", unix_time_ms()));
        let esg_store: Arc<dyn EsgStore> = Arc::new(
            crate::infrastructure::sqlite::SqliteEsgStore::open(database.to_str().unwrap())
                .unwrap(),
        );
        let poller = ChainPollingService::new(
            Arc::new(StubTokenReader),
            Arc::clone(&cache),
            Arc::clone(&monitor),
            Duration::from_secs(10),
            ObservationBroadcaster::new(16),
            esg_store,
            EsgBroadcaster::new(16),
        );
        let query = CachedTokenQueryService::new(cache, monitor);

        assert!(query.get_latest().await.is_err());
        poller.poll_once().await.unwrap();
        let observation = query.get_latest().await.unwrap();

        assert_eq!(observation.snapshot.block_number, 42);
        assert_eq!(observation.snapshot.symbol, "rEUR");
        assert!(query.polling_status().await.is_healthy);
    }

    #[tokio::test]
    async fn esg_broadcaster_updates_latest_value_and_subscribers() {
        let broadcaster = EsgBroadcaster::new(4);
        let mut subscriber = broadcaster.subscribe();
        let expected = crate::domain::EsgObservation {
            observed_at_unix_ms: 1,
            last_processed_block: 7,
            chain_id: 1,
            contract_address: "0xabc".to_owned(),
            current_day: crate::config::esg::estimate("2026-08-22".to_owned(), 2, "provisional"),
            methodology: crate::config::esg::methodology(),
        };

        broadcaster.publish(expected.clone()).await;

        assert_eq!(broadcaster.latest().await, Some(expected.clone()));
        assert_eq!(subscriber.recv().await.unwrap(), expected);
    }

    #[tokio::test]
    async fn polling_resumes_after_the_checkpoint_and_persists_only_new_transfers() {
        let reader = Arc::new(RecordingTokenReader {
            calls: std::sync::Mutex::new(Vec::new()),
            fail_transfer_read: false,
        });
        let store =
            Arc::new(crate::infrastructure::sqlite::SqliteEsgStore::open(":memory:").unwrap());
        let today = time::OffsetDateTime::now_utc().date().to_string();
        store
            .record_observation(31337, "0x1234", 40, &today, 3)
            .unwrap();
        let cache: Arc<dyn SnapshotCache> =
            Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
        let poller = ChainPollingService::new(
            reader.clone(),
            cache,
            Arc::new(PollingMonitor::new(Duration::from_secs(30))),
            Duration::from_secs(10),
            ObservationBroadcaster::new(4),
            store.clone(),
            EsgBroadcaster::new(4),
        );

        poller.poll_once().await.unwrap();

        assert_eq!(*reader.calls.lock().unwrap(), vec![(41, 42)]);
        assert_eq!(
            store.last_processed_block(31337, "0x1234").unwrap(),
            Some(42)
        );
        assert_eq!(
            store.recent_estimates(31337, "0x1234", 7).unwrap()[0].transaction_count,
            5
        );
    }

    #[tokio::test]
    async fn failed_log_read_does_not_advance_checkpoint_or_publish_cache() {
        let reader = Arc::new(RecordingTokenReader {
            calls: std::sync::Mutex::new(Vec::new()),
            fail_transfer_read: true,
        });
        let store =
            Arc::new(crate::infrastructure::sqlite::SqliteEsgStore::open(":memory:").unwrap());
        let cache: Arc<dyn SnapshotCache> =
            Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
        let monitor = Arc::new(PollingMonitor::new(Duration::from_secs(30)));
        let poller = ChainPollingService::new(
            reader,
            Arc::clone(&cache),
            Arc::clone(&monitor),
            Duration::from_secs(10),
            ObservationBroadcaster::new(4),
            store.clone(),
            EsgBroadcaster::new(4),
        );

        assert!(poller.poll_once().await.is_err());
        assert_eq!(store.last_processed_block(31337, "0x1234").unwrap(), None);
        assert!(cache.latest().await.unwrap().is_none());
        assert!(!monitor.status().await.is_healthy);
    }
}
