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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageDecisionCode {
    Accepted,
    Rejected,
    DataUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuanceCoverageDecision {
    pub operation_id: String,
    pub decision: CoverageDecisionCode,
    pub reason: String,
    pub current_reserve_minor: Option<String>,
    pub pre_operation_reserve_minor: Option<String>,
    pub confirmed_incoming_minor: String,
    pub current_supply_raw: Option<String>,
    pub proposed_mint_raw: String,
    pub current_coverage_bps: Option<String>,
    pub projected_coverage_bps: Option<String>,
    pub evidence_block_number: Option<u64>,
    pub bank_as_of_unix_ms: Option<u64>,
    pub policy_version: String,
    pub evaluated_at_unix_ms: u64,
}
