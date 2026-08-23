use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuanceOrder {
    pub operation_id: String,
    pub recipient_address: String,
    pub amount_usd_minor: String,
    pub token_amount_raw: String,
    pub bank_idempotency_key: String,
    pub status: IssuanceStatus,
    pub transaction_hash: Option<String>,
    pub last_error: Option<String>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuanceStatus {
    AwaitingFiat,
    Minting,
    Completed,
    Failed,
}
