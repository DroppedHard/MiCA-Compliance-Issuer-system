use crate::domain::{CaspDailyAggregate, CaspDailyReport, QuarterlyTransactionAssessment};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
pub const QUARTERLY_METHODOLOGY_VERSION: &str = "issuer-casp-quarterly-threshold-v1";
pub const DAILY_TRANSACTION_THRESHOLD: u64 = 1_000_000;
pub const DAILY_VALUE_THRESHOLD_EUR_MINOR: u64 = 20_000_000_000;
#[async_trait]
pub trait CaspReportSource: Send + Sync {
    async fn fetch(&self, from: &str, to: &str) -> Result<CaspDailyReport, CaspReportingError>;
}
pub trait CaspReportStore: Send + Sync {
    fn import(&self, report: &CaspDailyReport) -> Result<(), CaspReportingError>;
    fn daily(&self, from: &str, to: &str) -> Result<Vec<CaspDailyAggregate>, CaspReportingError>;
    fn range_was_imported(&self, from: &str, to: &str) -> Result<bool, CaspReportingError>;
    fn save_assessment(
        &self,
        assessment: &QuarterlyTransactionAssessment,
    ) -> Result<(), CaspReportingError>;
}
#[async_trait]
pub trait ActivityIssuanceController: Send + Sync {
    async fn synchronize_issuance_restriction(
        &self,
        assessment: &QuarterlyTransactionAssessment,
    ) -> Result<(), CaspReportingError>;
}
#[derive(Clone)]
pub struct CaspReportingService {
    source: Arc<dyn CaspReportSource>,
    store: Arc<dyn CaspReportStore>,
    issuance_controller: Option<Arc<dyn ActivityIssuanceController>>,
}
impl CaspReportingService {
    pub fn new(source: Arc<dyn CaspReportSource>, store: Arc<dyn CaspReportStore>) -> Self {
        Self {
            source,
            store,
            issuance_controller: None,
        }
    }
    pub fn with_issuance_controller(
        mut self,
        controller: Arc<dyn ActivityIssuanceController>,
    ) -> Self {
        self.issuance_controller = Some(controller);
        self
    }
    pub async fn ingest(
        &self,
        from: &str,
        to: &str,
    ) -> Result<CaspDailyReport, CaspReportingError> {
        validate_range(from, to)?;
        let report = self.source.fetch(from, to).await?;
        if report.from_date_utc != from || report.to_date_utc != to {
            return Err(CaspReportingError::SourceContract(
                "CASP returned a different date range".into(),
            ));
        }
        self.import_report(from, to, report).await
    }
    async fn import_report(
        &self,
        from: &str,
        to: &str,
        report: CaspDailyReport,
    ) -> Result<CaspDailyReport, CaspReportingError> {
        for day in &report.days {
            if day.asset_symbol != "rUSD"
                || day.currency_area != "USD"
                || day.conversion_methodology != "demo-usd-eur-parity-v1"
                || day.date_utc.as_str() < from
                || day.date_utc.as_str() > to
            {
                return Err(CaspReportingError::SourceContract(
                    "CASP aggregate violates the rUSD reporting contract".into(),
                ));
            }
        }
        self.store.import(&report)?;
        if let Some((year, quarter)) = exact_quarter(from, to) {
            let assessment = self.quarterly(year, quarter)?;
            self.store.save_assessment(&assessment)?;
            if let Some(controller) = &self.issuance_controller {
                controller
                    .synchronize_issuance_restriction(&assessment)
                    .await?;
            }
        }
        Ok(report)
    }
    pub async fn run_demo_threshold_breach(
        &self,
        year: i32,
        quarter: u8,
    ) -> Result<QuarterlyTransactionAssessment, CaspReportingError> {
        let (from, to, calendar_days) = quarter_range(year, quarter)?;
        let daily_count = DAILY_TRANSACTION_THRESHOLD + 1;
        let daily_value_minor = DAILY_VALUE_THRESHOLD_EUR_MINOR + 1;
        let count = daily_count
            .checked_mul(calendar_days)
            .ok_or(CaspReportingError::Overflow)?;
        let value_minor = daily_value_minor
            .checked_mul(calendar_days)
            .ok_or(CaspReportingError::Overflow)?;
        let value_raw = value_minor
            .checked_mul(10_000)
            .ok_or(CaspReportingError::Overflow)?;
        let report = CaspDailyReport {
            from_date_utc: from.clone(),
            to_date_utc: to.clone(),
            // One compact aggregate represents the synthetic quarter total. The
            // assessment still divides it by every calendar day in the quarter.
            days: vec![CaspDailyAggregate {
                date_utc: from.clone(),
                asset_symbol: "rUSD".into(),
                currency_area: "USD".into(),
                total_operation_count: count,
                total_value_raw: value_raw.to_string(),
                total_value_usd_minor: value_minor.to_string(),
                means_of_exchange_count: count,
                means_of_exchange_value_raw: value_raw.to_string(),
                means_of_exchange_value_usd_minor: value_minor.to_string(),
                means_of_exchange_value_eur_minor: value_minor.to_string(),
                excluded_operation_count: 0,
                known_onchain_overlap_count: 0,
                known_onchain_overlap_value_raw: "0".into(),
                classifications: vec![crate::domain::ClassificationAggregate {
                    classification: "goods_or_services".into(),
                    operation_count: count,
                    value_raw: value_raw.to_string(),
                }],
                methodology_version: "casp-daily-activity-v1-demo-threshold-seed".into(),
                conversion_methodology: "demo-usd-eur-parity-v1".into(),
            }],
        };
        self.import_report(&from, &to, report).await?;
        self.quarterly(year, quarter)
    }
    pub fn daily(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<CaspDailyAggregate>, CaspReportingError> {
        validate_range(from, to)?;
        self.store.daily(from, to)
    }
    pub fn quarterly(
        &self,
        year: i32,
        quarter: u8,
    ) -> Result<QuarterlyTransactionAssessment, CaspReportingError> {
        let (from, to, calendar_days) = quarter_range(year, quarter)?;
        let days = self.store.daily(&from, &to)?;
        let complete = self.store.range_was_imported(&from, &to)?;
        let count = days.iter().try_fold(0_u64, |sum, day| {
            sum.checked_add(day.means_of_exchange_count)
                .ok_or(CaspReportingError::Overflow)
        })?;
        let value = days.iter().try_fold(0_u64, |sum, day| {
            let value = day
                .means_of_exchange_value_eur_minor
                .parse::<u64>()
                .map_err(|_| CaspReportingError::SourceContract("invalid EUR value".into()))?;
            sum.checked_add(value).ok_or(CaspReportingError::Overflow)
        })?;
        let average_count = count as f64 / calendar_days as f64;
        let average_value_eur = value as f64 / 100.0 / calendar_days as f64;
        let breached = average_count > DAILY_TRANSACTION_THRESHOLD as f64
            && average_value_eur > DAILY_VALUE_THRESHOLD_EUR_MINOR as f64 / 100.0;
        Ok(QuarterlyTransactionAssessment {
            year,
            quarter,
            from_date_utc: from,
            to_date_utc: to,
            calendar_day_count: calendar_days,
            imported_activity_day_count: days.len() as u64,
            complete_source_range: complete,
            means_of_exchange_operation_count: count,
            means_of_exchange_value_eur_minor: value.to_string(),
            average_daily_operation_count: average_count,
            average_daily_value_eur: average_value_eur,
            transaction_count_threshold: DAILY_TRANSACTION_THRESHOLD,
            transaction_value_threshold_eur: "200000000".into(),
            threshold_breached: breached,
            threshold_enforceable: complete && breached,
            methodology_version: QUARTERLY_METHODOLOGY_VERSION.into(),
        })
    }
}
fn quarter_range(year: i32, quarter: u8) -> Result<(String, String, u64), CaspReportingError> {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    match quarter {
        1 => Ok((
            format!("{year:04}-01-01"),
            format!("{year:04}-03-31"),
            if leap { 91 } else { 90 },
        )),
        2 => Ok((format!("{year:04}-04-01"), format!("{year:04}-06-30"), 91)),
        3 => Ok((format!("{year:04}-07-01"), format!("{year:04}-09-30"), 92)),
        4 => Ok((format!("{year:04}-10-01"), format!("{year:04}-12-31"), 92)),
        _ => Err(CaspReportingError::InvalidRange),
    }
}
fn exact_quarter(from: &str, to: &str) -> Option<(i32, u8)> {
    let year = from.get(0..4)?.parse::<i32>().ok()?;
    (1..=4).find_map(|quarter| {
        quarter_range(year, quarter)
            .ok()
            .filter(|(start, end, _)| start == from && end == to)
            .map(|_| (year, quarter))
    })
}
fn validate_range(from: &str, to: &str) -> Result<(), CaspReportingError> {
    if valid_date(from) && valid_date(to) && from <= to {
        Ok(())
    } else {
        Err(CaspReportingError::InvalidRange)
    }
}
fn valid_date(value: &str) -> bool {
    let shape = value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .chars()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shape {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let maximum = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    day > 0 && day <= maximum
}
#[derive(Debug, Error)]
pub enum CaspReportingError {
    #[error("invalid CASP reporting date range")]
    InvalidRange,
    #[error("CASP reporting source failed: {0}")]
    Source(String),
    #[error("CASP reporting source contract failed: {0}")]
    SourceContract(String),
    #[error("CASP reporting storage failed: {0}")]
    Storage(String),
    #[error("CASP reporting calculation overflow")]
    Overflow,
    #[error("on-chain issuance restriction failed: {0}")]
    Enforcement(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::IssuanceRestriction;
    use crate::{
        domain::{CaspDailyAggregate, CaspDailyReport, ClassificationAggregate},
        infrastructure::casp_reporting_sqlite::SqliteCaspReportStore,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Source {
        report: CaspDailyReport,
    }
    #[async_trait]
    impl CaspReportSource for Source {
        async fn fetch(&self, _: &str, _: &str) -> Result<CaspDailyReport, CaspReportingError> {
            Ok(self.report.clone())
        }
    }
    struct Controller(AtomicBool);
    #[async_trait]
    impl ActivityIssuanceController for Controller {
        async fn synchronize_issuance_restriction(
            &self,
            assessment: &QuarterlyTransactionAssessment,
        ) -> Result<(), CaspReportingError> {
            self.0
                .store(assessment.threshold_enforceable, Ordering::SeqCst);
            Ok(())
        }
    }
    fn day(count: u64) -> CaspDailyAggregate {
        CaspDailyAggregate {
            date_utc: "2026-01-01".into(),
            asset_symbol: "rUSD".into(),
            currency_area: "USD".into(),
            total_operation_count: count,
            total_value_raw: "18000000000900000".into(),
            total_value_usd_minor: "1800000000090".into(),
            means_of_exchange_count: count,
            means_of_exchange_value_raw: "18000000000900000".into(),
            means_of_exchange_value_usd_minor: "1800000000090".into(),
            means_of_exchange_value_eur_minor: "1800000000090".into(),
            excluded_operation_count: 0,
            known_onchain_overlap_count: 0,
            known_onchain_overlap_value_raw: "0".into(),
            classifications: vec![ClassificationAggregate {
                classification: "goods_or_services".into(),
                operation_count: count,
                value_raw: "18000000000900000".into(),
            }],
            methodology_version: "casp-daily-activity-v1".into(),
            conversion_methodology: "demo-usd-eur-parity-v1".into(),
        }
    }
    #[tokio::test]
    async fn full_quarter_import_makes_threshold_evidence_enforceable() {
        let report = CaspDailyReport {
            from_date_utc: "2026-01-01".into(),
            to_date_utc: "2026-03-31".into(),
            days: vec![day(90_000_090)],
        };
        let store = Arc::new(SqliteCaspReportStore::open(":memory:").unwrap());
        let service = CaspReportingService::new(Arc::new(Source { report }), store.clone());
        service.ingest("2026-01-01", "2026-03-31").await.unwrap();
        let assessment = service.quarterly(2026, 1).unwrap();
        assert!(assessment.complete_source_range);
        assert!(assessment.threshold_breached);
        assert!(assessment.threshold_enforceable);
        assert!(assessment.average_daily_operation_count > 1_000_000.0);
        assert!(store.block_reason().unwrap().is_some());
    }
    #[tokio::test]
    async fn incomplete_range_never_becomes_enforceable() {
        let report = CaspDailyReport {
            from_date_utc: "2026-01-01".into(),
            to_date_utc: "2026-01-01".into(),
            days: vec![day(90_000_090)],
        };
        let service = CaspReportingService::new(
            Arc::new(Source { report }),
            Arc::new(SqliteCaspReportStore::open(":memory:").unwrap()),
        );
        service.ingest("2026-01-01", "2026-01-01").await.unwrap();
        let assessment = service.quarterly(2026, 1).unwrap();
        assert!(assessment.threshold_breached);
        assert!(!assessment.complete_source_range);
        assert!(!assessment.threshold_enforceable);
    }

    #[tokio::test]
    async fn demo_scenario_exceeds_both_thresholds_and_invokes_onchain_control() {
        let controller = Arc::new(Controller(AtomicBool::new(false)));
        let store = Arc::new(SqliteCaspReportStore::open(":memory:").unwrap());
        let service = CaspReportingService::new(
            Arc::new(Source {
                report: CaspDailyReport {
                    from_date_utc: String::new(),
                    to_date_utc: String::new(),
                    days: Vec::new(),
                },
            }),
            store.clone(),
        )
        .with_issuance_controller(controller.clone());

        let assessment = service.run_demo_threshold_breach(2026, 2).await.unwrap();

        assert_eq!(assessment.calendar_day_count, 91);
        assert_eq!(assessment.average_daily_operation_count, 1_000_001.0);
        assert!(assessment.average_daily_value_eur > 200_000_000.0);
        assert!(assessment.threshold_enforceable);
        assert!(controller.0.load(Ordering::SeqCst));
        assert!(store.block_reason().unwrap().is_some());

        let mut recovery_day = day(1);
        recovery_day.date_utc = "2026-04-01".into();
        let recovery = CaspReportingService::new(
            Arc::new(Source {
                report: CaspDailyReport {
                    from_date_utc: "2026-04-01".into(),
                    to_date_utc: "2026-06-30".into(),
                    days: vec![recovery_day],
                },
            }),
            store.clone(),
        )
        .with_issuance_controller(controller.clone());
        recovery.ingest("2026-04-01", "2026-06-30").await.unwrap();

        assert!(!controller.0.load(Ordering::SeqCst));
        assert!(store.block_reason().unwrap().is_none());
    }
}
