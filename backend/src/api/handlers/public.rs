use crate::{
    api::{responses::ApiError, state::AppState},
    domain::{AssetState, EsgHistory, EsgObservation, ReserveCoverage, TokenObservation},
};
use axum::{
    Json,
    extract::State,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use serde::Serialize;
use std::{convert::Infallible, time::Duration};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
pub(crate) async fn asset_state(State(s): State<AppState>) -> Result<Json<AssetState>, ApiError> {
    s.asset_state_service
        .current()
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}
pub(crate) async fn asset_state_stream(
    State(s): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    sse(
        BroadcastStream::new(s.asset_state_service.subscribe()),
        "asset-state",
    )
}
pub(crate) async fn reserve_coverage(
    State(s): State<AppState>,
) -> Result<Json<ReserveCoverage>, ApiError> {
    s.reserve_monitor
        .latest()
        .await
        .map(Json)
        .map_err(|e| ApiError::Unavailable(e.to_string()))
}
pub(crate) async fn reserve_stream(
    State(s): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    sse(
        BroadcastStream::new(s.reserve_monitor.subscribe()),
        "reserve",
    )
}
pub(crate) async fn esg_daily(State(s): State<AppState>) -> Result<Json<EsgHistory>, ApiError> {
    let current = s.esg_observations.latest().await.ok_or_else(|| {
        ApiError::Unavailable("ESG observer has no successful observation yet".into())
    })?;
    let days = s
        .esg_store
        .recent_estimates(current.chain_id, &current.contract_address, 7)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(EsgHistory {
        days,
        methodology: current.methodology,
    }))
}
pub(crate) async fn esg_snapshot(
    State(s): State<AppState>,
) -> Result<Json<EsgObservation>, ApiError> {
    s.esg_observations.latest().await.map(Json).ok_or_else(|| {
        ApiError::Unavailable("ESG observer has no successful observation yet".into())
    })
}
pub(crate) async fn esg_stream(
    State(s): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    sse(BroadcastStream::new(s.esg_observations.subscribe()), "esg")
}
pub(crate) async fn token_stream(
    State(s): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    sse(BroadcastStream::new(s.observations.subscribe()), "token")
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HealthResponse {
    status: &'static str,
    last_success_at_unix_ms: Option<u64>,
    last_error: Option<String>,
}
pub(crate) async fn health(State(s): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    let status = s.token_service.polling_status().await;
    if !status.is_healthy {
        return Err(ApiError::Unavailable(
            status
                .last_error
                .unwrap_or_else(|| "poller has no recent successful read".into()),
        ));
    }
    Ok(Json(HealthResponse {
        status: "ok",
        last_success_at_unix_ms: status.last_success_at_unix_ms,
        last_error: None,
    }))
}
pub(crate) async fn token_snapshot(
    State(s): State<AppState>,
) -> Result<Json<TokenObservation>, ApiError> {
    s.token_service
        .get_latest()
        .await
        .map(Json)
        .map_err(Into::into)
}
fn sse<T: Serialize + Clone + Send + 'static>(
    stream: BroadcastStream<T>,
    event: &'static str,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = stream.filter_map(move |r| {
        r.ok().and_then(|v| {
            serde_json::to_string(&v)
                .ok()
                .map(|json| Ok(Event::default().event(event).data(json)))
        })
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
