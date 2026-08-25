use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStateCode {
    Active,
    Warning,
    MintBlocked,
    DataUnavailable,
    WindDown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetState {
    pub state: AssetStateCode,
    pub reason: String,
    pub reserve_coverage_percent: Option<f64>,
    pub evidence_at_unix_ms: Option<u64>,
    pub policy_version: String,
    pub updated_at_unix_ms: u64,
}
