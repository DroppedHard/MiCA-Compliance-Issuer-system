use crate::{
    application::{AssetStateService, OperationGate},
    domain::{IssuanceOrder, IssuanceStatus, IssuerOperationKind, OperationDecisionOutcome},
};
use alloy::primitives::Address;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

const TOKEN_UNITS_PER_USD_CENT: u64 = 10_000;

#[derive(Debug, Clone)]
pub struct CreateIssuance {
    pub operation_id: String,
    pub recipient_address: String,
    pub amount_usd_minor: String,
}

#[derive(Debug, Clone)]
pub struct ConfirmedBankTransaction {
    pub operation_type: String,
    pub amount_minor: String,
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct MintResult {
    pub transaction_hash: Option<String>,
}

pub trait IssuanceStore: Send + Sync {
    fn create(&self, order: &IssuanceOrder) -> Result<IssuanceOrder, IssuanceError>;
    fn get(&self, operation_id: &str) -> Result<Option<IssuanceOrder>, IssuanceError>;
    fn claim_for_mint(&self, operation_id: &str) -> Result<IssuanceOrder, IssuanceError>;
    fn complete(
        &self,
        operation_id: &str,
        transaction_hash: Option<&str>,
    ) -> Result<IssuanceOrder, IssuanceError>;
    fn fail(&self, operation_id: &str, message: &str) -> Result<(), IssuanceError>;
}

#[async_trait]
pub trait BankTransactionReader: Send + Sync {
    async fn find(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<ConfirmedBankTransaction>, IssuanceError>;

    /// Compensates a confirmed CASP payment when issuance cannot proceed.
    /// The implementation must be idempotent for one operation ID.
    async fn refund_to_casp(
        &self,
        operation_id: &str,
        amount_minor: u64,
    ) -> Result<(), IssuanceError>;
}

const REFUNDED_MARKER: &str = "fiat_refunded_to_casp:";

#[async_trait]
pub trait TokenIssuer: Send + Sync {
    async fn mint_for_operation(
        &self,
        operation_id: &str,
        recipient: Address,
        amount_raw: u64,
    ) -> Result<MintResult, IssuanceError>;
}

#[derive(Clone)]
pub struct IssuanceService {
    store: Arc<dyn IssuanceStore>,
    bank: Arc<dyn BankTransactionReader>,
    token: Arc<dyn TokenIssuer>,
    asset_state: Arc<AssetStateService>,
    operation_gate: Arc<OperationGate>,
    settlement_lock: Arc<tokio::sync::Mutex<()>>,
}

impl IssuanceService {
    pub fn new(
        store: Arc<dyn IssuanceStore>,
        bank: Arc<dyn BankTransactionReader>,
        token: Arc<dyn TokenIssuer>,
        asset_state: Arc<AssetStateService>,
        operation_gate: Arc<OperationGate>,
    ) -> Self {
        Self {
            store,
            bank,
            token,
            asset_state,
            operation_gate,
            settlement_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn create(&self, command: CreateIssuance) -> Result<IssuanceOrder, IssuanceError> {
        let operation_id = command.operation_id.trim();
        if operation_id.is_empty() || operation_id.len() > 128 {
            return Err(IssuanceError::Invalid(
                "operationId must contain 1-128 characters".to_owned(),
            ));
        }
        let recipient = command.recipient_address.parse::<Address>().map_err(|_| {
            IssuanceError::Invalid("recipientAddress is not a valid Ethereum address".to_owned())
        })?;
        let amount = command
            .amount_usd_minor
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                IssuanceError::Invalid(
                    "amountUsdMinor must be a positive integer string".to_owned(),
                )
            })?;
        let token_amount = amount
            .checked_mul(TOKEN_UNITS_PER_USD_CENT)
            .ok_or_else(|| IssuanceError::Invalid("amountUsdMinor is too large".to_owned()))?;
        let now = unix_ms();
        self.store.create(&IssuanceOrder {
            operation_id: operation_id.to_owned(),
            recipient_address: recipient.to_checksum(None),
            amount_usd_minor: amount.to_string(),
            token_amount_raw: token_amount.to_string(),
            bank_idempotency_key: format!("issuance-{operation_id}"),
            status: IssuanceStatus::AwaitingFiat,
            transaction_hash: None,
            last_error: None,
            created_at_unix_ms: now,
            updated_at_unix_ms: now,
        })
    }

    pub fn get(&self, operation_id: &str) -> Result<IssuanceOrder, IssuanceError> {
        self.store.get(operation_id)?.ok_or(IssuanceError::NotFound)
    }

    pub async fn settle(&self, operation_id: &str) -> Result<IssuanceOrder, IssuanceError> {
        // The demo runs one issuer process. Serializing settlement removes the
        // preflight/send race while the contract operation ID protects restarts.
        let _settlement_guard = self.settlement_lock.lock().await;
        let order = self.get(operation_id)?;
        if order.status == IssuanceStatus::Completed {
            return Ok(order);
        }
        if order
            .last_error
            .as_deref()
            .is_some_and(|message| message.starts_with(REFUNDED_MARKER))
        {
            return Err(IssuanceError::IssuanceBlocked(
                "wpłata fiat została już zwrócona CASP; wymagane jest nowe zlecenie".to_owned(),
            ));
        }
        let bank = self
            .bank
            .find(&order.bank_idempotency_key)
            .await?
            .ok_or(IssuanceError::FiatNotConfirmed)?;
        if bank.operation_type != "deposit"
            || bank.amount_minor != order.amount_usd_minor
            || bank.reference != order.operation_id
        {
            return Err(IssuanceError::BankMismatch);
        }
        let state = self.asset_state.current().map_err(|error| {
            IssuanceError::IssuanceBlocked(format!("asset state is unavailable: {error}"))
        })?;
        let decision = self
            .operation_gate
            .decide(operation_id, IssuerOperationKind::Issuance, &state)
            .map_err(|error| IssuanceError::Storage(error.to_string()))?;
        if decision.outcome == OperationDecisionOutcome::Rejected {
            let amount = order
                .amount_usd_minor
                .parse::<u64>()
                .map_err(|_| IssuanceError::Storage("stored fiat amount is invalid".to_owned()))?;
            self.bank.refund_to_casp(operation_id, amount).await?;
            self.store.fail(
                operation_id,
                &format!("{REFUNDED_MARKER} {}", decision.reason),
            )?;
            return Err(IssuanceError::IssuanceBlocked(format!(
                "{}; wpłata fiat została zwrócona CASP",
                decision.reason
            )));
        }
        let claimed = self.store.claim_for_mint(operation_id)?;
        let recipient = claimed.recipient_address.parse::<Address>().map_err(|_| {
            IssuanceError::Storage("stored recipient address is invalid".to_owned())
        })?;
        let amount = claimed
            .token_amount_raw
            .parse::<u64>()
            .map_err(|_| IssuanceError::Storage("stored token amount is invalid".to_owned()))?;
        match self
            .token
            .mint_for_operation(operation_id, recipient, amount)
            .await
        {
            Ok(result) => self
                .store
                .complete(operation_id, result.transaction_hash.as_deref()),
            Err(error) => {
                let _ = self.store.fail(operation_id, &error.to_string());
                Err(error)
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum IssuanceError {
    #[error("invalid issuance request: {0}")]
    Invalid(String),
    #[error("issuance operation was not found")]
    NotFound,
    #[error("operationId is already associated with a different request")]
    IdempotencyConflict,
    #[error("fiat deposit has not been confirmed yet")]
    FiatNotConfirmed,
    #[error("confirmed bank transaction does not match the issuance order")]
    BankMismatch,
    #[error("issuance operation is already being settled")]
    SettlementInProgress,
    #[error("issuance is blocked: {0}")]
    IssuanceBlocked(String),
    #[error("issuance persistence failed: {0}")]
    Storage(String),
    #[error("mock bank request failed: {0}")]
    Bank(String),
    #[error("token mint failed: {0}")]
    Blockchain(String),
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
    use crate::application::{OperationDecisionStore, OperationGateError};
    use crate::domain::OperationDecision;
    use std::sync::Mutex;

    struct MemoryStore(Mutex<Option<IssuanceOrder>>);
    impl IssuanceStore for MemoryStore {
        fn create(&self, order: &IssuanceOrder) -> Result<IssuanceOrder, IssuanceError> {
            let mut stored = self.0.lock().unwrap();
            if let Some(existing) = stored.as_ref() {
                if existing.recipient_address == order.recipient_address
                    && existing.amount_usd_minor == order.amount_usd_minor
                {
                    return Ok(existing.clone());
                }
                return Err(IssuanceError::IdempotencyConflict);
            }
            *stored = Some(order.clone());
            Ok(order.clone())
        }
        fn get(&self, _: &str) -> Result<Option<IssuanceOrder>, IssuanceError> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn claim_for_mint(&self, _: &str) -> Result<IssuanceOrder, IssuanceError> {
            let mut value = self.0.lock().unwrap();
            let order = value.as_mut().unwrap();
            order.status = IssuanceStatus::Minting;
            Ok(order.clone())
        }
        fn complete(&self, _: &str, hash: Option<&str>) -> Result<IssuanceOrder, IssuanceError> {
            let mut value = self.0.lock().unwrap();
            let order = value.as_mut().unwrap();
            order.status = IssuanceStatus::Completed;
            order.transaction_hash = hash.map(str::to_owned);
            Ok(order.clone())
        }
        fn fail(&self, _: &str, message: &str) -> Result<(), IssuanceError> {
            let mut value = self.0.lock().unwrap();
            let order = value.as_mut().unwrap();
            order.status = IssuanceStatus::Failed;
            order.last_error = Some(message.to_owned());
            Ok(())
        }
    }
    struct Bank {
        transaction: Option<ConfirmedBankTransaction>,
        refunds: Mutex<Vec<(String, u64)>>,
    }
    impl Bank {
        fn new(transaction: Option<ConfirmedBankTransaction>) -> Self {
            Self {
                transaction,
                refunds: Mutex::new(Vec::new()),
            }
        }
    }
    #[async_trait]
    impl BankTransactionReader for Bank {
        async fn find(&self, _: &str) -> Result<Option<ConfirmedBankTransaction>, IssuanceError> {
            Ok(self.transaction.clone())
        }
        async fn refund_to_casp(
            &self,
            operation_id: &str,
            amount_minor: u64,
        ) -> Result<(), IssuanceError> {
            let mut refunds = self.refunds.lock().unwrap();
            let refund = (operation_id.to_owned(), amount_minor);
            if !refunds.contains(&refund) {
                refunds.push(refund);
            }
            Ok(())
        }
    }
    struct Token(Mutex<u8>);
    #[async_trait]
    impl TokenIssuer for Token {
        async fn mint_for_operation(
            &self,
            _: &str,
            _: Address,
            _: u64,
        ) -> Result<MintResult, IssuanceError> {
            *self.0.lock().unwrap() += 1;
            Ok(MintResult {
                transaction_hash: Some("0xabc".to_owned()),
            })
        }
    }

    struct Decisions;
    impl OperationDecisionStore for Decisions {
        fn append(&self, _: &OperationDecision) -> Result<(), OperationGateError> {
            Ok(())
        }
    }

    fn operation_gate() -> Arc<OperationGate> {
        Arc::new(OperationGate::new(Arc::new(Decisions)))
    }

    fn asset_state(ratio_percent: Option<f64>) -> Arc<AssetStateService> {
        let service = Arc::new(AssetStateService::new(
            Arc::new(
                crate::infrastructure::asset_state_sqlite::SqliteAssetStateStore::open(":memory:")
                    .unwrap(),
            ),
            4,
        ));
        if let Some(ratio_percent) = ratio_percent {
            service
                .evaluate_coverage(&crate::domain::ReserveCoverage {
                    observed_at_unix_ms: 1,
                    bank_as_of_unix_ms: 1,
                    reserve_account_id: "reserve-rusd".into(),
                    currency: "USD".into(),
                    reserve_balance_minor: "100".into(),
                    reserve_balance_usd: "1.00".into(),
                    token_supply_raw: "1000000".into(),
                    liability_usd: "1".into(),
                    surplus_usd: "0".into(),
                    ratio_percent: Some(ratio_percent),
                    status: if ratio_percent >= 100.0 {
                        crate::domain::CoverageStatus::Covered
                    } else {
                        crate::domain::CoverageStatus::Undercollateralized
                    },
                })
                .unwrap();
        }
        service
    }

    fn command() -> CreateIssuance {
        CreateIssuance {
            operation_id: "order-1".to_owned(),
            recipient_address: "0x0000000000000000000000000000000000000001".to_owned(),
            amount_usd_minor: "2500".to_owned(),
        }
    }

    #[test]
    fn create_is_idempotent_and_converts_usd_cents_to_six_decimal_token_units() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let service = IssuanceService::new(
            store,
            Arc::new(Bank::new(None)),
            Arc::new(Token(Mutex::new(0))),
            asset_state(Some(110.0)),
            operation_gate(),
        );
        let first = service.create(command()).unwrap();
        let second = service.create(command()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.token_amount_raw, "25000000");
        assert_eq!(first.bank_idempotency_key, "issuance-order-1");
    }

    #[tokio::test]
    async fn settlement_requires_matching_fiat_before_minting_once() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let service = IssuanceService::new(
            store,
            Arc::new(Bank::new(Some(ConfirmedBankTransaction {
                operation_type: "deposit".to_owned(),
                amount_minor: "2500".to_owned(),
                reference: "order-1".to_owned(),
            }))),
            token.clone(),
            asset_state(Some(110.0)),
            operation_gate(),
        );
        service.create(command()).unwrap();
        let result = service.settle("order-1").await.unwrap();
        assert_eq!(result.status, IssuanceStatus::Completed);
        assert_eq!(*token.0.lock().unwrap(), 1);
        assert_eq!(
            service
                .settle("order-1")
                .await
                .unwrap()
                .transaction_hash
                .as_deref(),
            Some("0xabc")
        );
        assert_eq!(*token.0.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn settlement_does_not_mint_before_fiat_confirmation() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let service = IssuanceService::new(
            store,
            Arc::new(Bank::new(None)),
            token.clone(),
            asset_state(Some(110.0)),
            operation_gate(),
        );
        service.create(command()).unwrap();
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::FiatNotConfirmed)
        ));
        assert_eq!(*token.0.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn mismatched_confirmed_fiat_does_not_mint() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let service = IssuanceService::new(
            store,
            Arc::new(Bank::new(Some(ConfirmedBankTransaction {
                operation_type: "deposit".into(),
                amount_minor: "2499".into(),
                reference: "order-1".into(),
            }))),
            token.clone(),
            asset_state(Some(110.0)),
            operation_gate(),
        );
        service.create(command()).unwrap();
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::BankMismatch)
        ));
        assert_eq!(*token.0.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn blocked_state_prevents_claim_and_mint_after_confirmed_fiat() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let bank = Arc::new(Bank::new(Some(ConfirmedBankTransaction {
            operation_type: "deposit".into(),
            amount_minor: "2500".into(),
            reference: "order-1".into(),
        })));
        let service = IssuanceService::new(
            store,
            bank.clone(),
            token.clone(),
            asset_state(Some(99.0)),
            operation_gate(),
        );
        service.create(command()).unwrap();
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::IssuanceBlocked(_))
        ));
        assert_eq!(
            service.get("order-1").unwrap().status,
            IssuanceStatus::Failed
        );
        assert_eq!(*token.0.lock().unwrap(), 0);
        assert_eq!(
            *bank.refunds.lock().unwrap(),
            vec![("order-1".to_owned(), 2500)]
        );
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::IssuanceBlocked(_))
        ));
        assert_eq!(bank.refunds.lock().unwrap().len(), 1);
    }
}
