use crate::{
    application::{AssetStateService, OperationGate},
    domain::{IssuerOperationKind, OperationDecisionOutcome, RedemptionOrder, RedemptionStatus},
};
use alloy::primitives::Address;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
const TOKEN_UNITS_PER_CENT: u64 = 10_000;
pub trait RedemptionStore: Send + Sync {
    fn create(&self, order: &RedemptionOrder) -> Result<RedemptionOrder, RedemptionError>;
    fn get(&self, id: &str) -> Result<Option<RedemptionOrder>, RedemptionError>;
    fn mark_burned(&self, id: &str, hash: Option<&str>)
    -> Result<RedemptionOrder, RedemptionError>;
    fn complete(&self, id: &str) -> Result<RedemptionOrder, RedemptionError>;
    fn fail(&self, id: &str, message: &str) -> Result<(), RedemptionError>;
}
#[async_trait]
pub trait RedemptionToken: Send + Sync {
    async fn burn_for_operation(
        &self,
        id: &str,
        holder: Address,
        amount: u64,
    ) -> Result<Option<String>, RedemptionError>;
}
#[async_trait]
pub trait PayoutBank: Send + Sync {
    async fn pay_usd(&self, id: &str, amount_minor: u64) -> Result<(), RedemptionError>;
}
#[derive(Clone)]
pub struct RedemptionService {
    store: Arc<dyn RedemptionStore>,
    token: Arc<dyn RedemptionToken>,
    bank: Arc<dyn PayoutBank>,
    asset_state: Arc<AssetStateService>,
    operation_gate: Arc<OperationGate>,
    lock: Arc<tokio::sync::Mutex<()>>,
}
impl RedemptionService {
    pub fn new(
        store: Arc<dyn RedemptionStore>,
        token: Arc<dyn RedemptionToken>,
        bank: Arc<dyn PayoutBank>,
        asset_state: Arc<AssetStateService>,
        operation_gate: Arc<OperationGate>,
    ) -> Self {
        Self {
            store,
            token,
            bank,
            asset_state,
            operation_gate,
            lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
    pub fn create(
        &self,
        id: String,
        holder: String,
        amount_raw: String,
    ) -> Result<RedemptionOrder, RedemptionError> {
        let id = id.trim();
        if id.is_empty() || id.len() > 128 {
            return Err(RedemptionError::Invalid(
                "operationId must contain 1-128 characters".into(),
            ));
        }
        let address: Address = holder
            .parse()
            .map_err(|_| RedemptionError::Invalid("holderAddress is invalid".into()))?;
        let amount: u64 =
            amount_raw.parse().ok().filter(|v| *v > 0).ok_or_else(|| {
                RedemptionError::Invalid("tokenAmountRaw must be positive".into())
            })?;
        if !amount.is_multiple_of(TOKEN_UNITS_PER_CENT) {
            return Err(RedemptionError::Invalid(
                "tokenAmountRaw must represent whole USD cents".into(),
            ));
        }
        let now = unix_ms();
        self.store.create(&RedemptionOrder {
            operation_id: id.to_owned(),
            holder_address: address.to_checksum(None),
            token_amount_raw: amount.to_string(),
            payout_usd_minor: (amount / TOKEN_UNITS_PER_CENT).to_string(),
            status: RedemptionStatus::Created,
            burn_transaction_hash: None,
            payout_reference: format!("redemption-{id}"),
            last_error: None,
            created_at_unix_ms: now,
            updated_at_unix_ms: now,
        })
    }
    pub fn get(&self, id: &str) -> Result<RedemptionOrder, RedemptionError> {
        self.store.get(id)?.ok_or(RedemptionError::NotFound)
    }
    pub async fn settle(&self, id: &str) -> Result<RedemptionOrder, RedemptionError> {
        let _guard = self.lock.lock().await;
        let mut order = self.get(id)?;
        if order.status == RedemptionStatus::Completed {
            return Ok(order);
        }
        let state = self
            .asset_state
            .current()
            .map_err(|error| RedemptionError::Gate(error.to_string()))?;
        let decision = self
            .operation_gate
            .decide(id, IssuerOperationKind::Redemption, &state)
            .map_err(|error| RedemptionError::Gate(error.to_string()))?;
        if decision.outcome == OperationDecisionOutcome::Rejected {
            return Err(RedemptionError::Gate(decision.reason));
        }
        if order.status != RedemptionStatus::Burned {
            let holder = order
                .holder_address
                .parse()
                .map_err(|_| RedemptionError::Storage("stored holder is invalid".into()))?;
            let amount = order
                .token_amount_raw
                .parse()
                .map_err(|_| RedemptionError::Storage("stored amount is invalid".into()))?;
            match self.token.burn_for_operation(id, holder, amount).await {
                Ok(hash) => order = self.store.mark_burned(id, hash.as_deref())?,
                Err(e) => {
                    let _ = self.store.fail(id, &e.to_string());
                    return Err(e);
                }
            }
        }
        let payout = order
            .payout_usd_minor
            .parse()
            .map_err(|_| RedemptionError::Storage("stored payout is invalid".into()))?;
        match self.bank.pay_usd(id, payout).await {
            Ok(()) => self.store.complete(id),
            Err(e) => {
                let _ = self.store.fail(id, &e.to_string());
                Err(e)
            }
        }
    }
}
#[derive(Debug, Error)]
pub enum RedemptionError {
    #[error("invalid redemption request: {0}")]
    Invalid(String),
    #[error("redemption order was not found")]
    NotFound,
    #[error("operationId is already associated with a different redemption")]
    IdempotencyConflict,
    #[error("redemption persistence failed: {0}")]
    Storage(String),
    #[error("redemption burn failed: {0}")]
    Blockchain(String),
    #[error("redemption payout failed: {0}")]
    Bank(String),
    #[error("redemption operation gate failed: {0}")]
    Gate(String),
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::OperationGate,
        domain::{CoverageStatus, ReserveCoverage},
        infrastructure::{
            asset_state_sqlite::SqliteAssetStateStore,
            operation_decision_sqlite::SqliteOperationDecisionStore,
            redemption_sqlite::SqliteRedemptionStore,
        },
    };
    use std::sync::Mutex;

    struct Token(Mutex<u8>);

    #[async_trait]
    impl RedemptionToken for Token {
        async fn burn_for_operation(
            &self,
            _: &str,
            _: Address,
            _: u64,
        ) -> Result<Option<String>, RedemptionError> {
            *self.0.lock().unwrap() += 1;
            Ok(Some("0xburn".into()))
        }
    }

    struct Bank(Mutex<Vec<u64>>);

    #[async_trait]
    impl PayoutBank for Bank {
        async fn pay_usd(&self, _: &str, amount_minor: u64) -> Result<(), RedemptionError> {
            self.0.lock().unwrap().push(amount_minor);
            Ok(())
        }
    }

    fn asset_state(ratio_percent: f64) -> Arc<AssetStateService> {
        let service = Arc::new(AssetStateService::new(
            Arc::new(SqliteAssetStateStore::open(":memory:").unwrap()),
            4,
        ));
        service
            .evaluate_coverage(&ReserveCoverage {
                observed_at_unix_ms: 1,
                bank_as_of_unix_ms: 1,
                reserve_account_id: "reserve-rusd".into(),
                currency: "USD".into(),
                reserve_balance_minor: "99".into(),
                reserve_balance_usd: "0.99".into(),
                token_supply_raw: "1000000".into(),
                liability_usd: "1".into(),
                surplus_usd: "-0.01".into(),
                ratio_percent: Some(ratio_percent),
                status: CoverageStatus::Undercollateralized,
            })
            .unwrap();
        service
    }

    #[tokio::test]
    async fn shortfall_does_not_block_redemption_and_payout_stays_at_par() {
        let token = Arc::new(Token(Mutex::new(0)));
        let bank = Arc::new(Bank(Mutex::new(Vec::new())));
        let service = RedemptionService::new(
            Arc::new(SqliteRedemptionStore::open(":memory:").unwrap()),
            token.clone(),
            bank.clone(),
            asset_state(99.0),
            Arc::new(OperationGate::new(Arc::new(
                SqliteOperationDecisionStore::open(":memory:").unwrap(),
            ))),
        );
        service
            .create(
                "redemption-1".into(),
                "0x0000000000000000000000000000000000000001".into(),
                "25000000".into(),
            )
            .unwrap();

        let settled = service.settle("redemption-1").await.unwrap();

        assert_eq!(settled.status, RedemptionStatus::Completed);
        assert_eq!(*token.0.lock().unwrap(), 1);
        assert_eq!(*bank.0.lock().unwrap(), vec![2500]);
    }
}
