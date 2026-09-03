use crate::api::{handlers::*, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/casp-reports/ingest",
            post(ingest_casp_reports),
        )
        .route("/api/v1/admin/casp-reports/daily", get(casp_daily_reports))
        .route(
            "/api/v1/admin/casp-reports/quarterly",
            get(casp_quarterly_report),
        )
        .route(
            "/api/v1/admin/demo/casp-threshold-breach",
            post(run_demo_casp_threshold_breach),
        )
}
