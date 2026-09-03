use crate::{
    application::{AssetStateError, AssetStateStore},
    domain::{AssetState, AssetStateCode},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{fs, path::Path, sync::Mutex};

pub struct SqliteAssetStateStore {
    connection: Mutex<Connection>,
}

impl SqliteAssetStateStore {
    pub fn open(path: &str) -> Result<Self, AssetStateError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!("../../../migrations/0004_asset_state.sql"))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl AssetStateStore for SqliteAssetStateStore {
    fn load(&self) -> Result<Option<AssetState>, AssetStateError> {
        self.connection.lock().map_err(storage)?.query_row(
            "SELECT state,reason,reserve_coverage_percent,evidence_at_unix_ms,policy_version,updated_at_unix_ms FROM asset_state WHERE singleton_id=1", [],
            |row| { let value: String = row.get(0)?; Ok(AssetState { state: parse_state(&value), reason: row.get(1)?, reserve_coverage_percent: row.get(2)?, evidence_at_unix_ms: row.get::<_, Option<i64>>(3)?.map(|v| v as u64), policy_version: row.get(4)?, updated_at_unix_ms: row.get::<_, i64>(5)? as u64 }) }
        ).optional().map_err(storage)
    }

    fn save(&self, state: &AssetState) -> Result<(), AssetStateError> {
        self.connection.lock().map_err(storage)?.execute(
            "UPDATE asset_state SET state=?1,reason=?2,reserve_coverage_percent=?3,evidence_at_unix_ms=?4,policy_version=?5,updated_at_unix_ms=?6 WHERE singleton_id=1",
            params![format_state(state.state), state.reason, state.reserve_coverage_percent, state.evidence_at_unix_ms.map(|v| v as i64), state.policy_version, state.updated_at_unix_ms as i64]
        ).map_err(storage)?;
        Ok(())
    }
}

fn format_state(value: AssetStateCode) -> &'static str {
    match value {
        AssetStateCode::Active => "active",
        AssetStateCode::Warning => "warning",
        AssetStateCode::MintBlocked => "mint_blocked",
        AssetStateCode::DataUnavailable => "data_unavailable",
        AssetStateCode::WindDown => "wind_down",
    }
}
fn parse_state(value: &str) -> AssetStateCode {
    match value {
        "active" => AssetStateCode::Active,
        "warning" => AssetStateCode::Warning,
        "mint_blocked" => AssetStateCode::MintBlocked,
        "wind_down" => AssetStateCode::WindDown,
        _ => AssetStateCode::DataUnavailable,
    }
}
fn storage(error: impl std::fmt::Display) -> AssetStateError {
    AssetStateError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initializes_and_persists_the_single_asset_state() {
        let store = SqliteAssetStateStore::open(":memory:").unwrap();
        assert_eq!(
            store.load().unwrap().unwrap().state,
            AssetStateCode::DataUnavailable
        );
        let state = AssetState {
            state: AssetStateCode::Warning,
            reason: "test".into(),
            reserve_coverage_percent: Some(101.0),
            evidence_at_unix_ms: Some(2),
            policy_version: "v1".into(),
            updated_at_unix_ms: 3,
        };
        store.save(&state).unwrap();
        assert_eq!(store.load().unwrap(), Some(state));
    }
}
