use crate::{
    application::{AssetStateService, CacheError, SnapshotCache},
    domain::{AssetState, BankReserve, CoverageStatus, ReserveCoverage},
};
use async_trait::async_trait;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::{
    sync::{RwLock, broadcast},
    time::{MissedTickBehavior, interval},
};
use tracing::{debug, error, info};

#[async_trait]
pub trait ReserveReader: Send + Sync {
    async fn read_reserve(&self) -> Result<BankReserve, ReserveError>;
}
#[async_trait]
pub trait ReserveInitializer: Send + Sync {
    async fn initialize_reserve(&self, target_balance_minor: u64) -> Result<(), ReserveError>;
}
#[async_trait]
pub trait ReserveStateController: Send + Sync {
    async fn synchronize_reserve_state(&self, state: &AssetState) -> Result<(), ReserveError>;
}

/// Returns the USD-cent reserve target for the explicit 110% demo policy.
/// Rounding is always upward so fractional cents never reduce coverage.
pub fn initial_reserve_target_minor(
    total_supply_raw: &str,
    decimals: u8,
) -> Result<u64, ReserveError> {
    if decimals < 2 {
        return Err(ReserveError::InvalidValue(
            "token must expose at least two decimals".into(),
        ));
    }
    let raw = total_supply_raw
        .parse::<u128>()
        .map_err(|_| ReserveError::InvalidValue(total_supply_raw.into()))?;
    let units_per_cent = 10_u128
        .checked_pow((decimals - 2).into())
        .ok_or_else(|| ReserveError::InvalidValue("token decimals overflow".into()))?;
    let numerator = raw
        .checked_mul(110)
        .ok_or_else(|| ReserveError::InvalidValue("reserve target overflow".into()))?;
    let denominator = 100_u128
        .checked_mul(units_per_cent)
        .ok_or_else(|| ReserveError::InvalidValue("reserve denominator overflow".into()))?;
    let target = numerator.div_ceil(denominator);
    u64::try_from(target)
        .map_err(|_| ReserveError::InvalidValue("reserve target exceeds demo range".into()))
}

#[derive(Debug, Error)]
pub enum ReserveError {
    #[error("mock bank request failed: {0}")]
    Bank(String),
    #[error("token cache failed: {0}")]
    Cache(String),
    #[error("token cache has no observation yet")]
    TokenUnavailable,
    #[error("unsupported reserve currency: {0}")]
    Currency(String),
    #[error("invalid monetary value: {0}")]
    InvalidValue(String),
    #[error("on-chain reserve state update failed: {0}")]
    Blockchain(String),
}
impl From<CacheError> for ReserveError {
    fn from(value: CacheError) -> Self {
        Self::Cache(value.to_string())
    }
}

#[derive(Clone)]
pub struct ReserveMonitor {
    state: Arc<RwLock<ReserveState>>,
    sender: broadcast::Sender<ReserveCoverage>,
}
#[derive(Default)]
struct ReserveState {
    latest: Option<ReserveCoverage>,
    last_error: Option<String>,
}
impl ReserveMonitor {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            state: Arc::new(RwLock::new(ReserveState::default())),
            sender,
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<ReserveCoverage> {
        self.sender.subscribe()
    }
    pub async fn latest(&self) -> Result<ReserveCoverage, ReserveError> {
        let state = self.state.read().await;
        if let Some(error) = &state.last_error {
            return Err(ReserveError::Bank(error.clone()));
        }
        state.latest.clone().ok_or(ReserveError::TokenUnavailable)
    }
    async fn success(&self, value: ReserveCoverage) {
        let mut state = self.state.write().await;
        state.latest = Some(value.clone());
        state.last_error = None;
        let _ = self.sender.send(value);
    }
    async fn failure(&self, error: String) {
        self.state.write().await.last_error = Some(error);
    }
    async fn record_poll_failure(&self, error: &ReserveError) {
        if !matches!(error, ReserveError::TokenUnavailable) {
            self.failure(error.to_string()).await;
        }
    }
}

pub struct ReservePollingService {
    reader: Arc<dyn ReserveReader>,
    token_cache: Arc<dyn SnapshotCache>,
    monitor: ReserveMonitor,
    asset_state: Arc<AssetStateService>,
    state_controller: Arc<dyn ReserveStateController>,
    poll_interval: Duration,
}
impl ReservePollingService {
    pub fn new(
        reader: Arc<dyn ReserveReader>,
        token_cache: Arc<dyn SnapshotCache>,
        monitor: ReserveMonitor,
        asset_state: Arc<AssetStateService>,
        state_controller: Arc<dyn ReserveStateController>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            reader,
            token_cache,
            monitor,
            asset_state,
            state_controller,
            poll_interval,
        }
    }
    pub async fn poll_once(&self) -> Result<ReserveCoverage, ReserveError> {
        let token = self
            .token_cache
            .latest()
            .await?
            .ok_or(ReserveError::TokenUnavailable)?;
        let reserve = self.reader.read_reserve().await?;
        let coverage = calculate_coverage(
            &token.snapshot.total_supply_raw,
            token.snapshot.decimals,
            reserve,
        )?;
        self.monitor.success(coverage.clone()).await;
        let state = self
            .asset_state
            .evaluate_coverage(&coverage)
            .map_err(|error| ReserveError::Cache(error.to_string()))?;
        self.state_controller
            .synchronize_reserve_state(&state)
            .await?;
        Ok(coverage)
    }
    pub async fn run(self) {
        let mut ticker = interval(self.poll_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match self.poll_once().await {
                Ok(value) => info!(status=?value.status, "reserve coverage updated"),
                Err(ReserveError::TokenUnavailable) => {
                    if let Err(error) = self
                        .block_for_unavailable_data(
                            "Token supply is unavailable, so reserve coverage cannot be evaluated",
                        )
                        .await
                    {
                        error!(error=%error, "asset state update failed");
                    }
                    debug!("reserve polling is waiting for the first token observation");
                }
                Err(error) => {
                    let message = error.to_string();
                    self.monitor.record_poll_failure(&error).await;
                    if let Err(state_error) = self.block_for_unavailable_data(message.clone()).await
                    {
                        error!(error=%state_error, "asset state update failed");
                    }
                    error!(error=%message,"reserve polling failed");
                }
            }
        }
    }

    async fn block_for_unavailable_data(
        &self,
        reason: impl Into<String>,
    ) -> Result<(), ReserveError> {
        let state = self
            .asset_state
            .mark_data_unavailable(reason)
            .map_err(|error| ReserveError::Cache(error.to_string()))?;
        self.state_controller
            .synchronize_reserve_state(&state)
            .await
    }
}

pub fn calculate_coverage(
    supply_raw: &str,
    decimals: u8,
    reserve: BankReserve,
) -> Result<ReserveCoverage, ReserveError> {
    if reserve.currency != "USD" {
        return Err(ReserveError::Currency(reserve.currency));
    }
    let supply = supply_raw
        .parse::<u128>()
        .map_err(|_| ReserveError::InvalidValue(supply_raw.to_owned()))?;
    let balance_minor = reserve
        .balance_minor
        .parse::<u128>()
        .map_err(|_| ReserveError::InvalidValue(reserve.balance_minor.clone()))?;
    let scale = 10_u128
        .checked_pow(decimals.into())
        .ok_or_else(|| ReserveError::InvalidValue("token decimals overflow".to_owned()))?;
    let reserve_raw = balance_minor
        .checked_mul(scale)
        .and_then(|v| v.checked_div(100))
        .ok_or_else(|| ReserveError::InvalidValue("reserve conversion overflow".to_owned()))?;
    let difference = reserve_raw as i128 - supply as i128;
    Ok(ReserveCoverage {
        observed_at_unix_ms: unix_ms(),
        bank_as_of_unix_ms: reserve.as_of_unix_ms,
        reserve_account_id: reserve.account_id,
        currency: reserve.currency,
        reserve_balance_minor: balance_minor.to_string(),
        reserve_balance_usd: format_minor(balance_minor),
        token_supply_raw: supply_raw.to_owned(),
        liability_usd: format_raw(supply, scale),
        surplus_usd: format_signed_raw(difference, scale),
        ratio_percent: if supply == 0 {
            None
        } else {
            Some(reserve_raw as f64 / supply as f64 * 100.0)
        },
        status: if reserve_raw >= supply {
            CoverageStatus::Covered
        } else {
            CoverageStatus::Undercollateralized
        },
    })
}
fn format_minor(value: u128) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}
fn format_raw(value: u128, scale: u128) -> String {
    format_decimal(value, scale)
}
fn format_signed_raw(value: i128, scale: u128) -> String {
    if value < 0 {
        format!("-{}", format_decimal(value.unsigned_abs(), scale))
    } else {
        format_decimal(value as u128, scale)
    }
}
fn format_decimal(value: u128, scale: u128) -> String {
    let digits = scale.ilog10() as usize;
    format!("{}.{:0digits$}", value / scale, value % scale)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calculates_initial_reserve_at_110_percent_and_rounds_up() {
        assert_eq!(
            initial_reserve_target_minor("10250000000", 6).unwrap(),
            1_127_500
        );
        assert_eq!(initial_reserve_target_minor("1", 6).unwrap(), 1)
    }
    fn bank(balance_minor: &str) -> BankReserve {
        BankReserve {
            account_id: "reserve-rusd".to_owned(),
            currency: "USD".to_owned(),
            balance_minor: balance_minor.to_owned(),
            version: 1,
            as_of_unix_ms: 1,
        }
    }
    #[test]
    fn calculates_coverage_with_integer_money_units() {
        let covered = calculate_coverage("4300000000", 6, bank("450000")).unwrap();
        assert_eq!(covered.reserve_balance_usd, "4500.00");
        assert_eq!(covered.liability_usd, "4300");
        assert_eq!(covered.surplus_usd, "200");
        assert_eq!(covered.status, CoverageStatus::Covered);
        assert!((covered.ratio_percent.unwrap() - 104.65116279).abs() < 0.000001);
        let missing = calculate_coverage("4300000000", 6, bank("390000")).unwrap();
        assert_eq!(missing.surplus_usd, "-400");
        assert_eq!(missing.status, CoverageStatus::Undercollateralized);
    }
    #[test]
    fn rejects_wrong_currency_and_handles_zero_supply() {
        let mut eur = bank("100");
        eur.currency = "EUR".to_owned();
        assert!(matches!(
            calculate_coverage("0", 6, eur),
            Err(ReserveError::Currency(_))
        ));
        assert_eq!(
            calculate_coverage("0", 6, bank("100"))
                .unwrap()
                .ratio_percent,
            None
        );
    }

    #[tokio::test]
    async fn startup_without_token_does_not_invalidate_last_valid_coverage() {
        let monitor = ReserveMonitor::new(4);
        let coverage = calculate_coverage("4000000000", 6, bank("500000")).unwrap();
        monitor.success(coverage.clone()).await;

        monitor
            .record_poll_failure(&ReserveError::TokenUnavailable)
            .await;

        assert_eq!(monitor.latest().await.unwrap(), coverage);
    }

    #[tokio::test]
    async fn bank_failure_marks_reserve_data_as_unavailable() {
        let monitor = ReserveMonitor::new(4);
        monitor
            .success(calculate_coverage("4000000000", 6, bank("500000")).unwrap())
            .await;

        monitor
            .record_poll_failure(&ReserveError::Bank("connection refused".to_owned()))
            .await;

        assert!(matches!(monitor.latest().await, Err(ReserveError::Bank(_))));
    }
}
