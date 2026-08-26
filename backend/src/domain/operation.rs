use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuerOperationKind {
    Issuance,
    Redemption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationDecisionOutcome {
    Allowed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDecision {
    pub operation_id: String,
    pub operation_kind: IssuerOperationKind,
    pub asset_state: String,
    pub outcome: OperationDecisionOutcome,
    pub reason: String,
    pub evidence_at_unix_ms: Option<u64>,
    pub policy_version: String,
    pub decided_at_unix_ms: u64,
}
