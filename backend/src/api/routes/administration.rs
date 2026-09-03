use crate::api::{handlers::*, state::AppState};
use axum::{
    Router,
    routing::{delete, get, post},
};
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/reserves/adjustments", post(adjust_reserve))
        .route("/api/v1/admin/asset-state/wind-down", post(enter_wind_down))
        .route(
            "/api/v1/admin/address-blacklist",
            get(list_address_restrictions).post(block_address),
        )
        .route(
            "/api/v1/admin/address-blacklist/{address}",
            delete(unblock_address),
        )
}
