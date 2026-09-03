use crate::domain::{
    AssetState, AssetStateCode, IssuerOperationKind, OperationDecision, OperationDecisionOutcome,
};
use std::sync::Arc;
use thiserror::Error;

pub const OPERATION_GATE_POLICY_VERSION: &str = "issuer-operation-gate-v1";

pub trait OperationDecisionStore: Send + Sync {
    fn append(&self, decision: &OperationDecision) -> Result<(), OperationGateError>;
}
pub trait IssuanceRestriction: Send + Sync {
    fn block_reason(&self) -> Result<Option<String>, OperationGateError>;
}

#[derive(Clone)]
pub struct OperationGate {
    store: Arc<dyn OperationDecisionStore>,
    issuance_restriction: Option<Arc<dyn IssuanceRestriction>>,
}

impl OperationGate {
    pub fn new(store: Arc<dyn OperationDecisionStore>) -> Self {
        Self {
            store,
            issuance_restriction: None,
        }
    }
    pub fn with_issuance_restriction(
        store: Arc<dyn OperationDecisionStore>,
        issuance_restriction: Arc<dyn IssuanceRestriction>,
    ) -> Self {
        Self {
            store,
            issuance_restriction: Some(issuance_restriction),
        }
    }

    pub fn decide(
        &self,
        operation_id: &str,
        operation_kind: IssuerOperationKind,
        state: &AssetState,
    ) -> Result<OperationDecision, OperationGateError> {
        let (mut outcome, mut reason) = evaluate_operation(operation_kind, state);
        if operation_kind == IssuerOperationKind::Issuance
            && outcome == OperationDecisionOutcome::Allowed
            && let Some(block_reason) = self
                .issuance_restriction
                .as_ref()
                .map(|restriction| restriction.block_reason())
                .transpose()?
                .flatten()
        {
            outcome = OperationDecisionOutcome::Rejected;
            reason = block_reason;
        }
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
            // Before the first issuance there is no token liability, so the
            // coverage ratio is mathematically undefined. Issuance still pairs
            // the confirmed USD deposit with the same amount of newly minted
            // rUSD and therefore does not reduce reserve coverage.
            AssetStateCode::DataUnavailable
                if state.reserve_coverage_percent.is_none()
                    && state.reason
                        == "Reserve coverage cannot be calculated without token supply" =>
            {
                (
                    OperationDecisionOutcome::Allowed,
                    "initial issuance is allowed after the matching fiat deposit".to_owned(),
                )
            }
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
    struct Decisions;
    impl OperationDecisionStore for Decisions {
        fn append(&self, _: &OperationDecision) -> Result<(), OperationGateError> {
            Ok(())
        }
    }
    struct ActivityBlock;
    impl IssuanceRestriction for ActivityBlock {
        fn block_reason(&self) -> Result<Option<String>, OperationGateError> {
            Ok(Some("Article 23 threshold evidence".into()))
        }
    }

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
    fn issuance_policy_allows_covered_states_and_zero_supply_bootstrap() {
        for code in [AssetStateCode::Active, AssetStateCode::Warning] {
            assert_eq!(
                evaluate_operation(IssuerOperationKind::Issuance, &state(code)).0,
                OperationDecisionOutcome::Allowed
            );
        }
        let zero_supply = AssetState {
            state: AssetStateCode::DataUnavailable,
            reason: "Reserve coverage cannot be calculated without token supply".into(),
            reserve_coverage_percent: None,
            evidence_at_unix_ms: None,
            policy_version: "reserve-coverage-v1".into(),
            updated_at_unix_ms: 1,
        };
        assert_eq!(
            evaluate_operation(IssuerOperationKind::Issuance, &zero_supply).0,
            OperationDecisionOutcome::Allowed
        );
        for code in [AssetStateCode::MintBlocked, AssetStateCode::WindDown] {
            assert_eq!(
                evaluate_operation(IssuerOperationKind::Issuance, &state(code)).0,
                OperationDecisionOutcome::Rejected
            );
        }
        let unavailable = state(AssetStateCode::DataUnavailable);
        assert_eq!(
            evaluate_operation(IssuerOperationKind::Issuance, &unavailable).0,
            OperationDecisionOutcome::Rejected
        );
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
    #[test]
    fn persisted_activity_restriction_blocks_issuance_even_when_reserves_are_active() {
        let gate =
            OperationGate::with_issuance_restriction(Arc::new(Decisions), Arc::new(ActivityBlock));
        let decision = gate
            .decide(
                "issuance",
                IssuerOperationKind::Issuance,
                &state(AssetStateCode::Active),
            )
            .unwrap();
        assert_eq!(decision.outcome, OperationDecisionOutcome::Rejected);
        assert_eq!(decision.reason, "Article 23 threshold evidence");
    }
}
