pub mod address_restrictions;
pub mod asset_state;
pub mod casp_reporting;
pub mod issuance;
pub mod operation_decisions;
pub mod redemption;
pub mod wind_down;

use crate::{
    application::{EsgStore, EsgStoreError},
    config::esg,
    domain::{EsgEstimate, EsgObservation},
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    fs,
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use time::{Date, Duration};

pub struct SqliteEsgStore {
    connection: Mutex<Connection>,
}

impl SqliteEsgStore {
    pub fn open(path: &str) -> Result<Self, EsgStoreError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!("../../../migrations/0001_esg.sql"))
            .map_err(storage)?;
        ensure_column(
            &connection,
            "energy_lower_milliwh",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &connection,
            "energy_upper_milliwh",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &connection,
            "data_origin",
            "TEXT NOT NULL DEFAULT 'observed'",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Inserts a finalized demonstration day without replacing observed data.
    pub fn seed_demo_day(
        &self,
        chain_id: u64,
        contract: &str,
        date: &str,
        count: u64,
    ) -> Result<bool, EsgStoreError> {
        let estimate = esg::estimate(date.to_owned(), count, "finalized");
        let now = unix_ms() as i64;
        let mut connection = self.connection.lock().map_err(storage)?;
        let tx = connection.transaction().map_err(storage)?;
        tx.execute("INSERT OR IGNORE INTO token_daily_activity(date_utc,chain_id,contract_address,transaction_count,first_block,last_block,finalized_at_unix_ms) VALUES (?1,?2,?3,?4,0,0,?5)", params![date,chain_id as i64,contract,count as i64,now]).map_err(storage)?;
        let inserted = tx.execute("INSERT OR IGNORE INTO esg_daily_estimates(date_utc,chain_id,contract_address,transaction_count,energy_milliwh,energy_lower_milliwh,energy_upper_milliwh,emissions_milligram_co2e,renewable_energy_milliwh,nuclear_energy_milliwh,fossil_energy_milliwh,methodology_version,calculated_at_unix_ms,data_origin) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'demo_seed')", params![date,chain_id as i64,contract,count as i64,(estimate.energy_best_guess_wh*1000.0) as i64,(estimate.energy_lower_wh*1000.0) as i64,(estimate.energy_upper_wh*1000.0) as i64,(estimate.emissions_g_co2e*1000.0) as i64,(estimate.renewable_energy_wh*1000.0) as i64,(estimate.nuclear_energy_wh*1000.0) as i64,(estimate.fossil_energy_wh*1000.0) as i64,esg::METHODOLOGY_VERSION,now]).map_err(storage)? > 0;
        tx.commit().map_err(storage)?;
        Ok(inserted)
    }

    /// Seeds the seven completed UTC days preceding `today` with deliberately
    /// small demo activity. At the configured Cambridge best-guess allocation,
    /// 6-9 transactions produce roughly 118-177 Wh per day.
    pub fn seed_demo_week(
        &self,
        chain_id: u64,
        contract: &str,
        today: Date,
    ) -> Result<usize, EsgStoreError> {
        let transaction_counts = [6_u64, 8, 7, 9, 6, 8, 7];
        let mut inserted = 0;
        for (offset, count) in (1_i64..=7).rev().zip(transaction_counts) {
            let date = (today - Duration::days(offset)).to_string();
            inserted += usize::from(self.seed_demo_day(chain_id, contract, &date, count)?);
        }
        Ok(inserted)
    }
}

impl EsgStore for SqliteEsgStore {
    fn last_processed_block(
        &self,
        chain_id: u64,
        contract: &str,
    ) -> Result<Option<u64>, EsgStoreError> {
        self.connection.lock().map_err(storage)?.query_row("SELECT last_processed_block FROM observer_state WHERE chain_id=?1 AND contract_address=?2", params![chain_id as i64, contract], |row| row.get::<_, i64>(0)).optional().map(|value| value.map(|value| value as u64)).map_err(storage)
    }

    fn record_observation(
        &self,
        chain_id: u64,
        contract: &str,
        block: u64,
        date: &str,
        count: u64,
    ) -> Result<EsgObservation, EsgStoreError> {
        let now = unix_ms();
        let (chain, block, count) = (chain_id as i64, block as i64, count as i64);
        let mut connection = self.connection.lock().map_err(storage)?;
        let tx = connection.transaction().map_err(storage)?;
        tx.execute(
            "UPDATE token_daily_activity SET finalized_at_unix_ms=?1 WHERE date_utc<?2 AND finalized_at_unix_ms IS NULL",
            params![now as i64, date],
        )
        .map_err(storage)?;
        let previous = tx.query_row("SELECT last_processed_block FROM observer_state WHERE chain_id=?1 AND contract_address=?2", params![chain, contract], |row| row.get::<_, i64>(0)).optional().map_err(storage)?;
        let increment = if previous.is_some_and(|value| value >= block) {
            0
        } else {
            count
        };
        tx.execute("INSERT INTO observer_state VALUES (?1,?2,?3,?4) ON CONFLICT(chain_id,contract_address) DO UPDATE SET last_processed_block=MAX(observer_state.last_processed_block,excluded.last_processed_block),updated_at_unix_ms=excluded.updated_at_unix_ms", params![chain,contract,block,now as i64]).map_err(storage)?;
        tx.execute("INSERT INTO token_daily_activity(date_utc,chain_id,contract_address,transaction_count,first_block,last_block) VALUES (?1,?2,?3,?4,?5,?5) ON CONFLICT(date_utc,chain_id,contract_address) DO UPDATE SET transaction_count=transaction_count+excluded.transaction_count,last_block=excluded.last_block", params![date,chain,contract,increment,block]).map_err(storage)?;
        let total = tx.query_row("SELECT transaction_count FROM token_daily_activity WHERE date_utc=?1 AND chain_id=?2 AND contract_address=?3", params![date,chain,contract], |row| row.get::<_, i64>(0)).map_err(storage)? as u64;
        let estimate = esg::estimate(date.to_owned(), total, "provisional");
        tx.execute("INSERT INTO esg_daily_estimates(date_utc,chain_id,contract_address,transaction_count,energy_milliwh,energy_lower_milliwh,energy_upper_milliwh,emissions_milligram_co2e,renewable_energy_milliwh,nuclear_energy_milliwh,fossil_energy_milliwh,methodology_version,calculated_at_unix_ms,data_origin) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'observed') ON CONFLICT(date_utc,chain_id,contract_address) DO UPDATE SET transaction_count=excluded.transaction_count,energy_milliwh=excluded.energy_milliwh,energy_lower_milliwh=excluded.energy_lower_milliwh,energy_upper_milliwh=excluded.energy_upper_milliwh,emissions_milligram_co2e=excluded.emissions_milligram_co2e,renewable_energy_milliwh=excluded.renewable_energy_milliwh,nuclear_energy_milliwh=excluded.nuclear_energy_milliwh,fossil_energy_milliwh=excluded.fossil_energy_milliwh,calculated_at_unix_ms=excluded.calculated_at_unix_ms,data_origin='observed'", params![date,chain,contract,total as i64,(estimate.energy_best_guess_wh*1000.0) as i64,(estimate.energy_lower_wh*1000.0) as i64,(estimate.energy_upper_wh*1000.0) as i64,(estimate.emissions_g_co2e*1000.0) as i64,(estimate.renewable_energy_wh*1000.0) as i64,(estimate.nuclear_energy_wh*1000.0) as i64,(estimate.fossil_energy_wh*1000.0) as i64,esg::METHODOLOGY_VERSION,now as i64]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(EsgObservation {
            observed_at_unix_ms: now,
            last_processed_block: block as u64,
            chain_id,
            contract_address: contract.to_owned(),
            current_day: estimate,
            methodology: esg::methodology(),
        })
    }

    fn recent_estimates(
        &self,
        chain_id: u64,
        contract: &str,
        limit: u8,
    ) -> Result<Vec<EsgEstimate>, EsgStoreError> {
        let connection = self.connection.lock().map_err(storage)?;
        let mut statement = connection.prepare("SELECT e.date_utc,e.transaction_count,e.energy_lower_milliwh,e.energy_milliwh,e.energy_upper_milliwh,e.emissions_milligram_co2e,e.renewable_energy_milliwh,e.nuclear_energy_milliwh,e.fossil_energy_milliwh,e.data_origin,a.finalized_at_unix_ms FROM esg_daily_estimates e LEFT JOIN token_daily_activity a USING(date_utc,chain_id,contract_address) WHERE e.chain_id=?1 AND e.contract_address=?2 ORDER BY e.date_utc DESC LIMIT ?3").map_err(storage)?;
        let rows = statement
            .query_map(params![chain_id as i64, contract, limit as i64], |row| {
                let origin: String = row.get(9)?;
                let finalized: Option<i64> = row.get(10)?;
                Ok(EsgEstimate {
                    date_utc: row.get(0)?,
                    status: if finalized.is_some() {
                        "finalized"
                    } else {
                        "provisional"
                    },
                    transaction_count: row.get::<_, i64>(1)? as u64,
                    data_origin: if origin == "demo_seed" {
                        "demoSeed"
                    } else {
                        "observed"
                    },
                    energy_lower_wh: row.get::<_, i64>(2)? as f64 / 1000.0,
                    energy_best_guess_wh: row.get::<_, i64>(3)? as f64 / 1000.0,
                    energy_upper_wh: row.get::<_, i64>(4)? as f64 / 1000.0,
                    emissions_g_co2e: row.get::<_, i64>(5)? as f64 / 1000.0,
                    renewable_energy_wh: row.get::<_, i64>(6)? as f64 / 1000.0,
                    nuclear_energy_wh: row.get::<_, i64>(7)? as f64 / 1000.0,
                    fossil_energy_wh: row.get::<_, i64>(8)? as f64 / 1000.0,
                })
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        Ok(rows.into_iter().rev().collect())
    }
}

fn ensure_column(
    connection: &Connection,
    name: &str,
    definition: &str,
) -> Result<(), EsgStoreError> {
    let exists = connection
        .prepare("PRAGMA table_info(esg_daily_estimates)")
        .map_err(storage)?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage)?
        .any(|column| column.is_ok_and(|column| column == name));
    if !exists {
        connection
            .execute(
                &format!("ALTER TABLE esg_daily_estimates ADD COLUMN {name} {definition}"),
                [],
            )
            .map_err(storage)?;
    }
    Ok(())
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn storage(error: impl std::fmt::Display) -> EsgStoreError {
    EsgStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteEsgStore {
        SqliteEsgStore::open(":memory:").expect("in-memory database should open")
    }

    fn activity_count(store: &SqliteEsgStore, date: &str, chain: i64, contract: &str) -> i64 {
        store.connection.lock().unwrap().query_row(
            "SELECT transaction_count FROM token_daily_activity WHERE date_utc=?1 AND chain_id=?2 AND contract_address=?3",
            params![date, chain, contract],
            |row| row.get(0),
        ).unwrap()
    }

    #[test]
    fn accumulates_new_blocks_and_persists_the_checkpoint() {
        let store = store();
        store
            .record_observation(1, "0xabc", 10, "2026-08-22", 2)
            .unwrap();
        let result = store
            .record_observation(1, "0xabc", 12, "2026-08-22", 3)
            .unwrap();

        assert_eq!(store.last_processed_block(1, "0xabc").unwrap(), Some(12));
        assert_eq!(result.current_day.transaction_count, 5);
        assert_eq!(result.current_day.energy_best_guess_wh, 98.375);
    }

    #[test]
    fn replayed_or_older_blocks_are_idempotent_and_do_not_regress_checkpoint() {
        let store = store();
        store
            .record_observation(1, "0xabc", 12, "2026-08-22", 4)
            .unwrap();
        store
            .record_observation(1, "0xabc", 12, "2026-08-22", 4)
            .unwrap();
        let result = store
            .record_observation(1, "0xabc", 11, "2026-08-22", 99)
            .unwrap();

        assert_eq!(result.current_day.transaction_count, 4);
        assert_eq!(store.last_processed_block(1, "0xabc").unwrap(), Some(12));
    }

    #[test]
    fn separates_contracts_and_chains() {
        let store = store();
        store
            .record_observation(1, "0xaaa", 5, "2026-08-22", 2)
            .unwrap();
        store
            .record_observation(1, "0xbbb", 5, "2026-08-22", 3)
            .unwrap();
        store
            .record_observation(2, "0xaaa", 5, "2026-08-22", 4)
            .unwrap();

        assert_eq!(activity_count(&store, "2026-08-22", 1, "0xaaa"), 2);
        assert_eq!(activity_count(&store, "2026-08-22", 1, "0xbbb"), 3);
        assert_eq!(activity_count(&store, "2026-08-22", 2, "0xaaa"), 4);
    }

    #[test]
    fn finalizes_previous_days_without_changing_their_estimate() {
        let store = store();
        store
            .record_observation(1, "0xabc", 5, "2026-08-21", 2)
            .unwrap();
        store
            .record_observation(1, "0xabc", 6, "2026-08-22", 1)
            .unwrap();
        let connection = store.connection.lock().unwrap();
        let finalized: Option<i64> = connection
            .query_row(
                "SELECT finalized_at_unix_ms FROM token_daily_activity WHERE date_utc='2026-08-21'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let estimate_count: i64 = connection
            .query_row(
                "SELECT transaction_count FROM esg_daily_estimates WHERE date_utc='2026-08-21'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(finalized.is_some());
        assert_eq!(estimate_count, 2);
    }

    #[test]
    fn checkpoint_and_daily_aggregate_survive_process_restart() {
        let path = std::env::temp_dir().join(format!(
            "crypto-asset-esg-{}.sqlite",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let store = SqliteEsgStore::open(path.to_str().unwrap()).unwrap();
            store
                .record_observation(1, "0xabc", 20, "2026-08-22", 3)
                .unwrap();
        }
        {
            let reopened = SqliteEsgStore::open(path.to_str().unwrap()).unwrap();
            assert_eq!(reopened.last_processed_block(1, "0xabc").unwrap(), Some(20));
            let result = reopened
                .record_observation(1, "0xabc", 21, "2026-08-22", 2)
                .unwrap();
            assert_eq!(result.current_day.transaction_count, 5);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn demo_seed_is_idempotent_and_history_contains_all_three_scenarios() {
        let store = store();
        assert!(store.seed_demo_day(1, "0xabc", "2026-08-21", 100).unwrap());
        assert!(!store.seed_demo_day(1, "0xabc", "2026-08-21", 999).unwrap());

        let history = store.recent_estimates(1, "0xabc", 7).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].transaction_count, 100);
        assert_eq!(history[0].data_origin, "demoSeed");
        assert_eq!(history[0].energy_lower_wh, 315.0);
        assert_eq!(history[0].energy_best_guess_wh, 1_967.5);
        assert_eq!(history[0].energy_upper_wh, 2_872.5);
    }

    #[test]
    fn demo_seed_never_overwrites_an_observed_day() {
        let store = store();
        store
            .record_observation(1, "0xabc", 10, "2026-08-21", 7)
            .unwrap();

        assert!(!store.seed_demo_day(1, "0xabc", "2026-08-21", 999).unwrap());
        let day = &store.recent_estimates(1, "0xabc", 7).unwrap()[0];
        assert_eq!(day.transaction_count, 7);
        assert_eq!(day.data_origin, "observed");
    }

    #[test]
    fn demo_week_is_relative_idempotent_and_stays_near_live_demo_scale() {
        let store = store();
        let today = Date::from_calendar_date(2026, time::Month::August, 30).unwrap();
        assert_eq!(store.seed_demo_week(1, "0xabc", today).unwrap(), 7);
        assert_eq!(store.seed_demo_week(1, "0xabc", today).unwrap(), 0);
        let history = store.recent_estimates(1, "0xabc", 7).unwrap();
        assert_eq!(history.first().unwrap().date_utc, "2026-08-23");
        assert_eq!(history.last().unwrap().date_utc, "2026-08-29");
        assert!(history.iter().all(|day| {
            (100.0..=200.0).contains(&day.energy_best_guess_wh)
                && day.data_origin == "demoSeed"
                && day.status == "finalized"
        }));
    }

    #[test]
    fn opening_a_legacy_database_adds_scenario_columns_without_losing_rows() {
        let path = std::env::temp_dir().join(format!("crypto-asset-legacy-{}.sqlite", unix_ms()));
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch("CREATE TABLE esg_daily_estimates(date_utc TEXT NOT NULL,chain_id INTEGER NOT NULL,contract_address TEXT NOT NULL,transaction_count INTEGER NOT NULL,energy_milliwh INTEGER NOT NULL,emissions_milligram_co2e INTEGER NOT NULL,renewable_energy_milliwh INTEGER NOT NULL,nuclear_energy_milliwh INTEGER NOT NULL,fossil_energy_milliwh INTEGER NOT NULL,methodology_version TEXT NOT NULL,calculated_at_unix_ms INTEGER NOT NULL,PRIMARY KEY(date_utc,chain_id,contract_address)); INSERT INTO esg_daily_estimates VALUES('2026-08-20',1,'0xabc',1,19675,5925,7732,3344,8578,'v0',1);").unwrap();
        }

        let store = SqliteEsgStore::open(path.to_str().unwrap()).unwrap();
        let connection = store.connection.lock().unwrap();
        let preserved: i64 = connection
            .query_row(
                "SELECT transaction_count FROM esg_daily_estimates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let origin: String = connection
            .query_row("SELECT data_origin FROM esg_daily_estimates", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(preserved, 1);
        assert_eq!(origin, "observed");
        drop(connection);
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
