//! Integracyjne testy wygaszania rUSD po stronie emitenta.
//!
//! Łączą usługę cyklu życia, trwały stan aktywa oraz dziennik audytowy SQLite.
//! Łańcuch bloków jest kontrolowanym portem, więc test potwierdza kolejność
//! trwałych efektów bez zależności od uruchomionego węzła Hardhat.

use async_trait::async_trait;
use crypto_asset_backend::{
    application::{
        AssetStateService, AssetStateStore, TokenLifecycle, WindDownAuditStore, WindDownError,
        WindDownService, evaluate_asset_state,
    },
    domain::AssetStateCode,
    infrastructure::{
        asset_state_sqlite::SqliteAssetStateStore, wind_down_sqlite::SqliteWindDownAuditStore,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct TestLifecycle {
    calls: AtomicUsize,
}

#[async_trait]
impl TokenLifecycle for TestLifecycle {
    async fn enter_wind_down(&self) -> Result<Option<String>, WindDownError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Some("0xwind-down".into()))
    }
}

#[tokio::test]
async fn wind_down_is_audited_idempotent_and_cannot_be_reversed_by_coverage_recovery() {
    let state_store = Arc::new(SqliteAssetStateStore::open(":memory:").unwrap());
    state_store
        .save(&evaluate_asset_state(Some(110.0), Some(1)))
        .unwrap();
    let asset_state = Arc::new(AssetStateService::new(state_store, 4));
    let audit = Arc::new(SqliteWindDownAuditStore::open(":memory:").unwrap());
    let chain = Arc::new(TestLifecycle::default());
    let service = WindDownService::new(asset_state.clone(), chain.clone(), audit.clone());

    let entered = service
        .enter("wind-down-integration-1", "decyzja administratora emitenta")
        .await
        .unwrap();
    let replay = service
        .enter("wind-down-integration-1", "decyzja administratora emitenta")
        .await
        .unwrap();

    assert_eq!(entered.state, AssetStateCode::WindDown);
    assert_eq!(replay.state, AssetStateCode::WindDown);
    assert_eq!(chain.calls.load(Ordering::SeqCst), 1);
    let audit_entry = audit.get("wind-down-integration-1").unwrap().unwrap();
    assert_eq!(audit_entry.transaction_hash.as_deref(), Some("0xwind-down"));

    let recovered = asset_state
        .evaluate_coverage(&crypto_asset_backend::domain::ReserveCoverage {
            observed_at_unix_ms: 2,
            bank_as_of_unix_ms: 2,
            reserve_account_id: "reserve-rusd".into(),
            currency: "USD".into(),
            reserve_balance_minor: "11000".into(),
            reserve_balance_usd: "110.00".into(),
            token_supply_raw: "100000000".into(),
            liability_usd: "100".into(),
            surplus_usd: "10".into(),
            ratio_percent: Some(110.0),
            status: crypto_asset_backend::domain::CoverageStatus::Covered,
        })
        .unwrap();
    assert_eq!(recovered.state, AssetStateCode::WindDown);

    assert!(matches!(
        service.enter("wind-down-integration-1", "inny powód").await,
        Err(WindDownError::IdempotencyConflict)
    ));
}
