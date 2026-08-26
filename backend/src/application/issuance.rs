use crate::{
    application::AssetStateService,
    domain::{
        BankReserve, CoverageDecisionCode, IssuanceCoverageDecision, IssuanceOrder, IssuanceStatus,
        TokenSnapshot,
    },
};
use alloy::primitives::Address;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

const TOKEN_UNITS_PER_USD_CENT: u64 = 10_000;
pub const ISSUANCE_COVERAGE_POLICY_VERSION: &str = "issuance-coverage-v1";

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
    fn record_coverage_decision(
        &self,
        decision: &IssuanceCoverageDecision,
    ) -> Result<(), IssuanceError>;
}

#[async_trait]
pub trait BankTransactionReader: Send + Sync {
    async fn find(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<ConfirmedBankTransaction>, IssuanceError>;
}

#[async_trait]
pub trait TokenIssuer: Send + Sync {
    async fn mint_for_operation(
        &self,
        operation_id: &str,
        recipient: Address,
        amount_raw: u64,
    ) -> Result<MintResult, IssuanceError>;
}

#[async_trait]
pub trait IssuanceEvidenceReader: Send + Sync {
    async fn read(&self) -> Result<(TokenSnapshot, BankReserve), IssuanceError>;
}

#[derive(Clone)]
pub struct IssuanceService {
    store: Arc<dyn IssuanceStore>,
    bank: Arc<dyn BankTransactionReader>,
    token: Arc<dyn TokenIssuer>,
    evidence: Arc<dyn IssuanceEvidenceReader>,
    asset_state: Arc<AssetStateService>,
    settlement_lock: Arc<tokio::sync::Mutex<()>>,
}

impl IssuanceService {
    pub fn new(
        store: Arc<dyn IssuanceStore>,
        bank: Arc<dyn BankTransactionReader>,
        token: Arc<dyn TokenIssuer>,
        evidence: Arc<dyn IssuanceEvidenceReader>,
        asset_state: Arc<AssetStateService>,
    ) -> Self {
        Self {
            store,
            bank,
            token,
            evidence,
            asset_state,
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
        let confirmed_minor = order
            .amount_usd_minor
            .parse::<u64>()
            .map_err(|_| IssuanceError::Storage("stored fiat amount is invalid".to_owned()))?;
        let proposed_mint_raw = order
            .token_amount_raw
            .parse::<u64>()
            .map_err(|_| IssuanceError::Storage("stored token amount is invalid".to_owned()))?;
        let (token_snapshot, reserve) = match self.evidence.read().await {
            Ok(value) => value,
            Err(error) => {
                let decision = unavailable_decision(
                    &order.operation_id,
                    confirmed_minor,
                    proposed_mint_raw,
                    error.to_string(),
                );
                self.store.record_coverage_decision(&decision)?;
                let _ = self
                    .asset_state
                    .mark_data_unavailable(decision.reason.clone());
                return Err(IssuanceError::CoverageUnavailable(decision.reason));
            }
        };
        let decision = match evaluate_projected_coverage(
            &order.operation_id,
            confirmed_minor,
            proposed_mint_raw,
            &token_snapshot,
            &reserve,
        ) {
            Ok(decision) => decision,
            Err(error) => {
                let decision = unavailable_decision(
                    &order.operation_id,
                    confirmed_minor,
                    proposed_mint_raw,
                    error.to_string(),
                );
                self.store.record_coverage_decision(&decision)?;
                let _ = self
                    .asset_state
                    .mark_data_unavailable(decision.reason.clone());
                return Err(IssuanceError::CoverageUnavailable(decision.reason));
            }
        };
        self.store.record_coverage_decision(&decision)?;
        if decision.decision != CoverageDecisionCode::Accepted {
            let percent = decision
                .projected_coverage_bps
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| value / 100.0);
            let _ = self.asset_state.block_mint(
                decision.reason.clone(),
                percent,
                Some(decision.evaluated_at_unix_ms),
            );
            return Err(IssuanceError::ProjectedCoverage(decision.reason));
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
    #[error("projected reserve coverage rejected issuance: {0}")]
    ProjectedCoverage(String),
    #[error("projected reserve coverage could not be evaluated: {0}")]
    CoverageUnavailable(String),
    #[error("issuance persistence failed: {0}")]
    Storage(String),
    #[error("mock bank request failed: {0}")]
    Bank(String),
    #[error("token mint failed: {0}")]
    Blockchain(String),
}

pub fn evaluate_projected_coverage(
    operation_id: &str,
    confirmed_incoming_minor: u64,
    proposed_mint_raw: u64,
    token: &TokenSnapshot,
    reserve: &BankReserve,
) -> Result<IssuanceCoverageDecision, IssuanceError> {
    if reserve.currency != "USD" {
        return Err(IssuanceError::CoverageUnavailable(format!(
            "expected USD reserve, received {}",
            reserve.currency
        )));
    }
    if token.decimals < 2 {
        return Err(IssuanceError::CoverageUnavailable(
            "token must expose at least two decimals".into(),
        ));
    }
    let reserve_minor = parse_u128(&reserve.balance_minor, "reserve balance")?;
    let incoming_minor = u128::from(confirmed_incoming_minor);
    let pre_operation_minor = reserve_minor.checked_sub(incoming_minor).ok_or_else(|| {
        IssuanceError::CoverageUnavailable(
            "fresh reserve does not contain the confirmed operation deposit".into(),
        )
    })?;
    let supply_raw = parse_u128(&token.total_supply_raw, "token supply")?;
    let scale = 10_u128
        .checked_pow(token.decimals.into())
        .ok_or_else(|| IssuanceError::CoverageUnavailable("token scale overflow".into()))?;
    let reserve_raw = reserve_minor
        .checked_mul(scale)
        .and_then(|value| value.checked_div(100))
        .ok_or_else(|| IssuanceError::CoverageUnavailable("reserve conversion overflow".into()))?;
    let projected_liability = supply_raw
        .checked_add(u128::from(proposed_mint_raw))
        .ok_or_else(|| IssuanceError::CoverageUnavailable("liability overflow".into()))?;
    let current_bps = ratio_bps(reserve_raw, supply_raw);
    let projected_bps = ratio_bps(reserve_raw, projected_liability);
    let accepted = reserve_raw >= projected_liability;
    Ok(IssuanceCoverageDecision {
        operation_id: operation_id.into(),
        decision: if accepted {
            CoverageDecisionCode::Accepted
        } else {
            CoverageDecisionCode::Rejected
        },
        reason: if accepted {
            "Fresh reserve evidence covers the projected post-mint liability".into()
        } else {
            "Projected post-mint reserve coverage would be below 100%".into()
        },
        current_reserve_minor: Some(reserve_minor.to_string()),
        pre_operation_reserve_minor: Some(pre_operation_minor.to_string()),
        confirmed_incoming_minor: confirmed_incoming_minor.to_string(),
        current_supply_raw: Some(supply_raw.to_string()),
        proposed_mint_raw: proposed_mint_raw.to_string(),
        current_coverage_bps: current_bps,
        projected_coverage_bps: projected_bps,
        evidence_block_number: Some(token.block_number),
        bank_as_of_unix_ms: Some(reserve.as_of_unix_ms),
        policy_version: ISSUANCE_COVERAGE_POLICY_VERSION.into(),
        evaluated_at_unix_ms: unix_ms(),
    })
}

fn unavailable_decision(
    operation_id: &str,
    incoming: u64,
    mint: u64,
    reason: String,
) -> IssuanceCoverageDecision {
    IssuanceCoverageDecision {
        operation_id: operation_id.into(),
        decision: CoverageDecisionCode::DataUnavailable,
        reason,
        current_reserve_minor: None,
        pre_operation_reserve_minor: None,
        confirmed_incoming_minor: incoming.to_string(),
        current_supply_raw: None,
        proposed_mint_raw: mint.to_string(),
        current_coverage_bps: None,
        projected_coverage_bps: None,
        evidence_block_number: None,
        bank_as_of_unix_ms: None,
        policy_version: ISSUANCE_COVERAGE_POLICY_VERSION.into(),
        evaluated_at_unix_ms: unix_ms(),
    }
}
fn parse_u128(value: &str, label: &str) -> Result<u128, IssuanceError> {
    value
        .parse()
        .map_err(|_| IssuanceError::CoverageUnavailable(format!("invalid {label}")))
}
fn ratio_bps(numerator: u128, denominator: u128) -> Option<String> {
    if denominator == 0 {
        None
    } else {
        numerator
            .checked_mul(10_000)
            .map(|value| (value / denominator).to_string())
    }
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
        fn fail(&self, _: &str, _: &str) -> Result<(), IssuanceError> {
            Ok(())
        }
        fn record_coverage_decision(
            &self,
            _: &IssuanceCoverageDecision,
        ) -> Result<(), IssuanceError> {
            Ok(())
        }
    }
    struct Bank(Option<ConfirmedBankTransaction>);
    #[async_trait]
    impl BankTransactionReader for Bank {
        async fn find(&self, _: &str) -> Result<Option<ConfirmedBankTransaction>, IssuanceError> {
            Ok(self.0.clone())
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

    struct Evidence;
    #[async_trait]
    impl IssuanceEvidenceReader for Evidence {
        async fn read(&self) -> Result<(TokenSnapshot, BankReserve), IssuanceError> {
            Ok((token_snapshot("0"), bank_reserve("2500")))
        }
    }
    struct StaticEvidence(TokenSnapshot, BankReserve);
    #[async_trait]
    impl IssuanceEvidenceReader for StaticEvidence {
        async fn read(&self) -> Result<(TokenSnapshot, BankReserve), IssuanceError> {
            Ok((self.0.clone(), self.1.clone()))
        }
    }
    struct UnavailableEvidence;
    #[async_trait]
    impl IssuanceEvidenceReader for UnavailableEvidence {
        async fn read(&self) -> Result<(TokenSnapshot, BankReserve), IssuanceError> {
            Err(IssuanceError::CoverageUnavailable(
                "fresh RPC evidence unavailable".into(),
            ))
        }
    }

    fn token_snapshot(supply: &str) -> TokenSnapshot {
        TokenSnapshot {
            chain_id: 31337,
            block_number: 7,
            contract_address: "0x1".into(),
            name: "rUSD".into(),
            symbol: "rUSD".into(),
            decimals: 6,
            total_supply_raw: supply.into(),
        }
    }
    fn bank_reserve(balance_minor: &str) -> BankReserve {
        BankReserve {
            account_id: "reserve-rusd".into(),
            currency: "USD".into(),
            balance_minor: balance_minor.into(),
            version: 1,
            as_of_unix_ms: 2,
        }
    }
    fn asset_state() -> Arc<AssetStateService> {
        Arc::new(AssetStateService::new(
            Arc::new(
                crate::infrastructure::asset_state_sqlite::SqliteAssetStateStore::open(":memory:")
                    .unwrap(),
            ),
            4,
        ))
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
            Arc::new(Bank(None)),
            Arc::new(Token(Mutex::new(0))),
            Arc::new(Evidence),
            asset_state(),
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
            Arc::new(Bank(Some(ConfirmedBankTransaction {
                operation_type: "deposit".to_owned(),
                amount_minor: "2500".to_owned(),
                reference: "order-1".to_owned(),
            }))),
            token.clone(),
            Arc::new(Evidence),
            asset_state(),
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
            Arc::new(Bank(None)),
            token.clone(),
            Arc::new(Evidence),
            asset_state(),
        );
        service.create(command()).unwrap();
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::FiatNotConfirmed)
        ));
        assert_eq!(*token.0.lock().unwrap(), 0);
    }

    #[test]
    fn projected_coverage_accepts_exactly_100_percent_and_rejects_below_it() {
        let exact = evaluate_projected_coverage(
            "op",
            1_000,
            10_000_000,
            &token_snapshot("100000000"),
            &bank_reserve("11000"),
        )
        .unwrap();
        assert_eq!(exact.decision, CoverageDecisionCode::Accepted);
        assert_eq!(exact.pre_operation_reserve_minor.as_deref(), Some("10000"));
        assert_eq!(exact.projected_coverage_bps.as_deref(), Some("10000"));

        let below = evaluate_projected_coverage(
            "op",
            999,
            10_000_000,
            &token_snapshot("100000000"),
            &bank_reserve("10999"),
        )
        .unwrap();
        assert_eq!(below.decision, CoverageDecisionCode::Rejected);
        assert_eq!(below.projected_coverage_bps.as_deref(), Some("9999"));
    }

    #[test]
    fn projected_coverage_rejects_a_large_jump_and_uses_integer_rounding() {
        let decision = evaluate_projected_coverage(
            "large",
            5_000,
            100_000_000,
            &token_snapshot("100000000"),
            &bank_reserve("15000"),
        )
        .unwrap();
        assert_eq!(decision.decision, CoverageDecisionCode::Rejected);
        assert_eq!(decision.projected_coverage_bps.as_deref(), Some("7500"));
    }

    #[test]
    fn projected_coverage_requires_the_confirmed_deposit_in_fresh_reserve() {
        assert!(matches!(
            evaluate_projected_coverage(
                "missing",
                2_500,
                25_000_000,
                &token_snapshot("0"),
                &bank_reserve("2499"),
            ),
            Err(IssuanceError::CoverageUnavailable(_))
        ));
    }

    #[tokio::test]
    async fn concurrent_retries_mint_the_same_operation_once() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let service = Arc::new(IssuanceService::new(
            store,
            Arc::new(Bank(Some(ConfirmedBankTransaction {
                operation_type: "deposit".into(),
                amount_minor: "2500".into(),
                reference: "order-1".into(),
            }))),
            token.clone(),
            Arc::new(Evidence),
            asset_state(),
        ));
        service.create(command()).unwrap();
        let (first, second) = tokio::join!(service.settle("order-1"), service.settle("order-1"));
        assert!(first.is_ok());
        assert!(second.is_ok());
        assert_eq!(*token.0.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn rejected_projection_does_not_claim_or_mint_and_blocks_issuance_state() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let state = asset_state();
        let service = IssuanceService::new(
            store,
            Arc::new(Bank(Some(ConfirmedBankTransaction {
                operation_type: "deposit".into(),
                amount_minor: "2500".into(),
                reference: "order-1".into(),
            }))),
            token.clone(),
            Arc::new(StaticEvidence(
                token_snapshot("100000000"),
                bank_reserve("10000"),
            )),
            state.clone(),
        );
        service.create(command()).unwrap();
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::ProjectedCoverage(_))
        ));
        assert_eq!(
            service.get("order-1").unwrap().status,
            IssuanceStatus::AwaitingFiat
        );
        assert_eq!(*token.0.lock().unwrap(), 0);
        assert_eq!(
            state.current().unwrap().state,
            crate::domain::AssetStateCode::MintBlocked
        );
    }

    #[tokio::test]
    async fn unavailable_fresh_evidence_fails_closed_without_minting() {
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let token = Arc::new(Token(Mutex::new(0)));
        let service = IssuanceService::new(
            store,
            Arc::new(Bank(Some(ConfirmedBankTransaction {
                operation_type: "deposit".into(),
                amount_minor: "2500".into(),
                reference: "order-1".into(),
            }))),
            token.clone(),
            Arc::new(UnavailableEvidence),
            asset_state(),
        );
        service.create(command()).unwrap();
        assert!(matches!(
            service.settle("order-1").await,
            Err(IssuanceError::CoverageUnavailable(_))
        ));
        assert_eq!(*token.0.lock().unwrap(), 0);
    }
}
