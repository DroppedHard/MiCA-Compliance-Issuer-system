//! Integracyjny test raportu aktywności CASP i bramki emisji emitenta.

use async_trait::async_trait;
use crypto_asset_backend::{
    application::{
        CaspReportSource, CaspReportStore, CaspReportingError, CaspReportingService, OperationGate,
    },
    domain::{
        AssetState, AssetStateCode, CaspDailyAggregate, CaspDailyReport, ClassificationAggregate,
        IssuerOperationKind, OperationDecisionOutcome,
    },
    infrastructure::{
        casp_reporting_sqlite::SqliteCaspReportStore,
        operation_decision_sqlite::SqliteOperationDecisionStore,
    },
};
use std::sync::Arc;

struct UnusedSource;

#[async_trait]
impl CaspReportSource for UnusedSource {
    async fn fetch(&self, _: &str, _: &str) -> Result<CaspDailyReport, CaspReportingError> {
        unreachable!("syntetyczny scenariusz progowy nie pobiera raportu przez HTTP")
    }
}

struct StaticSource(CaspDailyReport);

#[async_trait]
impl CaspReportSource for StaticSource {
    async fn fetch(&self, _: &str, _: &str) -> Result<CaspDailyReport, CaspReportingError> {
        Ok(self.0.clone())
    }
}

fn active_reserves() -> AssetState {
    AssetState {
        state: AssetStateCode::Active,
        reason: "pokrycie rezerw powyżej marginesu bezpieczeństwa".into(),
        reserve_coverage_percent: Some(110.0),
        evidence_at_unix_ms: Some(1),
        policy_version: "reserve-coverage-v1".into(),
        updated_at_unix_ms: 1,
    }
}

#[tokio::test]
async fn enforceable_quarterly_activity_blocks_issuance_but_keeps_redemption_available() {
    let report_store = Arc::new(SqliteCaspReportStore::open(":memory:").unwrap());
    let reporting = CaspReportingService::new(Arc::new(UnusedSource), report_store.clone());

    let assessment = reporting.run_demo_threshold_breach(2026, 2).await.unwrap();

    assert!(assessment.complete_source_range);
    assert!(assessment.threshold_breached);
    assert!(assessment.threshold_enforceable);
    let gate = OperationGate::with_issuance_restriction(
        Arc::new(SqliteOperationDecisionStore::open(":memory:").unwrap()),
        report_store,
    );
    let active_reserves = active_reserves();

    let issuance = gate
        .decide(
            "activity-blocked-issuance",
            IssuerOperationKind::Issuance,
            &active_reserves,
        )
        .unwrap();
    let redemption = gate
        .decide(
            "activity-allowed-redemption",
            IssuerOperationKind::Redemption,
            &active_reserves,
        )
        .unwrap();

    assert_eq!(issuance.outcome, OperationDecisionOutcome::Rejected);
    assert!(issuance.reason.contains("Article 23"));
    assert_eq!(redemption.outcome, OperationDecisionOutcome::Allowed);
}

#[tokio::test]
async fn incomplete_casp_range_never_blocks_issuance_even_when_its_activity_is_above_threshold() {
    let report_store = Arc::new(SqliteCaspReportStore::open(":memory:").unwrap());
    let high_activity = CaspDailyAggregate {
        date_utc: "2026-01-01".into(),
        asset_symbol: "rUSD".into(),
        currency_area: "USD".into(),
        total_operation_count: 90_000_090,
        total_value_raw: "18000000000900000".into(),
        total_value_usd_minor: "1800000000090".into(),
        means_of_exchange_count: 90_000_090,
        means_of_exchange_value_raw: "18000000000900000".into(),
        means_of_exchange_value_usd_minor: "1800000000090".into(),
        means_of_exchange_value_eur_minor: "1800000000090".into(),
        excluded_operation_count: 0,
        known_onchain_overlap_count: 0,
        known_onchain_overlap_value_raw: "0".into(),
        classifications: vec![ClassificationAggregate {
            classification: "goods_or_services".into(),
            operation_count: 90_000_090,
            value_raw: "18000000000900000".into(),
        }],
        methodology_version: "casp-daily-activity-v1".into(),
        conversion_methodology: "demo-usd-eur-parity-v1".into(),
    };
    let reporting = CaspReportingService::new(
        Arc::new(StaticSource(CaspDailyReport {
            from_date_utc: "2026-01-01".into(),
            to_date_utc: "2026-01-01".into(),
            days: vec![high_activity],
        })),
        report_store.clone(),
    );

    reporting.ingest("2026-01-01", "2026-01-01").await.unwrap();
    let assessment = reporting.quarterly(2026, 1).unwrap();
    report_store.save_assessment(&assessment).unwrap();

    assert!(assessment.threshold_breached);
    assert!(!assessment.complete_source_range);
    assert!(!assessment.threshold_enforceable);
    let gate = OperationGate::with_issuance_restriction(
        Arc::new(SqliteOperationDecisionStore::open(":memory:").unwrap()),
        report_store,
    );
    let decision = gate
        .decide(
            "activity-incomplete-issuance",
            IssuerOperationKind::Issuance,
            &active_reserves(),
        )
        .unwrap();

    assert_eq!(decision.outcome, OperationDecisionOutcome::Allowed);
}
