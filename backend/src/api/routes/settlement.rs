use crate::api::{handlers::*, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/issuance-orders", post(create_issuance))
        .route("/api/v1/issuance-orders/{operation_id}", get(get_issuance))
        .route(
            "/api/v1/issuance-orders/{operation_id}/settle",
            post(settle_issuance),
        )
        .route("/api/v1/redemption-orders", post(create_redemption))
        .route(
            "/api/v1/redemption-orders/{operation_id}",
            get(get_redemption),
        )
        .route(
            "/api/v1/redemption-orders/{operation_id}/settle",
            post(settle_redemption),
        )
}
