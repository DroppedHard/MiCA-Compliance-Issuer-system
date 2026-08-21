use crate::{application::TokenQueryService, domain::TokenSnapshot};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    token_service: Arc<TokenQueryService>,
}

pub fn router(token_service: Arc<TokenQueryService>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/token", get(token_snapshot))
        .with_state(AppState { token_service })
}

async fn health() -> &'static str {
    "ok"
}

async fn token_snapshot(State(state): State<AppState>) -> Result<Json<TokenSnapshot>, ApiError> {
    state
        .token_service
        .get_snapshot()
        .await
        .map(Json)
        .map_err(|error| ApiError::Upstream(error.to_string()))
}

enum ApiError {
    Upstream(String),
}
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let Self::Upstream(message) = self;
        (StatusCode::BAD_GATEWAY, Json(ErrorBody { error: message })).into_response()
    }
}
