use crate::{
    api::{
        requests::{CaspRangeQuery, QuarterQuery},
        responses::ApiError,
        state::AppState,
        validators::ValidatedQuery,
    },
    domain::{CaspDailyAggregate, CaspDailyReport, QuarterlyTransactionAssessment},
};
use axum::{Json, extract::State};
pub(crate) async fn ingest_casp_reports(
    State(s): State<AppState>,
    ValidatedQuery(q): ValidatedQuery<CaspRangeQuery>,
) -> Result<Json<CaspDailyReport>, ApiError> {
    s.casp_reporting_service
        .ingest(&q.from, &q.to)
        .await
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn casp_daily_reports(
    State(s): State<AppState>,
    ValidatedQuery(q): ValidatedQuery<CaspRangeQuery>,
) -> Result<Json<Vec<CaspDailyAggregate>>, ApiError> {
    s.casp_reporting_service
        .daily(&q.from, &q.to)
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn casp_quarterly_report(
    State(s): State<AppState>,
    ValidatedQuery(q): ValidatedQuery<QuarterQuery>,
) -> Result<Json<QuarterlyTransactionAssessment>, ApiError> {
    s.casp_reporting_service
        .quarterly(q.year, q.quarter)
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn run_demo_casp_threshold_breach(
    State(s): State<AppState>,
    ValidatedQuery(q): ValidatedQuery<QuarterQuery>,
) -> Result<Json<QuarterlyTransactionAssessment>, ApiError> {
    s.casp_reporting_service
        .run_demo_threshold_breach(q.year, q.quarter)
        .await
        .map(Json)
        .map_err(Into::into)
}
