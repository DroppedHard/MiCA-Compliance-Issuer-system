use crate::domain::{
    AssetState, AssetStateCode, IssuerOperationKind, OperationDecision, OperationDecisionOutcome,
};
use std::sync::Arc;
use thiserror::Error;

pub const OPERATION_GATE_POLICY_VERSION: &str = "issuer-operation-gate-v1";

pub trait OperationDecisionStore: Send + Sync {
    fn append(&self, decision: &OperationDecision) -> Result<(), OperationGateError>;
}

#[derive(Clone)]
pub struct OperationGate {
    store: Arc<dyn OperationDecisionStore>,
}

impl OperationGate {
    pub fn new(store: Arc<dyn OperationDecisionStore>) -> Self {
        Self { store }
    }

    pub fn decide(
        &self,
        operation_id: &str,
        operation_kind: IssuerOperationKind,
        state: &AssetState,
    ) -> Result<OperationDecision, OperationGateError> {
        let (outcome, reason) = evaluate_operation(operation_kind, state);
        let decision = OperationDecision {
            operation_id: operation_id.to_owned(),
            operation_kind,
            asset_state: state_code(state.state).to_owned(),
            outcome,
            reason,
            evidence_at_unix_ms: state.evidence_at_unix_ms,
            policy_version: OPERATION_GATE_POLICY_VERSION.to_owned(),
            decided_at_unix_ms: unix_ms(),
        };
        self.store.append(&decision)?;
        Ok(decision)
    }
}

pub fn evaluate_operation(
    operation_kind: IssuerOperationKind,
    state: &AssetState,
) -> (OperationDecisionOutcome, String) {
    match operation_kind {
        IssuerOperationKind::Issuance => match state.state {
            AssetStateCode::Active | AssetStateCode::Warning => (
                OperationDecisionOutcome::Allowed,
                format!("issuance is allowed in {}", state_code(state.state)),
            ),
            AssetStateCode::MintBlocked
            | AssetStateCode::DataUnavailable
            | AssetStateCode::WindDown => (
                OperationDecisionOutcome::Rejected,
                format!(
                    "issuance is blocked in {}: {}",
                    state_code(state.state),
                    state.reason
                ),
            ),
        },
        IssuerOperationKind::Redemption => (
            OperationDecisionOutcome::Allowed,
            format!(
                "redemption at par remains allowed in {}",
                state_code(state.state)
            ),
        ),
    }
}

fn state_code(state: AssetStateCode) -> &'static str {
    match state {
        AssetStateCode::Active => "active",
        AssetStateCode::Warning => "warning",
        AssetStateCode::MintBlocked => "mint_blocked",
        AssetStateCode::DataUnavailable => "data_unavailable",
        AssetStateCode::WindDown => "wind_down",
    }
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Error)]
pub enum OperationGateError {
    #[error("operation decision persistence failed: {0}")]
    Storage(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(state: AssetStateCode) -> AssetState {
        AssetState {
            state,
            reason: "test evidence".into(),
            reserve_coverage_percent: Some(99.0),
            evidence_at_unix_ms: Some(1),
            policy_version: "reserve-coverage-v1".into(),
            updated_at_unix_ms: 1,
        }
    }

    #[test]
    fn issuance_policy_allows_only_active_and_warning() {
        for code in [AssetStateCode::Active, AssetStateCode::Warning] {
            assert_eq!(
                evaluate_operation(IssuerOperationKind::Issuance, &state(code)).0,
                OperationDecisionOutcome::Allowed
            );
        }
        for code in [
            AssetStateCode::MintBlocked,
            AssetStateCode::DataUnavailable,
            AssetStateCode::WindDown,
        ] {
            assert_eq!(
                evaluate_operation(IssuerOperationKind::Issuance, &state(code)).0,
                OperationDecisionOutcome::Rejected
            );
        }
    }

    #[test]
    fn redemption_policy_allows_every_asset_state() {
        for code in [
            AssetStateCode::Active,
            AssetStateCode::Warning,
            AssetStateCode::MintBlocked,
            AssetStateCode::DataUnavailable,
            AssetStateCode::WindDown,
        ] {
            assert_eq!(
                evaluate_operation(IssuerOperationKind::Redemption, &state(code)).0,
                OperationDecisionOutcome::Allowed
            );
        }
    }
}
