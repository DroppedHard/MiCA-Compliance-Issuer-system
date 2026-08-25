use crate::domain::{RedemptionOrder, RedemptionStatus};
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
    lock: Arc<tokio::sync::Mutex<()>>,
}
impl RedemptionService {
    pub fn new(
        store: Arc<dyn RedemptionStore>,
        token: Arc<dyn RedemptionToken>,
        bank: Arc<dyn PayoutBank>,
    ) -> Self {
        Self {
            store,
            token,
            bank,
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
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
