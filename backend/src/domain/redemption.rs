use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedemptionStatus {
    Created,
    Burned,
    Completed,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionOrder {
    pub operation_id: String,
    pub holder_address: String,
    pub token_amount_raw: String,
    pub payout_usd_minor: String,
    pub status: RedemptionStatus,
    pub burn_transaction_hash: Option<String>,
    pub payout_reference: String,
    pub last_error: Option<String>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}
