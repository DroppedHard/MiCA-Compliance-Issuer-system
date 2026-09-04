//! Integracyjny test monitoringu rezerw emitenta i trwałego stanu tokena.

use async_trait::async_trait;
use crypto_asset_backend::{
    application::{
        AssetStateService, ReserveError, ReserveMonitor, ReservePollingService, ReserveReader,
        ReserveStateController, SnapshotCache,
    },
    domain::{AssetState, AssetStateCode, BankReserve, TokenObservation, TokenSnapshot},
    infrastructure::{asset_state_sqlite::SqliteAssetStateStore, cache::InMemorySnapshotCache},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Notify;

struct MutableReserveReader(AtomicU64);

#[async_trait]
impl ReserveReader for MutableReserveReader {
    async fn read_reserve(&self) -> Result<BankReserve, ReserveError> {
        Ok(BankReserve {
            account_id: "reserve-rusd".into(),
            currency: "USD".into(),
            balance_minor: self.0.load(Ordering::SeqCst).to_string(),
            version: 1,
            as_of_unix_ms: 1,
        })
    }
}

struct FailingReserveReader;

#[async_trait]
impl ReserveReader for FailingReserveReader {
    async fn read_reserve(&self) -> Result<BankReserve, ReserveError> {
        Err(ReserveError::Bank(
            "symulowana niedostępność mockBanku".into(),
        ))
    }
}

struct RecordingStateController {
    states: Mutex<Vec<AssetState>>,
    notified: Notify,
}

impl RecordingStateController {
    fn new() -> Self {
        Self {
            states: Mutex::new(Vec::new()),
            notified: Notify::new(),
        }
    }
}

#[async_trait]
impl ReserveStateController for RecordingStateController {
    async fn synchronize_reserve_state(&self, state: &AssetState) -> Result<(), ReserveError> {
        self.states.lock().unwrap().push(state.clone());
        self.notified.notify_one();
        Ok(())
    }
}

struct FailingOnceStateController {
    fail_next: AtomicBool,
    synchronized_states: Mutex<Vec<AssetState>>,
}

#[async_trait]
impl ReserveStateController for FailingOnceStateController {
    async fn synchronize_reserve_state(&self, state: &AssetState) -> Result<(), ReserveError> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(ReserveError::Blockchain(
                "symulowana chwilowa awaria zapisu stanu w kontrakcie".into(),
            ));
        }
        self.synchronized_states.lock().unwrap().push(state.clone());
        Ok(())
    }
}

#[tokio::test]
async fn reserve_poll_persists_warning_then_blocks_mint_below_one_hundred_percent() {
    let cache: Arc<dyn SnapshotCache> =
        Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
    cache
        .write(TokenObservation {
            observed_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            snapshot: TokenSnapshot {
                chain_id: 31_337,
                block_number: 20,
                contract_address: "0xtoken".into(),
                name: "Research USD EMT".into(),
                symbol: "rUSD".into(),
                decimals: 6,
                total_supply_raw: "100000000".into(),
            },
        })
        .await
        .unwrap();

    let asset_state = Arc::new(AssetStateService::new(
        Arc::new(SqliteAssetStateStore::open(":memory:").unwrap()),
        4,
    ));
    let reader = Arc::new(MutableReserveReader(AtomicU64::new(10_400)));
    let controller = Arc::new(RecordingStateController::new());
    let monitor = ReserveMonitor::new(4);
    let poller = ReservePollingService::new(
        reader.clone(),
        cache,
        monitor.clone(),
        asset_state.clone(),
        controller.clone(),
        Duration::from_secs(10),
    );

    let warning = poller.poll_once().await.unwrap();
    assert_eq!(
        warning.status,
        crypto_asset_backend::domain::CoverageStatus::Covered
    );
    assert_eq!(
        asset_state.current().unwrap().state,
        AssetStateCode::Warning
    );
    assert_eq!(monitor.latest().await.unwrap().ratio_percent, Some(104.0));

    reader.0.store(9_900, Ordering::SeqCst);
    let blocked = poller.poll_once().await.unwrap();

    assert_eq!(
        blocked.status,
        crypto_asset_backend::domain::CoverageStatus::Undercollateralized
    );
    assert_eq!(
        asset_state.current().unwrap().state,
        AssetStateCode::MintBlocked
    );
    assert_eq!(monitor.latest().await.unwrap().ratio_percent, Some(99.0));
    assert_eq!(
        controller
            .states
            .lock()
            .unwrap()
            .iter()
            .map(|state| state.state)
            .collect::<Vec<_>>(),
        vec![AssetStateCode::Warning, AssetStateCode::MintBlocked]
    );
}

#[tokio::test]
async fn failed_contract_state_sync_keeps_local_mint_block_and_is_retried_on_next_poll() {
    let cache: Arc<dyn SnapshotCache> =
        Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
    cache
        .write(TokenObservation {
            observed_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            snapshot: TokenSnapshot {
                chain_id: 31_337,
                block_number: 22,
                contract_address: "0xtoken".into(),
                name: "Research USD EMT".into(),
                symbol: "rUSD".into(),
                decimals: 6,
                total_supply_raw: "100000000".into(),
            },
        })
        .await
        .unwrap();
    let asset_state = Arc::new(AssetStateService::new(
        Arc::new(SqliteAssetStateStore::open(":memory:").unwrap()),
        4,
    ));
    let controller = Arc::new(FailingOnceStateController {
        fail_next: AtomicBool::new(true),
        synchronized_states: Mutex::new(Vec::new()),
    });
    let poller = ReservePollingService::new(
        Arc::new(MutableReserveReader(AtomicU64::new(9_900))),
        cache,
        ReserveMonitor::new(4),
        asset_state.clone(),
        controller.clone(),
        Duration::from_secs(10),
    );

    assert!(matches!(
        poller.poll_once().await,
        Err(ReserveError::Blockchain(_))
    ));
    assert_eq!(
        asset_state.current().unwrap().state,
        AssetStateCode::MintBlocked,
        "lokalna bramka emisji pozostaje fail-safe mimo błędu synchronizacji kontraktu"
    );

    poller.poll_once().await.unwrap();
    assert_eq!(
        controller
            .synchronized_states
            .lock()
            .unwrap()
            .iter()
            .map(|state| state.state)
            .collect::<Vec<_>>(),
        vec![AssetStateCode::MintBlocked]
    );
}

#[tokio::test]
async fn bank_failure_in_polling_loop_persists_data_unavailable_and_synchronizes_contract_state() {
    let cache: Arc<dyn SnapshotCache> =
        Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
    cache
        .write(TokenObservation {
            observed_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            snapshot: TokenSnapshot {
                chain_id: 31_337,
                block_number: 21,
                contract_address: "0xtoken".into(),
                name: "Research USD EMT".into(),
                symbol: "rUSD".into(),
                decimals: 6,
                total_supply_raw: "100000000".into(),
            },
        })
        .await
        .unwrap();
    let asset_state = Arc::new(AssetStateService::new(
        Arc::new(SqliteAssetStateStore::open(":memory:").unwrap()),
        4,
    ));
    let controller = Arc::new(RecordingStateController::new());
    let monitor = ReserveMonitor::new(4);
    let poller = ReservePollingService::new(
        Arc::new(FailingReserveReader),
        cache,
        monitor.clone(),
        asset_state.clone(),
        controller.clone(),
        Duration::from_secs(60),
    );

    let task = tokio::spawn(poller.run());
    tokio::time::timeout(Duration::from_secs(1), controller.notified.notified())
        .await
        .expect("awaria banku powinna zostać obsłużona przez pierwszą iterację pollingu");
    task.abort();
    let _ = task.await;

    assert_eq!(
        asset_state.current().unwrap().state,
        AssetStateCode::DataUnavailable
    );
    assert!(matches!(monitor.latest().await, Err(ReserveError::Bank(_))));
    assert_eq!(
        controller.states.lock().unwrap().last().unwrap().state,
        AssetStateCode::DataUnavailable
    );
}
