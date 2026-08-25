use crate::domain::BankReserve;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const ACCOUNT_ID: &str = "reserve-rusd";

pub struct MockBankStore {
    connection: Mutex<Connection>,
}
impl MockBankStore {
    pub fn open(path: &str, initial_balance_minor: u64) -> Result<Self, MockBankError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!("../migrations/mock_bank.sql"))
            .map_err(storage)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO reserve_accounts VALUES (?1,'USD',?2,1,?3)",
                params![ACCOUNT_ID, initial_balance_minor as i64, unix_ms() as i64],
            )
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    pub fn reserve(&self) -> Result<BankReserve, MockBankError> {
        self.connection.lock().map_err(storage)?.query_row("SELECT account_id,currency,balance_minor,version,updated_at_unix_ms FROM reserve_accounts WHERE account_id=?1", [ACCOUNT_ID], |row| Ok(BankReserve { account_id: row.get(0)?, currency: row.get(1)?, balance_minor: row.get::<_, i64>(2)?.to_string(), version: row.get::<_, i64>(3)? as u64, as_of_unix_ms: row.get::<_, i64>(4)? as u64 })).map_err(storage)
    }
    pub fn apply(
        &self,
        operation: Operation,
        request: ReserveOperationRequest,
    ) -> Result<BankReserve, MockBankError> {
        let amount = request
            .amount_minor
            .parse::<i64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or(MockBankError::InvalidAmount)?;
        if request.idempotency_key.trim().is_empty() || request.reference.trim().is_empty() {
            return Err(MockBankError::InvalidRequest);
        }
        let mut connection = self.connection.lock().map_err(storage)?;
        let tx = connection.transaction().map_err(storage)?;
        let existing = tx
            .query_row(
                "SELECT operation_type,amount_minor,reference FROM reserve_transactions WHERE idempotency_key=?1",
                [&request.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()
            .map_err(storage)?;
        if let Some((existing_operation, existing_amount, existing_reference)) = existing {
            if existing_operation != operation.as_str()
                || existing_amount != amount
                || existing_reference != request.reference
            {
                return Err(MockBankError::IdempotencyConflict);
            }
            drop(tx);
            drop(connection);
            return self.reserve();
        }
        let current: i64 = tx
            .query_row(
                "SELECT balance_minor FROM reserve_accounts WHERE account_id=?1",
                [ACCOUNT_ID],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let next = match operation {
            Operation::Deposit => current.checked_add(amount),
            Operation::Withdrawal if amount <= current => current.checked_sub(amount),
            Operation::Withdrawal => return Err(MockBankError::InsufficientFunds),
        }
        .ok_or(MockBankError::InvalidAmount)?;
        let now = unix_ms() as i64;
        tx.execute("UPDATE reserve_accounts SET balance_minor=?1,version=version+1,updated_at_unix_ms=?2 WHERE account_id=?3", params![next,now,ACCOUNT_ID]).map_err(storage)?;
        tx.execute(
            "INSERT INTO reserve_transactions VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                request.idempotency_key,
                ACCOUNT_ID,
                operation.as_str(),
                amount,
                next,
                request.reference,
                now
            ],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        drop(connection);
        self.reserve()
    }

    pub fn transaction(&self, idempotency_key: &str) -> Result<ReserveTransaction, MockBankError> {
        self.connection
            .lock()
            .map_err(storage)?
            .query_row(
                "SELECT idempotency_key,operation_type,amount_minor,reference,created_at_unix_ms FROM reserve_transactions WHERE idempotency_key=?1",
                [idempotency_key],
                |row| Ok(ReserveTransaction {
                    idempotency_key: row.get(0)?,
                    operation_type: row.get(1)?,
                    amount_minor: row.get::<_, i64>(2)?.to_string(),
                    reference: row.get(3)?,
                    created_at_unix_ms: row.get::<_, i64>(4)? as u64,
                }),
            )
            .optional()
            .map_err(storage)?
            .ok_or(MockBankError::TransactionNotFound)
    }
    pub fn initialize(
        &self,
        request: InitializeReserveRequest,
    ) -> Result<BankReserve, MockBankError> {
        let target = request
            .target_balance_minor
            .parse::<i64>()
            .ok()
            .filter(|value| *value >= 0)
            .ok_or(MockBankError::InvalidTarget)?;
        if request.reference.trim().is_empty() {
            return Err(MockBankError::InvalidRequest);
        }
        let mut connection = self.connection.lock().map_err(storage)?;
        let tx = connection.transaction().map_err(storage)?;
        let previous: i64 = tx
            .query_row(
                "SELECT balance_minor FROM reserve_accounts WHERE account_id=?1",
                [ACCOUNT_ID],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let now = unix_ms() as i64;
        // The two assignments intentionally model demo cleanup followed by issuer initialization.
        // They remain one SQLite transaction, so observers never see an unexplained intermediate state.
        tx.execute("UPDATE reserve_accounts SET balance_minor=0,version=version+1,updated_at_unix_ms=?1 WHERE account_id=?2",params![now,ACCOUNT_ID]).map_err(storage)?;
        tx.execute("UPDATE reserve_accounts SET balance_minor=?1,version=version+1,updated_at_unix_ms=?2 WHERE account_id=?3",params![target,now,ACCOUNT_ID]).map_err(storage)?;
        tx.execute("INSERT INTO reserve_initializations(account_id,previous_balance_minor,target_balance_minor,reference,created_at_unix_ms) VALUES(?1,?2,?3,?4,?5)",params![ACCOUNT_ID,previous,target,request.reference,now]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        drop(connection);
        self.reserve()
    }
}

#[derive(Clone, Copy)]
pub enum Operation {
    Deposit,
    Withdrawal,
}
impl Operation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdrawal => "withdrawal",
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReserveOperationRequest {
    pub amount_minor: String,
    pub reference: String,
    pub idempotency_key: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeReserveRequest {
    pub target_balance_minor: String,
    pub reference: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReserveTransaction {
    pub idempotency_key: String,
    pub operation_type: String,
    pub amount_minor: String,
    pub reference: String,
    pub created_at_unix_ms: u64,
}
#[derive(Debug, Error)]
pub enum MockBankError {
    #[error("invalid positive amountMinor")]
    InvalidAmount,
    #[error("reference and idempotencyKey are required")]
    InvalidRequest,
    #[error("invalid non-negative targetBalanceMinor")]
    InvalidTarget,
    #[error("reserve account has insufficient funds")]
    InsufficientFunds,
    #[error("idempotency key was already used with a different operation")]
    IdempotencyConflict,
    #[error("reserve transaction was not found")]
    TransactionNotFound,
    #[error("mock bank storage failed: {0}")]
    Storage(String),
}
fn storage(error: impl std::fmt::Display) -> MockBankError {
    MockBankError::Storage(error.to_string())
}
fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone)]
struct AppState {
    store: Arc<MockBankStore>,
}
pub fn router(store: Arc<MockBankStore>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/reserve-accounts/reserve-rusd", get(get_reserve))
        .route(
            "/api/v1/reserve-accounts/reserve-rusd/deposits",
            post(deposit),
        )
        .route(
            "/api/v1/admin/reserve-accounts/reserve-rusd/initialize",
            put(initialize),
        )
        .route(
            "/api/v1/reserve-accounts/reserve-rusd/withdrawals",
            post(withdraw),
        )
        .route(
            "/api/v1/reserve-transactions/{idempotency_key}",
            get(get_transaction),
        )
        .with_state(AppState { store })
}
async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}
async fn get_reserve(State(state): State<AppState>) -> Result<Json<BankReserve>, MockBankError> {
    state.store.reserve().map(Json)
}
async fn deposit(
    State(state): State<AppState>,
    Json(request): Json<ReserveOperationRequest>,
) -> Result<Json<BankReserve>, MockBankError> {
    state.store.apply(Operation::Deposit, request).map(Json)
}
async fn withdraw(
    State(state): State<AppState>,
    Json(request): Json<ReserveOperationRequest>,
) -> Result<Json<BankReserve>, MockBankError> {
    state.store.apply(Operation::Withdrawal, request).map(Json)
}
async fn initialize(
    State(state): State<AppState>,
    Json(request): Json<InitializeReserveRequest>,
) -> Result<Json<BankReserve>, MockBankError> {
    state.store.initialize(request).map(Json)
}
async fn get_transaction(
    State(state): State<AppState>,
    axum::extract::Path(idempotency_key): axum::extract::Path<String>,
) -> Result<Json<ReserveTransaction>, MockBankError> {
    state.store.transaction(&idempotency_key).map(Json)
}
#[derive(Serialize)]
struct Health {
    status: &'static str,
}
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}
impl IntoResponse for MockBankError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::InvalidAmount | Self::InvalidRequest | Self::InvalidTarget => {
                StatusCode::BAD_REQUEST
            }
            Self::InsufficientFunds | Self::IdempotencyConflict => StatusCode::CONFLICT,
            Self::TransactionNotFound => StatusCode::NOT_FOUND,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(ErrorBody {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(key: &str, amount: &str) -> ReserveOperationRequest {
        ReserveOperationRequest {
            amount_minor: amount.to_owned(),
            reference: "test".to_owned(),
            idempotency_key: key.to_owned(),
        }
    }
    #[test]
    fn deposit_and_withdrawal_are_atomic_and_idempotent() {
        let store = MockBankStore::open(":memory:", 10_000).unwrap();
        assert_eq!(
            store
                .apply(Operation::Deposit, request("d1", "5000"))
                .unwrap()
                .balance_minor,
            "15000"
        );
        assert_eq!(
            store
                .apply(Operation::Deposit, request("d1", "5000"))
                .unwrap()
                .balance_minor,
            "15000"
        );
        let result = store
            .apply(Operation::Withdrawal, request("w1", "2500"))
            .unwrap();
        assert_eq!(result.balance_minor, "12500");
        assert_eq!(result.version, 3);
    }
    #[test]
    fn refuses_overdraft_without_changing_balance() {
        let store = MockBankStore::open(":memory:", 100).unwrap();
        assert!(matches!(
            store.apply(Operation::Withdrawal, request("w1", "101")),
            Err(MockBankError::InsufficientFunds)
        ));
        assert_eq!(store.reserve().unwrap().balance_minor, "100");
    }

    #[test]
    fn exposes_confirmed_transaction_and_rejects_idempotency_payload_changes() {
        let store = MockBankStore::open(":memory:", 100).unwrap();
        store
            .apply(Operation::Deposit, request("purchase-1", "50"))
            .unwrap();
        let transaction = store.transaction("purchase-1").unwrap();
        assert_eq!(transaction.operation_type, "deposit");
        assert_eq!(transaction.amount_minor, "50");
        assert!(matches!(
            store.apply(Operation::Deposit, request("purchase-1", "51")),
            Err(MockBankError::IdempotencyConflict)
        ));
    }
    #[test]
    fn issuer_initialization_replaces_balance_and_records_auditable_state() {
        let store = MockBankStore::open(":memory:", 500_000).unwrap();
        let result = store
            .initialize(InitializeReserveRequest {
                target_balance_minor: "110000".into(),
                reference: "issuer-startup".into(),
            })
            .unwrap();
        assert_eq!(result.balance_minor, "110000");
        let connection = store.connection.lock().unwrap();
        let values: (i64, i64) = connection
            .query_row(
                "SELECT previous_balance_minor,target_balance_minor FROM reserve_initializations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(values, (500_000, 110_000));
    }
}
