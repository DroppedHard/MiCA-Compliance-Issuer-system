//! Integracyjny test obserwatora emitenta: polling, trwały checkpoint i cache.

use async_trait::async_trait;
use crypto_asset_backend::{
    application::{
        ChainPollingService, EsgBroadcaster, EsgStore, ObservationBroadcaster, PollingMonitor,
        SnapshotCache, TokenReadError, TokenReader,
    },
    domain::TokenSnapshot,
    infrastructure::{cache::InMemorySnapshotCache, sqlite::SqliteEsgStore},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct FailOnceLogReader {
    calls: Mutex<Vec<(u64, u64)>>,
    fail_next_log_read: AtomicBool,
}

#[async_trait]
impl TokenReader for FailOnceLogReader {
    async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError> {
        Ok(TokenSnapshot {
            chain_id: 31_337,
            block_number: 42,
            contract_address: "0x1234".into(),
            name: "Research USD EMT".into(),
            symbol: "rUSD".into(),
            decimals: 6,
            total_supply_raw: "100000000".into(),
        })
    }

    async fn count_transfer_transactions(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<u64, TokenReadError> {
        self.calls.lock().unwrap().push((from_block, to_block));
        if self.fail_next_log_read.swap(false, Ordering::SeqCst) {
            return Err(TokenReadError::Rpc("symulowany błąd odczytu logów".into()));
        }
        Ok(2)
    }
}

#[tokio::test]
async fn failed_log_read_keeps_checkpoint_and_retry_processes_the_missing_range_once() {
    let reader = Arc::new(FailOnceLogReader {
        calls: Mutex::new(Vec::new()),
        fail_next_log_read: AtomicBool::new(true),
    });
    let store = Arc::new(SqliteEsgStore::open(":memory:").unwrap());
    let today = time::OffsetDateTime::now_utc().date().to_string();
    store
        .record_observation(31_337, "0x1234", 40, &today, 3)
        .unwrap();

    let cache: Arc<dyn SnapshotCache> =
        Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
    let monitor = Arc::new(PollingMonitor::new(Duration::from_secs(30)));
    let poller = ChainPollingService::new(
        reader.clone(),
        cache.clone(),
        monitor.clone(),
        Duration::from_secs(10),
        ObservationBroadcaster::new(4),
        store.clone(),
        EsgBroadcaster::new(4),
    );

    assert!(poller.poll_once().await.is_err());
    assert_eq!(
        store.last_processed_block(31_337, "0x1234").unwrap(),
        Some(40)
    );
    assert!(cache.latest().await.unwrap().is_none());
    assert!(!monitor.status().await.is_healthy);

    let observation = poller.poll_once().await.unwrap();

    assert_eq!(observation.snapshot.block_number, 42);
    assert_eq!(
        *reader.calls.lock().unwrap(),
        vec![(41, 42), (41, 42)],
        "ponowienie ma odczytać identyczny, niezatwierdzony zakres"
    );
    assert_eq!(
        store.last_processed_block(31_337, "0x1234").unwrap(),
        Some(42)
    );
    assert_eq!(
        store.recent_estimates(31_337, "0x1234", 1).unwrap()[0].transaction_count,
        5,
        "dwa transfery z ponowienia mogą zwiększyć agregat tylko raz"
    );
    assert_eq!(
        cache.latest().await.unwrap().unwrap().snapshot.block_number,
        42
    );
    assert!(monitor.status().await.is_healthy);
}
