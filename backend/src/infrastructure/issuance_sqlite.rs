use crate::{
    application::{IssuanceError, IssuanceStore},
    domain::{CoverageDecisionCode, IssuanceCoverageDecision, IssuanceOrder, IssuanceStatus},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{fs, path::Path, sync::Mutex};

pub struct SqliteIssuanceStore {
    connection: Mutex<Connection>,
}

impl SqliteIssuanceStore {
    pub fn open(path: &str) -> Result<Self, IssuanceError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!("../../migrations/0002_issuance.sql"))
            .map_err(storage)?;
        connection
            .execute_batch(include_str!("../../migrations/0005_issuance_coverage.sql"))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl IssuanceStore for SqliteIssuanceStore {
    fn create(&self, order: &IssuanceOrder) -> Result<IssuanceOrder, IssuanceError> {
        let connection = self.connection.lock().map_err(storage)?;
        if let Some(existing) = query(&connection, &order.operation_id)? {
            if existing.recipient_address == order.recipient_address
                && existing.amount_usd_minor == order.amount_usd_minor
            {
                return Ok(existing);
            }
            return Err(IssuanceError::IdempotencyConflict);
        }
        connection.execute("INSERT INTO issuance_orders VALUES(?1,?2,?3,?4,?5,'awaiting_fiat',NULL,NULL,?6,?6)",params![order.operation_id,order.recipient_address,parse_i64(&order.amount_usd_minor)?,parse_i64(&order.token_amount_raw)?,order.bank_idempotency_key,order.created_at_unix_ms as i64]).map_err(storage)?;
        query(&connection, &order.operation_id)?
            .ok_or_else(|| IssuanceError::Storage("created order disappeared".to_owned()))
    }

    fn get(&self, operation_id: &str) -> Result<Option<IssuanceOrder>, IssuanceError> {
        let connection = self.connection.lock().map_err(storage)?;
        query(&connection, operation_id)
    }

    fn claim_for_mint(&self, operation_id: &str) -> Result<IssuanceOrder, IssuanceError> {
        let connection = self.connection.lock().map_err(storage)?;
        let changed=connection.execute("UPDATE issuance_orders SET status='minting',last_error=NULL,updated_at_unix_ms=?1 WHERE operation_id=?2 AND status IN ('awaiting_fiat','failed')",params![unix_ms() as i64,operation_id]).map_err(storage)?;
        if changed == 0 {
            let existing = query(&connection, operation_id)?.ok_or(IssuanceError::NotFound)?;
            return if matches!(
                existing.status,
                IssuanceStatus::Completed | IssuanceStatus::Minting
            ) {
                Ok(existing)
            } else {
                Err(IssuanceError::SettlementInProgress)
            };
        }
        query(&connection, operation_id)?.ok_or(IssuanceError::NotFound)
    }

    fn complete(
        &self,
        operation_id: &str,
        transaction_hash: Option<&str>,
    ) -> Result<IssuanceOrder, IssuanceError> {
        let connection = self.connection.lock().map_err(storage)?;
        connection.execute("UPDATE issuance_orders SET status='completed',transaction_hash=?1,last_error=NULL,updated_at_unix_ms=?2 WHERE operation_id=?3",params![transaction_hash,unix_ms() as i64,operation_id]).map_err(storage)?;
        query(&connection, operation_id)?.ok_or(IssuanceError::NotFound)
    }

    fn fail(&self, operation_id: &str, message: &str) -> Result<(), IssuanceError> {
        self.connection.lock().map_err(storage)?.execute("UPDATE issuance_orders SET status='failed',last_error=?1,updated_at_unix_ms=?2 WHERE operation_id=?3",params![message,unix_ms() as i64,operation_id]).map_err(storage)?;
        Ok(())
    }

    fn record_coverage_decision(
        &self,
        decision: &IssuanceCoverageDecision,
    ) -> Result<(), IssuanceError> {
        self.connection.lock().map_err(storage)?.execute(
            "INSERT INTO issuance_coverage_decisions(operation_id,decision,reason,current_reserve_minor,pre_operation_reserve_minor,confirmed_incoming_minor,current_supply_raw,proposed_mint_raw,current_coverage_bps,projected_coverage_bps,evidence_block_number,bank_as_of_unix_ms,policy_version,evaluated_at_unix_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![decision.operation_id, decision_code(decision.decision), decision.reason, decision.current_reserve_minor, decision.pre_operation_reserve_minor, decision.confirmed_incoming_minor, decision.current_supply_raw, decision.proposed_mint_raw, decision.current_coverage_bps, decision.projected_coverage_bps, decision.evidence_block_number.map(|v| v as i64), decision.bank_as_of_unix_ms.map(|v| v as i64), decision.policy_version, decision.evaluated_at_unix_ms as i64],
        ).map_err(storage)?;
        Ok(())
    }
}

fn decision_code(value: CoverageDecisionCode) -> &'static str {
    match value {
        CoverageDecisionCode::Accepted => "accepted",
        CoverageDecisionCode::Rejected => "rejected",
        CoverageDecisionCode::DataUnavailable => "data_unavailable",
    }
}

fn query(connection: &Connection, id: &str) -> Result<Option<IssuanceOrder>, IssuanceError> {
    connection.query_row("SELECT operation_id,recipient_address,amount_usd_minor,token_amount_raw,bank_idempotency_key,status,transaction_hash,last_error,created_at_unix_ms,updated_at_unix_ms FROM issuance_orders WHERE operation_id=?1",[id],|row|{
        let status:String=row.get(5)?;
        Ok(IssuanceOrder{operation_id:row.get(0)?,recipient_address:row.get(1)?,amount_usd_minor:row.get::<_,i64>(2)?.to_string(),token_amount_raw:row.get::<_,i64>(3)?.to_string(),bank_idempotency_key:row.get(4)?,status:parse_status(&status),transaction_hash:row.get(6)?,last_error:row.get(7)?,created_at_unix_ms:row.get::<_,i64>(8)? as u64,updated_at_unix_ms:row.get::<_,i64>(9)? as u64})
    }).optional().map_err(storage)
}
fn parse_status(value: &str) -> IssuanceStatus {
    match value {
        "awaiting_fiat" => IssuanceStatus::AwaitingFiat,
        "minting" => IssuanceStatus::Minting,
        "completed" => IssuanceStatus::Completed,
        _ => IssuanceStatus::Failed,
    }
}
fn parse_i64(value: &str) -> Result<i64, IssuanceError> {
    value.parse().map_err(|_| {
        IssuanceError::Storage("numeric value exceeds SQLite integer range".to_owned())
    })
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn storage(error: impl std::fmt::Display) -> IssuanceError {
    IssuanceError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn order() -> IssuanceOrder {
        IssuanceOrder {
            operation_id: "op-1".to_owned(),
            recipient_address: "0x0000000000000000000000000000000000000001".to_owned(),
            amount_usd_minor: "100".to_owned(),
            token_amount_raw: "1000000".to_owned(),
            bank_idempotency_key: "issuance-op-1".to_owned(),
            status: IssuanceStatus::AwaitingFiat,
            transaction_hash: None,
            last_error: None,
            created_at_unix_ms: 1,
            updated_at_unix_ms: 1,
        }
    }
    #[test]
    fn persists_idempotent_order_and_state_transitions() {
        let store = SqliteIssuanceStore::open(":memory:").unwrap();
        assert_eq!(
            store.create(&order()).unwrap(),
            store.create(&order()).unwrap()
        );
        assert_eq!(
            store.claim_for_mint("op-1").unwrap().status,
            IssuanceStatus::Minting
        );
        assert_eq!(
            store.claim_for_mint("op-1").unwrap().status,
            IssuanceStatus::Minting
        );
        assert_eq!(
            store.complete("op-1", Some("0xabc")).unwrap().status,
            IssuanceStatus::Completed
        );

        store
            .record_coverage_decision(&IssuanceCoverageDecision {
                operation_id: "op-1".into(),
                decision: CoverageDecisionCode::Accepted,
                reason: "covered".into(),
                current_reserve_minor: Some("110".into()),
                pre_operation_reserve_minor: Some("100".into()),
                confirmed_incoming_minor: "10".into(),
                current_supply_raw: Some("1000000".into()),
                proposed_mint_raw: "100000".into(),
                current_coverage_bps: Some("11000".into()),
                projected_coverage_bps: Some("10000".into()),
                evidence_block_number: Some(7),
                bank_as_of_unix_ms: Some(8),
                policy_version: "issuance-coverage-v1".into(),
                evaluated_at_unix_ms: 9,
            })
            .unwrap();
        let count: i64 = store
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM issuance_coverage_decisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
