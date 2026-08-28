use crate::{
    application::{CaspReportStore, CaspReportingError, IssuanceRestriction, OperationGateError},
    domain::{
        CaspDailyAggregate, CaspDailyReport, ClassificationAggregate,
        QuarterlyTransactionAssessment,
    },
};
use rusqlite::{Connection, params};
use std::{fs, path::Path, sync::Mutex};
pub struct SqliteCaspReportStore {
    connection: Mutex<Connection>,
}
impl SqliteCaspReportStore {
    pub fn open(path: &str) -> Result<Self, CaspReportingError> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(storage)?
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(include_str!("../../migrations/0008_casp_reporting.sql"))
            .map_err(storage)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}
impl CaspReportStore for SqliteCaspReportStore {
    fn import(&self, report: &CaspDailyReport) -> Result<(), CaspReportingError> {
        let mut connection = self.connection.lock().map_err(storage)?;
        let tx = connection.transaction().map_err(storage)?;
        let imported = now();
        for day in &report.days {
            tx.execute("INSERT INTO casp_daily_transaction_reports(date_utc,asset_symbol,currency_area,total_operation_count,total_value_raw,total_value_usd_minor,means_of_exchange_count,means_of_exchange_value_raw,means_of_exchange_value_usd_minor,means_of_exchange_value_eur_minor,excluded_operation_count,known_onchain_overlap_count,known_onchain_overlap_value_raw,classifications_json,methodology_version,conversion_methodology,imported_at_unix_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17) ON CONFLICT(date_utc) DO UPDATE SET asset_symbol=excluded.asset_symbol,currency_area=excluded.currency_area,total_operation_count=excluded.total_operation_count,total_value_raw=excluded.total_value_raw,total_value_usd_minor=excluded.total_value_usd_minor,means_of_exchange_count=excluded.means_of_exchange_count,means_of_exchange_value_raw=excluded.means_of_exchange_value_raw,means_of_exchange_value_usd_minor=excluded.means_of_exchange_value_usd_minor,means_of_exchange_value_eur_minor=excluded.means_of_exchange_value_eur_minor,excluded_operation_count=excluded.excluded_operation_count,known_onchain_overlap_count=excluded.known_onchain_overlap_count,known_onchain_overlap_value_raw=excluded.known_onchain_overlap_value_raw,classifications_json=excluded.classifications_json,methodology_version=excluded.methodology_version,conversion_methodology=excluded.conversion_methodology,imported_at_unix_ms=excluded.imported_at_unix_ms",params![day.date_utc,day.asset_symbol,day.currency_area,as_i64(day.total_operation_count)?,number(&day.total_value_raw)?,number(&day.total_value_usd_minor)?,as_i64(day.means_of_exchange_count)?,number(&day.means_of_exchange_value_raw)?,number(&day.means_of_exchange_value_usd_minor)?,number(&day.means_of_exchange_value_eur_minor)?,as_i64(day.excluded_operation_count)?,as_i64(day.known_onchain_overlap_count)?,number(&day.known_onchain_overlap_value_raw)?,serde_json::to_string(&day.classifications).map_err(storage)?,day.methodology_version,day.conversion_methodology,imported as i64]).map_err(storage)?;
        }
        tx.execute("INSERT INTO casp_report_imports(from_date_utc,to_date_utc,imported_at_unix_ms) VALUES(?1,?2,?3)",params![report.from_date_utc,report.to_date_utc,imported as i64]).map_err(storage)?;
        tx.commit().map_err(storage)
    }
    fn daily(&self, from: &str, to: &str) -> Result<Vec<CaspDailyAggregate>, CaspReportingError> {
        let connection = self.connection.lock().map_err(storage)?;
        let mut statement=connection.prepare("SELECT date_utc,asset_symbol,currency_area,total_operation_count,total_value_raw,total_value_usd_minor,means_of_exchange_count,means_of_exchange_value_raw,means_of_exchange_value_usd_minor,means_of_exchange_value_eur_minor,excluded_operation_count,known_onchain_overlap_count,known_onchain_overlap_value_raw,classifications_json,methodology_version,conversion_methodology FROM casp_daily_transaction_reports WHERE date_utc BETWEEN ?1 AND ?2 ORDER BY date_utc").map_err(storage)?;
        statement
            .query_map(params![from, to], |r| {
                let json: String = r.get(13)?;
                let classifications: Vec<ClassificationAggregate> = serde_json::from_str(&json)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            13,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(CaspDailyAggregate {
                    date_utc: r.get(0)?,
                    asset_symbol: r.get(1)?,
                    currency_area: r.get(2)?,
                    total_operation_count: r.get::<_, i64>(3)? as u64,
                    total_value_raw: r.get::<_, i64>(4)?.to_string(),
                    total_value_usd_minor: r.get::<_, i64>(5)?.to_string(),
                    means_of_exchange_count: r.get::<_, i64>(6)? as u64,
                    means_of_exchange_value_raw: r.get::<_, i64>(7)?.to_string(),
                    means_of_exchange_value_usd_minor: r.get::<_, i64>(8)?.to_string(),
                    means_of_exchange_value_eur_minor: r.get::<_, i64>(9)?.to_string(),
                    excluded_operation_count: r.get::<_, i64>(10)? as u64,
                    known_onchain_overlap_count: r.get::<_, i64>(11)? as u64,
                    known_onchain_overlap_value_raw: r.get::<_, i64>(12)?.to_string(),
                    classifications,
                    methodology_version: r.get(14)?,
                    conversion_methodology: r.get(15)?,
                })
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)
    }
    fn range_was_imported(&self, from: &str, to: &str) -> Result<bool, CaspReportingError> {
        let connection = self.connection.lock().map_err(storage)?;
        let count:i64=connection.query_row("SELECT COUNT(*) FROM casp_report_imports WHERE from_date_utc<=?1 AND to_date_utc>=?2",params![from,to],|r|r.get(0)).map_err(storage)?;
        Ok(count > 0)
    }
    fn save_assessment(
        &self,
        assessment: &QuarterlyTransactionAssessment,
    ) -> Result<(), CaspReportingError> {
        let mut connection = self.connection.lock().map_err(storage)?;
        let tx = connection.transaction().map_err(storage)?;
        let evidence = serde_json::to_string(assessment).map_err(storage)?;
        let timestamp = now();
        tx.execute("INSERT INTO casp_quarterly_assessments(year,quarter,evidence_json,complete_source_range,threshold_breached,threshold_enforceable,assessed_at_unix_ms) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(year,quarter) DO UPDATE SET evidence_json=excluded.evidence_json,complete_source_range=excluded.complete_source_range,threshold_breached=excluded.threshold_breached,threshold_enforceable=excluded.threshold_enforceable,assessed_at_unix_ms=excluded.assessed_at_unix_ms",params![assessment.year,assessment.quarter,evidence,i64::from(assessment.complete_source_range),i64::from(assessment.threshold_breached),i64::from(assessment.threshold_enforceable),timestamp as i64]).map_err(storage)?;
        if assessment.threshold_enforceable {
            let reason = format!(
                "issuance is blocked because complete Q{} {} evidence exceeds both Article 23 demo thresholds",
                assessment.quarter, assessment.year
            );
            tx.execute("UPDATE activity_issuance_gate SET blocked=1,reason=?1,evidence_json=?2,updated_at_unix_ms=?3 WHERE singleton=1",params![reason,evidence,timestamp as i64]).map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }
}
impl IssuanceRestriction for SqliteCaspReportStore {
    fn block_reason(&self) -> Result<Option<String>, OperationGateError> {
        self.connection.lock().map_err(gate_storage)?.query_row("SELECT CASE WHEN blocked=1 THEN reason ELSE NULL END FROM activity_issuance_gate WHERE singleton=1",[],|row|row.get(0)).map_err(gate_storage)
    }
}
fn number(value: &str) -> Result<i64, CaspReportingError> {
    value.parse::<u64>().map_err(storage).and_then(as_i64)
}
fn as_i64(value: u64) -> Result<i64, CaspReportingError> {
    i64::try_from(value).map_err(storage)
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn storage(error: impl std::fmt::Display) -> CaspReportingError {
    CaspReportingError::Storage(error.to_string())
}
fn gate_storage(error: impl std::fmt::Display) -> OperationGateError {
    OperationGateError::Storage(error.to_string())
}
