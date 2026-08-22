use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EsgMethodology {
    pub version: &'static str,
    pub annual_transactions_assumption: u64,
    pub lower_energy_wh_per_transaction: f64,
    pub best_guess_energy_wh_per_transaction: f64,
    pub upper_energy_wh_per_transaction: f64,
    pub emissions_g_co2e_per_transaction: f64,
    pub renewable_percent: f64,
    pub nuclear_percent: f64,
    pub fossil_percent: f64,
    pub source_name: &'static str,
    pub source_url: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EsgEstimate {
    pub date_utc: String,
    pub status: &'static str,
    pub transaction_count: u64,
    pub data_origin: &'static str,
    pub energy_lower_wh: f64,
    pub energy_best_guess_wh: f64,
    pub energy_upper_wh: f64,
    pub emissions_g_co2e: f64,
    pub renewable_energy_wh: f64,
    pub nuclear_energy_wh: f64,
    pub fossil_energy_wh: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EsgObservation {
    pub observed_at_unix_ms: u64,
    pub last_processed_block: u64,
    pub chain_id: u64,
    pub contract_address: String,
    pub current_day: EsgEstimate,
    pub methodology: EsgMethodology,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EsgHistory {
    pub days: Vec<EsgEstimate>,
    pub methodology: EsgMethodology,
}
