use crate::domain::{AssetState, AssetStateCode, ReserveCoverage};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::sync::broadcast;

pub const ASSET_STATE_POLICY_VERSION: &str = "reserve-coverage-v1";

#[derive(Debug, Error)]
pub enum AssetStateError {
    #[error("asset state storage failed: {0}")]
    Storage(String),
}

pub trait AssetStateStore: Send + Sync {
    fn load(&self) -> Result<Option<AssetState>, AssetStateError>;
    fn save(&self, state: &AssetState) -> Result<(), AssetStateError>;
}

#[derive(Clone)]
pub struct AssetStateService {
    store: Arc<dyn AssetStateStore>,
    sender: broadcast::Sender<AssetState>,
}

impl AssetStateService {
    pub fn new(store: Arc<dyn AssetStateStore>, capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { store, sender }
    }

    pub fn current(&self) -> Result<AssetState, AssetStateError> {
        self.store
            .load()?
            .ok_or_else(|| AssetStateError::Storage("asset state was not initialized".into()))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AssetState> {
        self.sender.subscribe()
    }

    pub fn evaluate_coverage(
        &self,
        coverage: &ReserveCoverage,
    ) -> Result<AssetState, AssetStateError> {
        if self.current()?.state == AssetStateCode::WindDown {
            return self.current();
        }
        let state = evaluate(coverage.ratio_percent, Some(coverage.observed_at_unix_ms));
        self.persist_and_publish(state)
    }

    pub fn mark_data_unavailable(
        &self,
        reason: impl Into<String>,
    ) -> Result<AssetState, AssetStateError> {
        if self.current()?.state == AssetStateCode::WindDown {
            return self.current();
        }
        self.persist_and_publish(AssetState {
            state: AssetStateCode::DataUnavailable,
            reason: reason.into(),
            reserve_coverage_percent: None,
            evidence_at_unix_ms: None,
            policy_version: ASSET_STATE_POLICY_VERSION.into(),
            updated_at_unix_ms: unix_ms(),
        })
    }

    pub fn enter_wind_down(
        &self,
        reason: impl Into<String>,
    ) -> Result<AssetState, AssetStateError> {
        let current = self.current()?;
        self.persist_and_publish(AssetState {
            state: AssetStateCode::WindDown,
            reason: reason.into(),
            reserve_coverage_percent: current.reserve_coverage_percent,
            evidence_at_unix_ms: current.evidence_at_unix_ms,
            policy_version: ASSET_STATE_POLICY_VERSION.into(),
            updated_at_unix_ms: unix_ms(),
        })
    }

    pub fn block_mint(
        &self,
        reason: impl Into<String>,
        projected_coverage_percent: Option<f64>,
        evidence_at_unix_ms: Option<u64>,
    ) -> Result<AssetState, AssetStateError> {
        if self.current()?.state == AssetStateCode::WindDown {
            return self.current();
        }
        self.persist_and_publish(AssetState {
            state: AssetStateCode::MintBlocked,
            reason: reason.into(),
            reserve_coverage_percent: projected_coverage_percent,
            evidence_at_unix_ms,
            policy_version: ASSET_STATE_POLICY_VERSION.into(),
            updated_at_unix_ms: unix_ms(),
        })
    }

    fn persist_and_publish(&self, state: AssetState) -> Result<AssetState, AssetStateError> {
        self.store.save(&state)?;
        let _ = self.sender.send(state.clone());
        Ok(state)
    }
}

pub fn evaluate(ratio_percent: Option<f64>, evidence_at_unix_ms: Option<u64>) -> AssetState {
    let (state, reason) = match ratio_percent {
        None => (
            AssetStateCode::DataUnavailable,
            "Reserve coverage cannot be calculated without token supply",
        ),
        Some(value) if value >= 105.0 => (
            AssetStateCode::Active,
            "Reserve coverage is at or above the 105% demo safety margin",
        ),
        Some(value) if value >= 100.0 => (
            AssetStateCode::Warning,
            "Reserve coverage is below the 105% demo safety margin",
        ),
        Some(_) => (
            AssetStateCode::MintBlocked,
            "Reserve coverage is below 100%; issuance must be blocked",
        ),
    };
    AssetState {
        state,
        reason: reason.into(),
        reserve_coverage_percent: ratio_percent,
        evidence_at_unix_ms,
        policy_version: ASSET_STATE_POLICY_VERSION.into(),
        updated_at_unix_ms: unix_ms(),
    }
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
    use std::sync::Mutex;

    struct MemoryStore(Mutex<Option<AssetState>>);
    impl AssetStateStore for MemoryStore {
        fn load(&self) -> Result<Option<AssetState>, AssetStateError> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save(&self, state: &AssetState) -> Result<(), AssetStateError> {
            *self.0.lock().unwrap() = Some(state.clone());
            Ok(())
        }
    }
    #[test]
    fn evaluates_all_coverage_boundaries_deterministically() {
        assert_eq!(evaluate(Some(105.0), Some(1)).state, AssetStateCode::Active);
        assert_eq!(
            evaluate(Some(104.999), Some(1)).state,
            AssetStateCode::Warning
        );
        assert_eq!(
            evaluate(Some(100.0), Some(1)).state,
            AssetStateCode::Warning
        );
        assert_eq!(
            evaluate(Some(99.999), Some(1)).state,
            AssetStateCode::MintBlocked
        );
        assert_eq!(evaluate(None, None).state, AssetStateCode::DataUnavailable);
    }

    #[test]
    fn wind_down_is_manual_persisted_and_not_reversed_by_polling() {
        let initial = evaluate(Some(110.0), Some(1));
        let service = AssetStateService::new(Arc::new(MemoryStore(Mutex::new(Some(initial)))), 4);
        service.enter_wind_down("authority decision").unwrap();

        let coverage = ReserveCoverage {
            observed_at_unix_ms: 2,
            bank_as_of_unix_ms: 2,
            reserve_account_id: "reserve-rusd".into(),
            currency: "USD".into(),
            reserve_balance_minor: "11000".into(),
            reserve_balance_usd: "110.00".into(),
            token_supply_raw: "100000000".into(),
            liability_usd: "100".into(),
            surplus_usd: "10".into(),
            ratio_percent: Some(110.0),
            status: crate::domain::CoverageStatus::Covered,
        };
        assert_eq!(
            service.evaluate_coverage(&coverage).unwrap().state,
            AssetStateCode::WindDown
        );
        assert_eq!(service.current().unwrap().reason, "authority decision");
    }
}
