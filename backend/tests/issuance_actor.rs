//! Integracyjne testy emitenta na poziomie aktora.
//!
//! Łączą rzeczywistą usługę emisji z produkcyjnymi adapterami SQLite. Bank i
//! kontrakt są kontrolowanymi granicami testowymi — ich rejestry wywołań są
//! asercją efektu, którego nie może wykonać sama baza danych.

use alloy::primitives::Address;
use async_trait::async_trait;
use crypto_asset_backend::{
    application::{
        AssetStateService, BankTransactionReader, ConfirmedBankTransaction, CreateIssuance,
        IssuanceError, IssuanceService, MintResult, OperationGate, TokenIssuer,
    },
    domain::{CoverageStatus, IssuanceStatus, ReserveCoverage},
    infrastructure::{
        asset_state_sqlite::SqliteAssetStateStore, issuance_sqlite::SqliteIssuanceStore,
        operation_decision_sqlite::SqliteOperationDecisionStore,
    },
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

const OPERATION_ID: &str = "integration-issuance-1";
const RECIPIENT: &str = "0x0000000000000000000000000000000000000001";
static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct TestBank {
    confirmed: Mutex<Option<ConfirmedBankTransaction>>,
    refunds: Mutex<Vec<(String, u64)>>,
}

impl TestBank {
    fn confirm_matching_deposit(&self) {
        *self.confirmed.lock().unwrap() = Some(ConfirmedBankTransaction {
            operation_type: "deposit".into(),
            amount_minor: "1250".into(),
            reference: OPERATION_ID.into(),
        });
    }
}

#[async_trait]
impl BankTransactionReader for TestBank {
    async fn find(&self, _: &str) -> Result<Option<ConfirmedBankTransaction>, IssuanceError> {
        Ok(self.confirmed.lock().unwrap().clone())
    }

    async fn refund_to_casp(
        &self,
        operation_id: &str,
        amount_minor: u64,
    ) -> Result<(), IssuanceError> {
        self.refunds
            .lock()
            .unwrap()
            .push((operation_id.to_owned(), amount_minor));
        Ok(())
    }
}

#[derive(Default)]
struct TestToken {
    mints: Mutex<Vec<(String, Address, u64)>>,
}

#[async_trait]
impl TokenIssuer for TestToken {
    async fn mint_for_operation(
        &self,
        operation_id: &str,
        recipient: Address,
        amount_raw: u64,
    ) -> Result<MintResult, IssuanceError> {
        self.mints
            .lock()
            .unwrap()
            .push((operation_id.to_owned(), recipient, amount_raw));
        Ok(MintResult {
            transaction_hash: Some("0xintegrationmint".into()),
        })
    }
}

struct Fixture {
    service: Arc<IssuanceService>,
    state: Arc<AssetStateService>,
    bank: Arc<TestBank>,
    token: Arc<TestToken>,
    _database_directory: std::path::PathBuf,
}

impl Fixture {
    fn new(coverage_percent: f64) -> Self {
        let directory = unique_test_directory();
        let path = directory.join("issuer.sqlite");
        let path = path.to_string_lossy().into_owned();

        let state = Arc::new(AssetStateService::new(
            Arc::new(SqliteAssetStateStore::open(&path).unwrap()),
            4,
        ));
        state
            .evaluate_coverage(&coverage(coverage_percent))
            .unwrap();

        let bank = Arc::new(TestBank::default());
        let token = Arc::new(TestToken::default());
        let service = Arc::new(IssuanceService::new(
            Arc::new(SqliteIssuanceStore::open(&path).unwrap()),
            bank.clone(),
            token.clone(),
            state.clone(),
            Arc::new(OperationGate::new(Arc::new(
                SqliteOperationDecisionStore::open(&path).unwrap(),
            ))),
        ));

        Self {
            service,
            state,
            bank,
            token,
            _database_directory: directory,
        }
    }

    fn create_order(&self) {
        self.service
            .create(CreateIssuance {
                operation_id: OPERATION_ID.into(),
                recipient_address: RECIPIENT.into(),
                amount_usd_minor: "1250".into(),
            })
            .unwrap();
    }
}

fn coverage(percent: f64) -> ReserveCoverage {
    ReserveCoverage {
        observed_at_unix_ms: 1,
        bank_as_of_unix_ms: 1,
        reserve_account_id: "reserve-rusd".into(),
        currency: "USD".into(),
        reserve_balance_minor: "125000".into(),
        reserve_balance_usd: "1250.00".into(),
        token_supply_raw: "100000000".into(),
        liability_usd: "100".into(),
        surplus_usd: "0".into(),
        ratio_percent: Some(percent),
        status: if percent >= 100.0 {
            CoverageStatus::Covered
        } else {
            CoverageStatus::Undercollateralized
        },
    }
}

fn unique_test_directory() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "rusd-issuer-integration-{}-{nonce}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[tokio::test]
async fn issuance_is_persisted_before_fiat_and_minted_once_after_confirmation() {
    let fixture = Fixture::new(110.0);

    let created = fixture
        .service
        .create(CreateIssuance {
            operation_id: OPERATION_ID.into(),
            recipient_address: RECIPIENT.into(),
            amount_usd_minor: "1250".into(),
        })
        .unwrap();
    let replayed = fixture
        .service
        .create(CreateIssuance {
            operation_id: OPERATION_ID.into(),
            recipient_address: RECIPIENT.into(),
            amount_usd_minor: "1250".into(),
        })
        .unwrap();

    assert_eq!(created, replayed);
    assert_eq!(created.status, IssuanceStatus::AwaitingFiat);
    assert_eq!(created.token_amount_raw, "12500000");
    assert!(matches!(
        fixture.service.settle(OPERATION_ID).await,
        Err(IssuanceError::FiatNotConfirmed)
    ));
    assert!(fixture.token.mints.lock().unwrap().is_empty());

    fixture.bank.confirm_matching_deposit();
    let completed = fixture.service.settle(OPERATION_ID).await.unwrap();
    let replayed_settlement = fixture.service.settle(OPERATION_ID).await.unwrap();

    assert_eq!(completed.status, IssuanceStatus::Completed);
    assert_eq!(completed, replayed_settlement);
    assert_eq!(
        fixture.token.mints.lock().unwrap().as_slice(),
        &[(OPERATION_ID.into(), RECIPIENT.parse().unwrap(), 12_500_000,)]
    );
}

#[tokio::test]
async fn concurrent_settlement_of_one_operation_mints_only_once() {
    let fixture = Fixture::new(110.0);
    fixture.create_order();
    fixture.bank.confirm_matching_deposit();

    let first = fixture.service.clone();
    let second = fixture.service.clone();
    let (left, right) = tokio::join!(
        async move { first.settle(OPERATION_ID).await },
        async move { second.settle(OPERATION_ID).await },
    );

    assert_eq!(left.unwrap().status, IssuanceStatus::Completed);
    assert_eq!(right.unwrap().status, IssuanceStatus::Completed);
    assert_eq!(fixture.token.mints.lock().unwrap().len(), 1);
    assert_eq!(
        fixture
            .service
            .get(OPERATION_ID)
            .unwrap()
            .transaction_hash
            .as_deref(),
        Some("0xintegrationmint")
    );
}

#[tokio::test]
async fn reserve_state_blocks_issuance_refunds_fiat_and_allows_recovery() {
    let fixture = Fixture::new(99.0);
    fixture.create_order();
    fixture.bank.confirm_matching_deposit();

    assert!(matches!(
        fixture.service.settle(OPERATION_ID).await,
        Err(IssuanceError::IssuanceBlocked(_))
    ));
    assert!(fixture.token.mints.lock().unwrap().is_empty());
    assert_eq!(
        fixture.bank.refunds.lock().unwrap().as_slice(),
        &[(OPERATION_ID.into(), 1250)]
    );
    assert_eq!(
        fixture.service.get(OPERATION_ID).unwrap().status,
        IssuanceStatus::Failed
    );

    fixture.state.evaluate_coverage(&coverage(105.0)).unwrap();
    let new_operation = "integration-issuance-after-recovery";
    fixture
        .service
        .create(CreateIssuance {
            operation_id: new_operation.into(),
            recipient_address: RECIPIENT.into(),
            amount_usd_minor: "1250".into(),
        })
        .unwrap();
    fixture.bank.confirm_matching_deposit();

    // The test bank confirmation must use the idempotency reference of the
    // current order; replace it after validating the first rejected order.
    *fixture.bank.confirmed.lock().unwrap() = Some(ConfirmedBankTransaction {
        operation_type: "deposit".into(),
        amount_minor: "1250".into(),
        reference: new_operation.into(),
    });
    assert_eq!(
        fixture.service.settle(new_operation).await.unwrap().status,
        IssuanceStatus::Completed
    );
    assert_eq!(fixture.token.mints.lock().unwrap().len(), 1);
}
