use crate::{
    application::{RedemptionError, RedemptionStore},
    domain::{RedemptionOrder, RedemptionStatus},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{fs, path::Path, sync::Mutex};
pub struct SqliteRedemptionStore {
    connection: Mutex<Connection>,
}
impl SqliteRedemptionStore {
    pub fn open(path: &str) -> Result<Self, RedemptionError> {
        if let Some(p) = Path::new(path).parent() {
            fs::create_dir_all(p).map_err(storage)?
        }
        let c = Connection::open(path).map_err(storage)?;
        c.execute_batch(include_str!("../../../migrations/0003_redemption.sql"))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(c),
        })
    }
}
impl RedemptionStore for SqliteRedemptionStore {
    fn create(&self, o: &RedemptionOrder) -> Result<RedemptionOrder, RedemptionError> {
        let c = self.connection.lock().map_err(storage)?;
        if let Some(e) = query(&c, &o.operation_id)? {
            if e.holder_address == o.holder_address && e.token_amount_raw == o.token_amount_raw {
                return Ok(e);
            }
            return Err(RedemptionError::IdempotencyConflict);
        }
        c.execute(
            "INSERT INTO redemption_orders VALUES(?1,?2,?3,?4,'created',NULL,?5,NULL,?6,?6)",
            params![
                o.operation_id,
                o.holder_address,
                num(&o.token_amount_raw)?,
                num(&o.payout_usd_minor)?,
                o.payout_reference,
                o.created_at_unix_ms as i64
            ],
        )
        .map_err(storage)?;
        query(&c, &o.operation_id)?.ok_or(RedemptionError::NotFound)
    }
    fn get(&self, id: &str) -> Result<Option<RedemptionOrder>, RedemptionError> {
        let c = self.connection.lock().map_err(storage)?;
        query(&c, id)
    }
    fn mark_burned(&self, id: &str, h: Option<&str>) -> Result<RedemptionOrder, RedemptionError> {
        update(&self.connection, id, "burned", h, None)
    }
    fn complete(&self, id: &str) -> Result<RedemptionOrder, RedemptionError> {
        update(&self.connection, id, "completed", None, None)
    }
    fn fail(&self, id: &str, m: &str) -> Result<(), RedemptionError> {
        // Po potwierdzonym burnie błąd wypłaty nie może cofnąć operacji do
        // ogólnego `failed`: retry próbowałby ponownie spalić te same tokeny,
        // a kontrakt poprawnie odrzuciłby zduplikowany operationId. Zachowanie
        // `burned` pozwala ponowić wyłącznie wypłatę fiat.
        self.connection.lock().map_err(storage)?.execute("UPDATE redemption_orders SET status=CASE WHEN burn_transaction_hash IS NULL THEN 'failed' ELSE 'burned' END,last_error=?1,updated_at_unix_ms=?2 WHERE operation_id=?3",params![m,now() as i64,id]).map_err(storage)?;
        Ok(())
    }
}
fn update(
    lock: &Mutex<Connection>,
    id: &str,
    status: &str,
    hash: Option<&str>,
    error: Option<&str>,
) -> Result<RedemptionOrder, RedemptionError> {
    let c = lock.lock().map_err(storage)?;
    c.execute("UPDATE redemption_orders SET status=?1,burn_transaction_hash=COALESCE(?2,burn_transaction_hash),last_error=?3,updated_at_unix_ms=?4 WHERE operation_id=?5",params![status,hash,error,now() as i64,id]).map_err(storage)?;
    query(&c, id)?.ok_or(RedemptionError::NotFound)
}
fn query(c: &Connection, id: &str) -> Result<Option<RedemptionOrder>, RedemptionError> {
    c.query_row("SELECT operation_id,holder_address,token_amount_raw,payout_usd_minor,status,burn_transaction_hash,payout_reference,last_error,created_at_unix_ms,updated_at_unix_ms FROM redemption_orders WHERE operation_id=?1",[id],|r|{let s:String=r.get(4)?;Ok(RedemptionOrder{operation_id:r.get(0)?,holder_address:r.get(1)?,token_amount_raw:r.get::<_,i64>(2)?.to_string(),payout_usd_minor:r.get::<_,i64>(3)?.to_string(),status:match s.as_str(){"created"=>RedemptionStatus::Created,"burned"=>RedemptionStatus::Burned,"completed"=>RedemptionStatus::Completed,_=>RedemptionStatus::Failed},burn_transaction_hash:r.get(5)?,payout_reference:r.get(6)?,last_error:r.get(7)?,created_at_unix_ms:r.get::<_,i64>(8)? as u64,updated_at_unix_ms:r.get::<_,i64>(9)? as u64})}).optional().map_err(storage)
}
fn num(v: &str) -> Result<i64, RedemptionError> {
    v.parse()
        .map_err(|_| RedemptionError::Storage("numeric value exceeds SQLite range".into()))
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn storage(e: impl std::fmt::Display) -> RedemptionError {
    RedemptionError::Storage(e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn order() -> RedemptionOrder {
        RedemptionOrder {
            operation_id: "r1".into(),
            holder_address: "0x1".into(),
            token_amount_raw: "1000000".into(),
            payout_usd_minor: "100".into(),
            status: RedemptionStatus::Created,
            burn_transaction_hash: None,
            payout_reference: "redemption-r1".into(),
            last_error: None,
            created_at_unix_ms: 1,
            updated_at_unix_ms: 1,
        }
    }
    #[test]
    fn persists_idempotent_redemption() {
        let s = SqliteRedemptionStore::open(":memory:").unwrap();
        assert_eq!(s.create(&order()).unwrap(), s.create(&order()).unwrap());
        assert_eq!(
            s.mark_burned("r1", Some("0xabc")).unwrap().status,
            RedemptionStatus::Burned
        );
        assert_eq!(
            s.complete("r1").unwrap().status,
            RedemptionStatus::Completed
        );
    }
}
