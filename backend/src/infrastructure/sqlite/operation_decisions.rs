use crate::{
    application::{OperationDecisionStore, OperationGateError},
    domain::{IssuerOperationKind, OperationDecision, OperationDecisionOutcome},
};
use rusqlite::{Connection, params};
use std::{fs, path::Path, sync::Mutex};

pub struct SqliteOperationDecisionStore {
    connection: Mutex<Connection>,
}

impl SqliteOperationDecisionStore {
    pub fn open(path: &str) -> Result<Self, OperationGateError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!(
                "../../../migrations/0006_operation_decisions.sql"
            ))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl OperationDecisionStore for SqliteOperationDecisionStore {
    fn append(&self, decision: &OperationDecision) -> Result<(), OperationGateError> {
        self.connection
            .lock()
            .map_err(storage)?
            .execute(
                "INSERT INTO issuer_operation_decisions(operation_id,operation_kind,asset_state,outcome,reason,evidence_at_unix_ms,policy_version,decided_at_unix_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    decision.operation_id,
                    operation_kind(decision.operation_kind),
                    decision.asset_state,
                    outcome(decision.outcome),
                    decision.reason,
                    decision.evidence_at_unix_ms.map(|value| value as i64),
                    decision.policy_version,
                    decision.decided_at_unix_ms as i64,
                ],
            )
            .map_err(storage)?;
        Ok(())
    }
}

fn operation_kind(value: IssuerOperationKind) -> &'static str {
    match value {
        IssuerOperationKind::Issuance => "issuance",
        IssuerOperationKind::Redemption => "redemption",
    }
}

fn outcome(value: OperationDecisionOutcome) -> &'static str {
    match value {
        OperationDecisionOutcome::Allowed => "allowed",
        OperationDecisionOutcome::Rejected => "rejected",
    }
}

fn storage(error: impl std::fmt::Display) -> OperationGateError {
    OperationGateError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_decisions_without_overwriting_history() {
        let store = SqliteOperationDecisionStore::open(":memory:").unwrap();
        let decision = OperationDecision {
            operation_id: "op-1".into(),
            operation_kind: IssuerOperationKind::Issuance,
            asset_state: "mint_blocked".into(),
            outcome: OperationDecisionOutcome::Rejected,
            reason: "reserve shortfall".into(),
            evidence_at_unix_ms: Some(1),
            policy_version: "issuer-operation-gate-v1".into(),
            decided_at_unix_ms: 2,
        };
        store.append(&decision).unwrap();
        store.append(&decision).unwrap();
        let count: i64 = store
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM issuer_operation_decisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }
}
