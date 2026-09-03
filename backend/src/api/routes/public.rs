use crate::api::{handlers::*, state::AppState};
use axum::{Router, routing::get};
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/token", get(token_snapshot))
        .route("/api/v1/token/stream", get(token_stream))
        .route("/api/v1/esg", get(esg_snapshot))
        .route("/api/v1/esg/stream", get(esg_stream))
        .route("/api/v1/esg/daily", get(esg_daily))
        .route("/api/v1/reserves", get(reserve_coverage))
        .route("/api/v1/reserves/stream", get(reserve_stream))
        .route("/api/v1/asset-state", get(asset_state))
        .route("/api/v1/asset-state/stream", get(asset_state_stream))
}
