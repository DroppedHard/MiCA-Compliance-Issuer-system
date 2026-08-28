use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationAggregate {
    pub classification: String,
    pub operation_count: u64,
    pub value_raw: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaspDailyAggregate {
    pub date_utc: String,
    pub asset_symbol: String,
    pub currency_area: String,
    pub total_operation_count: u64,
    pub total_value_raw: String,
    pub total_value_usd_minor: String,
    pub means_of_exchange_count: u64,
    pub means_of_exchange_value_raw: String,
    pub means_of_exchange_value_usd_minor: String,
    pub means_of_exchange_value_eur_minor: String,
    pub excluded_operation_count: u64,
    pub known_onchain_overlap_count: u64,
    pub known_onchain_overlap_value_raw: String,
    pub classifications: Vec<ClassificationAggregate>,
    pub methodology_version: String,
    pub conversion_methodology: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaspDailyReport {
    pub from_date_utc: String,
    pub to_date_utc: String,
    pub days: Vec<CaspDailyAggregate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuarterlyTransactionAssessment {
    pub year: i32,
    pub quarter: u8,
    pub from_date_utc: String,
    pub to_date_utc: String,
    pub calendar_day_count: u64,
    pub imported_activity_day_count: u64,
    pub complete_source_range: bool,
    pub means_of_exchange_operation_count: u64,
    pub means_of_exchange_value_eur_minor: String,
    pub average_daily_operation_count: f64,
    pub average_daily_value_eur: f64,
    pub transaction_count_threshold: u64,
    pub transaction_value_threshold_eur: String,
    pub threshold_breached: bool,
    pub threshold_enforceable: bool,
    pub methodology_version: String,
}
