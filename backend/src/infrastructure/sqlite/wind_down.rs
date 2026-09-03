use crate::application::{WindDownAudit, WindDownAuditStore, WindDownError};
use rusqlite::{Connection, OptionalExtension, params};
use std::{fs, path::Path, sync::Mutex};

pub struct SqliteWindDownAuditStore {
    connection: Mutex<Connection>,
}

impl SqliteWindDownAuditStore {
    pub fn open(path: &str) -> Result<Self, WindDownError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!("../../../migrations/0007_wind_down_audit.sql"))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl WindDownAuditStore for SqliteWindDownAuditStore {
    fn get(&self, operation_id: &str) -> Result<Option<WindDownAudit>, WindDownError> {
        self.connection
            .lock()
            .map_err(storage)?
            .query_row(
                "SELECT operation_id,reason,transaction_hash,confirmed_at_unix_ms FROM wind_down_audit WHERE operation_id=?1",
                [operation_id],
                |row| Ok(WindDownAudit {
                    operation_id: row.get(0)?,
                    reason: row.get(1)?,
                    transaction_hash: row.get(2)?,
                    confirmed_at_unix_ms: row.get::<_, i64>(3)? as u64,
                }),
            )
            .optional()
            .map_err(storage)
    }

    fn append(&self, audit: &WindDownAudit) -> Result<(), WindDownError> {
        self.connection
            .lock()
            .map_err(storage)?
            .execute(
                "INSERT INTO wind_down_audit VALUES(?1,?2,?3,?4)",
                params![
                    audit.operation_id,
                    audit.reason,
                    audit.transaction_hash,
                    audit.confirmed_at_unix_ms as i64,
                ],
            )
            .map_err(storage)?;
        Ok(())
    }
}

fn storage(error: impl std::fmt::Display) -> WindDownError {
    WindDownError::Storage(error.to_string())
}
