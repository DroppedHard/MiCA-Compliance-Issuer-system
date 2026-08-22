use crate::{
    application::{
        CachedTokenQueryService, EsgBroadcaster, EsgStore, ObservationBroadcaster, QueryError,
    },
    domain::{EsgHistory, EsgObservation, TokenObservation},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use serde::Serialize;
use std::sync::Arc;
use std::{convert::Infallible, time::Duration};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

#[derive(Clone)]
struct AppState {
    token_service: Arc<CachedTokenQueryService>,
    observations: ObservationBroadcaster,
    esg_observations: EsgBroadcaster,
    esg_store: Arc<dyn EsgStore>,
}

pub fn router(
    token_service: Arc<CachedTokenQueryService>,
    observations: ObservationBroadcaster,
    esg_observations: EsgBroadcaster,
    esg_store: Arc<dyn EsgStore>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/token", get(token_snapshot))
        .route("/api/v1/token/stream", get(token_stream))
        .route("/api/v1/esg", get(esg_snapshot))
        .route("/api/v1/esg/stream", get(esg_stream))
        .route("/api/v1/esg/daily", get(esg_daily))
        .with_state(AppState {
            token_service,
            observations,
            esg_observations,
            esg_store,
        })
}

async fn esg_daily(State(state): State<AppState>) -> Result<Json<EsgHistory>, ApiError> {
    let current = state.esg_observations.latest().await.ok_or_else(|| {
        ApiError::Unavailable("ESG observer has no successful observation yet".to_owned())
    })?;
    let days = state
        .esg_store
        .recent_estimates(current.chain_id, &current.contract_address, 7)
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(EsgHistory {
        days,
        methodology: current.methodology,
    }))
}

async fn esg_snapshot(State(state): State<AppState>) -> Result<Json<EsgObservation>, ApiError> {
    state
        .esg_observations
        .latest()
        .await
        .map(Json)
        .ok_or_else(|| {
            ApiError::Unavailable("ESG observer has no successful observation yet".to_owned())
        })
}

async fn esg_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.esg_observations.subscribe()).filter_map(|result| {
        result.ok().and_then(|value| {
            serde_json::to_string(&value)
                .ok()
                .map(|json| Ok(Event::default().event("esg").data(json)))
        })
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn token_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.observations.subscribe()).filter_map(|result| {
        result.ok().and_then(|observation| {
            serde_json::to_string(&observation)
                .ok()
                .map(|json| Ok(Event::default().event("token").data(json)))
        })
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    last_success_at_unix_ms: Option<u64>,
    last_error: Option<String>,
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    let status = state.token_service.polling_status().await;
    if !status.is_healthy {
        return Err(ApiError::Unavailable(status.last_error.unwrap_or_else(
            || "poller has no recent successful read".to_owned(),
        )));
    }
    Ok(Json(HealthResponse {
        status: "ok",
        last_success_at_unix_ms: status.last_success_at_unix_ms,
        last_error: None,
    }))
}

async fn token_snapshot(State(state): State<AppState>) -> Result<Json<TokenObservation>, ApiError> {
    state
        .token_service
        .get_latest()
        .await
        .map(Json)
        .map_err(ApiError::from)
}

enum ApiError {
    Unavailable(String),
    Internal(String),
}
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unavailable(message) => (StatusCode::SERVICE_UNAVAILABLE, message),
            Self::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<QueryError> for ApiError {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::PollingUnavailable(message) => Self::Unavailable(message),
            QueryError::CacheEmpty => Self::Unavailable(error.to_string()),
            QueryError::Cache(message) => Self::Internal(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{PollingMonitor, SnapshotCache},
        config::esg,
        domain::EsgObservation,
        infrastructure::cache::InMemorySnapshotCache,
    };
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    fn test_router(esg: EsgBroadcaster) -> Router {
        let store: Arc<dyn EsgStore> =
            Arc::new(crate::infrastructure::sqlite::SqliteEsgStore::open(":memory:").unwrap());
        test_router_with_store(esg, store)
    }

    fn test_router_with_store(esg: EsgBroadcaster, store: Arc<dyn EsgStore>) -> Router {
        let cache: Arc<dyn SnapshotCache> =
            Arc::new(InMemorySnapshotCache::new(Duration::from_secs(60)));
        let service = Arc::new(CachedTokenQueryService::new(
            cache,
            Arc::new(PollingMonitor::new(Duration::from_secs(30))),
        ));
        router(service, ObservationBroadcaster::new(4), esg, store)
    }

    #[tokio::test]
    async fn esg_endpoint_is_unavailable_before_first_observation() {
        let response = test_router(EsgBroadcaster::new(4))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/esg")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn esg_endpoint_serializes_the_latest_contract() {
        let broadcaster = EsgBroadcaster::new(4);
        broadcaster
            .publish(EsgObservation {
                observed_at_unix_ms: 123,
                last_processed_block: 42,
                chain_id: 1,
                contract_address: "0xabc".to_owned(),
                current_day: esg::estimate("2026-08-22".to_owned(), 2, "provisional"),
                methodology: esg::methodology(),
            })
            .await;

        let response = test_router(broadcaster)
            .oneshot(
                Request::builder()
                    .uri("/api/v1/esg")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["currentDay"]["transactionCount"], 2);
        assert_eq!(json["currentDay"]["energyBestGuessWh"], 39.35);
        assert_eq!(json["currentDay"]["energyLowerWh"], 6.3);
        assert_eq!(json["currentDay"]["energyUpperWh"], 57.45);
        assert_eq!(json["methodology"]["version"], esg::METHODOLOGY_VERSION);
        assert_eq!(json["lastProcessedBlock"], 42);
    }

    #[tokio::test]
    async fn daily_endpoint_returns_ordered_scenario_history_for_the_current_token_only() {
        let concrete =
            Arc::new(crate::infrastructure::sqlite::SqliteEsgStore::open(":memory:").unwrap());
        concrete
            .seed_demo_day(1, "0xabc", "2026-08-20", 10)
            .unwrap();
        concrete
            .seed_demo_day(1, "0xabc", "2026-08-21", 20)
            .unwrap();
        concrete
            .seed_demo_day(1, "0xother", "2026-08-21", 999)
            .unwrap();
        let store: Arc<dyn EsgStore> = concrete;
        let broadcaster = EsgBroadcaster::new(4);
        broadcaster
            .publish(EsgObservation {
                observed_at_unix_ms: 123,
                last_processed_block: 42,
                chain_id: 1,
                contract_address: "0xabc".to_owned(),
                current_day: esg::estimate("2026-08-22".to_owned(), 0, "provisional"),
                methodology: esg::methodology(),
            })
            .await;

        let response = test_router_with_store(broadcaster, store)
            .oneshot(
                Request::builder()
                    .uri("/api/v1/esg/daily")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["days"].as_array().unwrap().len(), 2);
        assert_eq!(json["days"][0]["dateUtc"], "2026-08-20");
        assert_eq!(json["days"][1]["transactionCount"], 20);
        assert_eq!(json["days"][1]["energyLowerWh"], 63.0);
        assert_eq!(json["days"][1]["energyBestGuessWh"], 393.5);
        assert_eq!(json["days"][1]["energyUpperWh"], 574.5);
    }
}
