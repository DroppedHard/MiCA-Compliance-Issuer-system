use serde::Serialize;

/// Read-only representation of the current token state exposed by the API.
/// The raw supply is a string because Ethereum integers can exceed JavaScript's safe range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSnapshot {
    pub chain_id: u64,
    pub block_number: u64,
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply_raw: String,
}

/// A snapshot together with the wall-clock time at which the backend observed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenObservation {
    pub observed_at_unix_ms: u64,
    pub snapshot: TokenSnapshot,
}
