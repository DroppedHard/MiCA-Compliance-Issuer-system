use crate::application::{AddressRestriction, AddressRestrictionError, AddressRestrictionStore};
use rusqlite::{Connection, params};
use std::{fs, path::Path, sync::Mutex};

pub struct SqliteAddressRestrictionStore {
    connection: Mutex<Connection>,
}

impl SqliteAddressRestrictionStore {
    pub fn open(path: &str) -> Result<Self, AddressRestrictionError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!(
                "../../../migrations/0009_address_restrictions.sql"
            ))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl AddressRestrictionStore for SqliteAddressRestrictionStore {
    fn list(&self) -> Result<Vec<AddressRestriction>, AddressRestrictionError> {
        let connection = self.connection.lock().map_err(storage)?;
        let mut statement = connection.prepare("SELECT address,reason,active,transaction_hash,updated_at_unix_ms FROM address_restrictions ORDER BY updated_at_unix_ms DESC").map_err(storage)?;
        statement
            .query_map([], |row| {
                Ok(AddressRestriction {
                    address: row.get(0)?,
                    reason: row.get(1)?,
                    active: row.get::<_, i64>(2)? != 0,
                    transaction_hash: row.get(3)?,
                    updated_at_unix_ms: row.get::<_, i64>(4)? as u64,
                })
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)
    }

    fn save(&self, entry: &AddressRestriction) -> Result<(), AddressRestrictionError> {
        self.connection
            .lock()
            .map_err(storage)?
            .execute(
                "INSERT INTO address_restrictions(normalized_address,address,reason,active,transaction_hash,updated_at_unix_ms) VALUES(lower(?1),?1,?2,?3,?4,?5) ON CONFLICT(normalized_address) DO UPDATE SET address=excluded.address,reason=excluded.reason,active=excluded.active,transaction_hash=excluded.transaction_hash,updated_at_unix_ms=excluded.updated_at_unix_ms",
                params![entry.address, entry.reason, entry.active as i64, entry.transaction_hash, entry.updated_at_unix_ms as i64],
            )
            .map_err(storage)?;
        Ok(())
    }
}

fn storage(error: impl std::fmt::Display) -> AddressRestrictionError {
    AddressRestrictionError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_current_state_and_audit_details() {
        let store = SqliteAddressRestrictionStore::open(":memory:").unwrap();
        let mut entry = AddressRestriction {
            address: "0x0000000000000000000000000000000000000001".into(),
            reason: "test".into(),
            active: true,
            transaction_hash: Some("0xabc".into()),
            updated_at_unix_ms: 1,
        };
        store.save(&entry).unwrap();
        entry.active = false;
        entry.updated_at_unix_ms = 2;
        store.save(&entry).unwrap();
        let entries = store.list().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].active);
    }
}
