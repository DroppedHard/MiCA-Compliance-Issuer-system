//! Integracyjne testy wykupu emitenta: usługa, trwały store i bramki skutków.

use alloy::primitives::Address;
use async_trait::async_trait;
use crypto_asset_backend::{
    application::{
        AssetStateService, OperationGate, PayoutBank, RedemptionError, RedemptionService,
        RedemptionToken,
    },
    domain::{CoverageStatus, RedemptionStatus, ReserveCoverage},
    infrastructure::{
        asset_state_sqlite::SqliteAssetStateStore,
        operation_decision_sqlite::SqliteOperationDecisionStore,
        redemption_sqlite::SqliteRedemptionStore,
    },
};
use std::sync::{Arc, Mutex};

const OPERATION_ID: &str = "integration-redemption-1";
const HOLDER: &str = "0x0000000000000000000000000000000000000001";

#[derive(Default)]
struct TestToken(Mutex<Vec<(String, Address, u64)>>);

#[async_trait]
impl RedemptionToken for TestToken {
    async fn burn_for_operation(
        &self,
        id: &str,
        holder: Address,
        amount: u64,
    ) -> Result<Option<String>, RedemptionError> {
        self.0.lock().unwrap().push((id.into(), holder, amount));
        Ok(Some("0xintegrationburn".into()))
    }
}

#[derive(Default)]
struct TestBank(Mutex<Vec<(String, u64)>>);

#[async_trait]
impl PayoutBank for TestBank {
    async fn pay_usd(&self, id: &str, amount_minor: u64) -> Result<(), RedemptionError> {
        self.0.lock().unwrap().push((id.into(), amount_minor));
        Ok(())
    }
}

#[derive(Default)]
struct FailOnceBank {
    attempts: Mutex<u8>,
    successful_payouts: Mutex<Vec<(String, u64)>>,
}

#[async_trait]
impl PayoutBank for FailOnceBank {
    async fn pay_usd(&self, id: &str, amount_minor: u64) -> Result<(), RedemptionError> {
        let mut attempts = self.attempts.lock().unwrap();
        *attempts += 1;
        if *attempts == 1 {
            return Err(RedemptionError::Bank("temporary bank failure".into()));
        }
        self.successful_payouts
            .lock()
            .unwrap()
            .push((id.into(), amount_minor));
        Ok(())
    }
}

fn service_at_coverage(
    coverage_percent: f64,
) -> (Arc<RedemptionService>, Arc<TestToken>, Arc<TestBank>) {
    let state = Arc::new(AssetStateService::new(
        Arc::new(SqliteAssetStateStore::open(":memory:").unwrap()),
        4,
    ));
    state
        .evaluate_coverage(&ReserveCoverage {
            observed_at_unix_ms: 1,
            bank_as_of_unix_ms: 1,
            reserve_account_id: "reserve-rusd".into(),
            currency: "USD".into(),
            reserve_balance_minor: "9900".into(),
            reserve_balance_usd: "99.00".into(),
            token_supply_raw: "100000000".into(),
            liability_usd: "100".into(),
            surplus_usd: "-1.00".into(),
            ratio_percent: Some(coverage_percent),
            status: if coverage_percent >= 100.0 {
                CoverageStatus::Covered
            } else {
                CoverageStatus::Undercollateralized
            },
        })
        .unwrap();
    let token = Arc::new(TestToken::default());
    let bank = Arc::new(TestBank::default());
    let service = Arc::new(RedemptionService::new(
        Arc::new(SqliteRedemptionStore::open(":memory:").unwrap()),
        token.clone(),
        bank.clone(),
        state,
        Arc::new(OperationGate::new(Arc::new(
            SqliteOperationDecisionStore::open(":memory:").unwrap(),
        ))),
    ));
    (service, token, bank)
}

#[tokio::test]
async fn undercollateralization_does_not_change_redemption_parity_or_execution() {
    let (service, token, bank) = service_at_coverage(99.0);
    let created = service
        .create(OPERATION_ID.into(), HOLDER.into(), "25000000".into())
        .unwrap();

    assert_eq!(created.payout_usd_minor, "2500");
    assert_eq!(
        service.settle(OPERATION_ID).await.unwrap().status,
        RedemptionStatus::Completed
    );
    assert_eq!(
        token.0.lock().unwrap().as_slice(),
        &[(OPERATION_ID.into(), HOLDER.parse().unwrap(), 25_000_000,)]
    );
    assert_eq!(
        bank.0.lock().unwrap().as_slice(),
        &[(OPERATION_ID.into(), 2500)]
    );
}

#[tokio::test]
async fn concurrent_redemption_settlement_burns_and_pays_exactly_once() {
    let (service, token, bank) = service_at_coverage(99.0);
    service
        .create(OPERATION_ID.into(), HOLDER.into(), "25000000".into())
        .unwrap();

    let first = service.clone();
    let second = service.clone();
    let (left, right) = tokio::join!(
        async move { first.settle(OPERATION_ID).await },
        async move { second.settle(OPERATION_ID).await },
    );

    assert_eq!(left.unwrap().status, RedemptionStatus::Completed);
    assert_eq!(right.unwrap().status, RedemptionStatus::Completed);
    assert_eq!(token.0.lock().unwrap().len(), 1);
    assert_eq!(bank.0.lock().unwrap().len(), 1);
    assert_eq!(
        service
            .get(OPERATION_ID)
            .unwrap()
            .burn_transaction_hash
            .as_deref(),
        Some("0xintegrationburn")
    );
}

#[tokio::test]
async fn payout_retry_after_confirmed_burn_does_not_submit_a_second_burn() {
    let state = Arc::new(AssetStateService::new(
        Arc::new(SqliteAssetStateStore::open(":memory:").unwrap()),
        4,
    ));
    state
        .evaluate_coverage(&ReserveCoverage {
            observed_at_unix_ms: 1,
            bank_as_of_unix_ms: 1,
            reserve_account_id: "reserve-rusd".into(),
            currency: "USD".into(),
            reserve_balance_minor: "9900".into(),
            reserve_balance_usd: "99.00".into(),
            token_supply_raw: "100000000".into(),
            liability_usd: "100".into(),
            surplus_usd: "-1.00".into(),
            ratio_percent: Some(99.0),
            status: CoverageStatus::Undercollateralized,
        })
        .unwrap();
    let token = Arc::new(TestToken::default());
    let bank = Arc::new(FailOnceBank::default());
    let service = RedemptionService::new(
        Arc::new(SqliteRedemptionStore::open(":memory:").unwrap()),
        token.clone(),
        bank.clone(),
        state,
        Arc::new(OperationGate::new(Arc::new(
            SqliteOperationDecisionStore::open(":memory:").unwrap(),
        ))),
    );
    service
        .create(OPERATION_ID.into(), HOLDER.into(), "25000000".into())
        .unwrap();

    assert!(matches!(
        service.settle(OPERATION_ID).await,
        Err(RedemptionError::Bank(_))
    ));
    let after_failure = service.get(OPERATION_ID).unwrap();
    assert_eq!(after_failure.status, RedemptionStatus::Burned);
    assert_eq!(token.0.lock().unwrap().len(), 1);

    let completed = service.settle(OPERATION_ID).await.unwrap();

    assert_eq!(completed.status, RedemptionStatus::Completed);
    assert_eq!(token.0.lock().unwrap().len(), 1);
    assert_eq!(*bank.attempts.lock().unwrap(), 2);
    assert_eq!(
        bank.successful_payouts.lock().unwrap().as_slice(),
        &[(OPERATION_ID.into(), 2500)]
    );
}
