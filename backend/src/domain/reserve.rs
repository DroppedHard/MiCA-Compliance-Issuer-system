use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankReserve {
    pub account_id: String,
    pub currency: String,
    pub balance_minor: String,
    pub version: u64,
    pub as_of_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoverageStatus {
    Covered,
    Undercollateralized,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReserveCoverage {
    pub observed_at_unix_ms: u64,
    pub bank_as_of_unix_ms: u64,
    pub reserve_account_id: String,
    pub currency: String,
    pub reserve_balance_minor: String,
    pub reserve_balance_usd: String,
    pub token_supply_raw: String,
    pub liability_usd: String,
    pub surplus_usd: String,
    pub ratio_percent: Option<f64>,
    pub status: CoverageStatus,
}
