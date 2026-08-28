use crate::{
    application::{
        AdjustReserve, AssetStateService, CachedTokenQueryService, CaspReportingError,
        CaspReportingService, CreateIssuance, EsgBroadcaster, EsgStore, IssuanceError,
        IssuanceService, ObservationBroadcaster, QueryError, RedemptionError, RedemptionService,
        ReserveAdjustmentDirection, ReserveAdjustmentError, ReserveAdjustmentService,
        ReserveMonitor, WindDownError, WindDownService,
    },
    domain::{
        AssetState, CaspDailyAggregate, CaspDailyReport, EsgHistory, EsgObservation,
        QuarterlyTransactionAssessment, ReserveCoverage, TokenObservation,
    },
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{convert::Infallible, time::Duration};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

#[derive(Clone)]
struct AppState {
    token_service: Arc<CachedTokenQueryService>,
    observations: ObservationBroadcaster,
    esg_observations: EsgBroadcaster,
    esg_store: Arc<dyn EsgStore>,
    reserve_monitor: ReserveMonitor,
    reserve_adjustment_service: Arc<ReserveAdjustmentService>,
    asset_state_service: Arc<AssetStateService>,
    issuance_service: Arc<IssuanceService>,
    redemption_service: Arc<RedemptionService>,
    wind_down_service: Arc<WindDownService>,
    casp_reporting_service: Arc<CaspReportingService>,
}

pub struct RouterDependencies {
    pub token_service: Arc<CachedTokenQueryService>,
    pub observations: ObservationBroadcaster,
    pub esg_observations: EsgBroadcaster,
    pub esg_store: Arc<dyn EsgStore>,
    pub reserve_monitor: ReserveMonitor,
    pub reserve_adjustment_service: Arc<ReserveAdjustmentService>,
    pub asset_state_service: Arc<AssetStateService>,
    pub issuance_service: Arc<IssuanceService>,
    pub redemption_service: Arc<RedemptionService>,
    pub wind_down_service: Arc<WindDownService>,
    pub casp_reporting_service: Arc<CaspReportingService>,
}

pub fn router(dependencies: RouterDependencies) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/token", get(token_snapshot))
        .route("/api/v1/token/stream", get(token_stream))
        .route("/api/v1/esg", get(esg_snapshot))
        .route("/api/v1/esg/stream", get(esg_stream))
        .route("/api/v1/esg/daily", get(esg_daily))
        .route("/api/v1/reserves", get(reserve_coverage))
        .route("/api/v1/reserves/stream", get(reserve_stream))
        .route("/api/v1/admin/reserves/adjustments", post(adjust_reserve))
        .route("/api/v1/asset-state", get(asset_state))
        .route("/api/v1/asset-state/stream", get(asset_state_stream))
        .route("/api/v1/admin/asset-state/wind-down", post(enter_wind_down))
        .route(
            "/api/v1/admin/casp-reports/ingest",
            post(ingest_casp_reports),
        )
        .route("/api/v1/admin/casp-reports/daily", get(casp_daily_reports))
        .route(
            "/api/v1/admin/casp-reports/quarterly",
            get(casp_quarterly_report),
        )
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
        .with_state(AppState {
            token_service: dependencies.token_service,
            observations: dependencies.observations,
            esg_observations: dependencies.esg_observations,
            esg_store: dependencies.esg_store,
            reserve_monitor: dependencies.reserve_monitor,
            reserve_adjustment_service: dependencies.reserve_adjustment_service,
            asset_state_service: dependencies.asset_state_service,
            issuance_service: dependencies.issuance_service,
            redemption_service: dependencies.redemption_service,
            wind_down_service: dependencies.wind_down_service,
            casp_reporting_service: dependencies.casp_reporting_service,
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReserveAdjustmentRequest {
    operation_id: String,
    direction: ReserveAdjustmentRequestDirection,
    amount_usd: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReserveAdjustmentRequestDirection {
    Deposit,
    Withdrawal,
}

async fn adjust_reserve(
    State(state): State<AppState>,
    Json(request): Json<ReserveAdjustmentRequest>,
) -> Result<Json<crate::domain::BankReserve>, ApiError> {
    let direction = match request.direction {
        ReserveAdjustmentRequestDirection::Deposit => ReserveAdjustmentDirection::Deposit,
        ReserveAdjustmentRequestDirection::Withdrawal => ReserveAdjustmentDirection::Withdrawal,
    };
    state
        .reserve_adjustment_service
        .execute(AdjustReserve {
            operation_id: request.operation_id,
            direction,
            amount_usd: request.amount_usd,
            reason: request.reason,
        })
        .await
        .map(Json)
        .map_err(ApiError::from)
}
#[derive(Deserialize)]
struct CaspRangeQuery {
    from: String,
    to: String,
}
#[derive(Deserialize)]
struct QuarterQuery {
    year: i32,
    quarter: u8,
}
async fn ingest_casp_reports(
    State(state): State<AppState>,
    Query(query): Query<CaspRangeQuery>,
) -> Result<Json<CaspDailyReport>, ApiError> {
    state
        .casp_reporting_service
        .ingest(&query.from, &query.to)
        .await
        .map(Json)
        .map_err(ApiError::from)
}
async fn casp_daily_reports(
    State(state): State<AppState>,
    Query(query): Query<CaspRangeQuery>,
) -> Result<Json<Vec<CaspDailyAggregate>>, ApiError> {
    state
        .casp_reporting_service
        .daily(&query.from, &query.to)
        .map(Json)
        .map_err(ApiError::from)
}
async fn casp_quarterly_report(
    State(state): State<AppState>,
    Query(query): Query<QuarterQuery>,
) -> Result<Json<QuarterlyTransactionAssessment>, ApiError> {
    state
        .casp_reporting_service
        .quarterly(query.year, query.quarter)
        .map(Json)
        .map_err(ApiError::from)
}

async fn asset_state(State(state): State<AppState>) -> Result<Json<AssetState>, ApiError> {
    state
        .asset_state_service
        .current()
        .map(Json)
        .map_err(|error| ApiError::Internal(error.to_string()))
}

async fn asset_state_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.asset_state_service.subscribe()).filter_map(|result| {
        result.ok().and_then(|value| {
            serde_json::to_string(&value)
                .ok()
                .map(|json| Ok(Event::default().event("asset-state").data(json)))
        })
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindDownRequest {
    operation_id: String,
    reason: String,
}

async fn enter_wind_down(
    State(state): State<AppState>,
    Json(request): Json<WindDownRequest>,
) -> Result<Json<AssetState>, ApiError> {
    state
        .wind_down_service
        .enter(&request.operation_id, &request.reason)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRedemptionRequest {
    operation_id: String,
    holder_address: String,
    token_amount_raw: String,
}
async fn create_redemption(
    State(s): State<AppState>,
    Json(r): Json<CreateRedemptionRequest>,
) -> Result<(StatusCode, Json<crate::domain::RedemptionOrder>), ApiError> {
    let o = s
        .redemption_service
        .create(r.operation_id, r.holder_address, r.token_amount_raw)
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(o)))
}
async fn get_redemption(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::domain::RedemptionOrder>, ApiError> {
    s.redemption_service
        .get(&id)
        .map(Json)
        .map_err(ApiError::from)
}
async fn settle_redemption(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::domain::RedemptionOrder>, ApiError> {
    s.redemption_service
        .settle(&id)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateIssuanceRequest {
    operation_id: String,
    recipient_address: String,
    amount_usd_minor: String,
}
async fn create_issuance(
    State(state): State<AppState>,
    Json(request): Json<CreateIssuanceRequest>,
) -> Result<(StatusCode, Json<crate::domain::IssuanceOrder>), ApiError> {
    let order = state
        .issuance_service
        .create(CreateIssuance {
            operation_id: request.operation_id,
            recipient_address: request.recipient_address,
            amount_usd_minor: request.amount_usd_minor,
        })
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(order)))
}
async fn get_issuance(
    State(state): State<AppState>,
    Path(operation_id): Path<String>,
) -> Result<Json<crate::domain::IssuanceOrder>, ApiError> {
    state
        .issuance_service
        .get(&operation_id)
        .map(Json)
        .map_err(ApiError::from)
}
async fn settle_issuance(
    State(state): State<AppState>,
    Path(operation_id): Path<String>,
) -> Result<Json<crate::domain::IssuanceOrder>, ApiError> {
    state
        .issuance_service
        .settle(&operation_id)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn reserve_coverage(
    State(state): State<AppState>,
) -> Result<Json<ReserveCoverage>, ApiError> {
    state
        .reserve_monitor
        .latest()
        .await
        .map(Json)
        .map_err(|error| ApiError::Unavailable(error.to_string()))
}
async fn reserve_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.reserve_monitor.subscribe()).filter_map(|result| {
        result.ok().and_then(|value| {
            serde_json::to_string(&value)
                .ok()
                .map(|json| Ok(Event::default().event("reserve").data(json)))
        })
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
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
    BadRequest(String),
    NotFound(String),
    Conflict(String),
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
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::Unavailable(message) => (StatusCode::SERVICE_UNAVAILABLE, message),
            Self::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<IssuanceError> for ApiError {
    fn from(error: IssuanceError) -> Self {
        match error {
            IssuanceError::Invalid(_) => Self::BadRequest(error.to_string()),
            IssuanceError::NotFound => Self::NotFound(error.to_string()),
            IssuanceError::IdempotencyConflict
            | IssuanceError::FiatNotConfirmed
            | IssuanceError::BankMismatch
            | IssuanceError::SettlementInProgress
            | IssuanceError::IssuanceBlocked(_) => Self::Conflict(error.to_string()),
            IssuanceError::Bank(_) => Self::Unavailable(error.to_string()),
            IssuanceError::Storage(_) | IssuanceError::Blockchain(_) => {
                Self::Internal(error.to_string())
            }
        }
    }
}
impl From<RedemptionError> for ApiError {
    fn from(e: RedemptionError) -> Self {
        match e {
            RedemptionError::Invalid(_) => Self::BadRequest(e.to_string()),
            RedemptionError::NotFound => Self::NotFound(e.to_string()),
            RedemptionError::IdempotencyConflict => Self::Conflict(e.to_string()),
            RedemptionError::Storage(_)
            | RedemptionError::Blockchain(_)
            | RedemptionError::Bank(_)
            | RedemptionError::Gate(_) => Self::Internal(e.to_string()),
        }
    }
}

impl From<WindDownError> for ApiError {
    fn from(error: WindDownError) -> Self {
        match error {
            WindDownError::Invalid(_) => Self::BadRequest(error.to_string()),
            WindDownError::IdempotencyConflict => Self::Conflict(error.to_string()),
            WindDownError::Blockchain(_) => Self::Unavailable(error.to_string()),
            WindDownError::State(_) | WindDownError::Storage(_) => {
                Self::Internal(error.to_string())
            }
        }
    }
}

impl From<ReserveAdjustmentError> for ApiError {
    fn from(error: ReserveAdjustmentError) -> Self {
        match error {
            ReserveAdjustmentError::Invalid(_) => Self::BadRequest(error.to_string()),
            ReserveAdjustmentError::Bank(_) => Self::Unavailable(error.to_string()),
        }
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
impl From<CaspReportingError> for ApiError {
    fn from(error: CaspReportingError) -> Self {
        match error {
            CaspReportingError::InvalidRange | CaspReportingError::SourceContract(_) => {
                Self::BadRequest(error.to_string())
            }
            CaspReportingError::Source(_) => Self::Unavailable(error.to_string()),
            CaspReportingError::Storage(_) | CaspReportingError::Overflow => {
                Self::Internal(error.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            BankTransactionReader, ConfirmedBankTransaction, IssuanceStore, MintResult,
            OperationGate, PayoutBank, PollingMonitor, RedemptionToken, ReserveAdjustmentGateway,
            SnapshotCache, TokenIssuer, TokenLifecycle,
        },
        config::esg,
        domain::{EsgObservation, IssuanceOrder},
        infrastructure::cache::InMemorySnapshotCache,
    };
    use alloy::primitives::Address;
    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use std::sync::Mutex;
    use tower::ServiceExt;

    struct TestIssuanceStore(Mutex<Option<IssuanceOrder>>);
    struct TestReserveAdjustment;
    #[async_trait]
    impl ReserveAdjustmentGateway for TestReserveAdjustment {
        async fn adjust(
            &self,
            _: &str,
            _: ReserveAdjustmentDirection,
            _: u64,
            _: &str,
        ) -> Result<crate::domain::BankReserve, ReserveAdjustmentError> {
            Ok(crate::domain::BankReserve {
                account_id: "reserve-rusd".into(),
                currency: "USD".into(),
                balance_minor: "100".into(),
                version: 1,
                as_of_unix_ms: 1,
            })
        }
    }
    impl IssuanceStore for TestIssuanceStore {
        fn create(&self, order: &IssuanceOrder) -> Result<IssuanceOrder, IssuanceError> {
            let mut value = self.0.lock().unwrap();
            if let Some(existing) = value.as_ref() {
                return Ok(existing.clone());
            }
            *value = Some(order.clone());
            Ok(order.clone())
        }
        fn get(&self, _: &str) -> Result<Option<IssuanceOrder>, IssuanceError> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn claim_for_mint(&self, _: &str) -> Result<IssuanceOrder, IssuanceError> {
            Err(IssuanceError::SettlementInProgress)
        }
        fn complete(&self, _: &str, _: Option<&str>) -> Result<IssuanceOrder, IssuanceError> {
            Err(IssuanceError::NotFound)
        }
        fn fail(&self, _: &str, _: &str) -> Result<(), IssuanceError> {
            Ok(())
        }
    }
    fn operation_gate() -> Arc<OperationGate> {
        Arc::new(OperationGate::new(Arc::new(
            crate::infrastructure::operation_decision_sqlite::SqliteOperationDecisionStore::open(
                ":memory:",
            )
            .unwrap(),
        )))
    }
    struct TestBank;
    struct TestCaspSource;
    #[async_trait]
    impl crate::application::CaspReportSource for TestCaspSource {
        async fn fetch(
            &self,
            from: &str,
            to: &str,
        ) -> Result<crate::domain::CaspDailyReport, CaspReportingError> {
            Ok(crate::domain::CaspDailyReport {
                from_date_utc: from.into(),
                to_date_utc: to.into(),
                days: Vec::new(),
            })
        }
    }
    #[async_trait]
    impl BankTransactionReader for TestBank {
        async fn find(&self, _: &str) -> Result<Option<ConfirmedBankTransaction>, IssuanceError> {
            Ok(None)
        }
    }
    struct TestToken;
    #[async_trait]
    impl TokenIssuer for TestToken {
        async fn mint_for_operation(
            &self,
            _: &str,
            _: Address,
            _: u64,
        ) -> Result<MintResult, IssuanceError> {
            Err(IssuanceError::Blockchain("not used".to_owned()))
        }
    }
    #[async_trait]
    impl RedemptionToken for TestToken {
        async fn burn_for_operation(
            &self,
            _: &str,
            _: Address,
            _: u64,
        ) -> Result<Option<String>, RedemptionError> {
            Ok(Some("0xburn".into()))
        }
    }
    #[async_trait]
    impl TokenLifecycle for TestToken {
        async fn enter_wind_down(&self) -> Result<Option<String>, WindDownError> {
            Ok(Some("0xwinddown".into()))
        }
    }
    #[async_trait]
    impl PayoutBank for TestBank {
        async fn pay_usd(&self, _: &str, _: u64) -> Result<(), RedemptionError> {
            Ok(())
        }
    }
    fn issuance_service() -> Arc<IssuanceService> {
        Arc::new(IssuanceService::new(
            Arc::new(TestIssuanceStore(Mutex::new(None))),
            Arc::new(TestBank),
            Arc::new(TestToken),
            Arc::new(AssetStateService::new(
                Arc::new(
                    crate::infrastructure::asset_state_sqlite::SqliteAssetStateStore::open(
                        ":memory:",
                    )
                    .unwrap(),
                ),
                4,
            )),
            operation_gate(),
        ))
    }
    fn redemption_service() -> Arc<RedemptionService> {
        Arc::new(RedemptionService::new(
            Arc::new(
                crate::infrastructure::redemption_sqlite::SqliteRedemptionStore::open(":memory:")
                    .unwrap(),
            ),
            Arc::new(TestToken),
            Arc::new(TestBank),
            Arc::new(AssetStateService::new(
                Arc::new(
                    crate::infrastructure::asset_state_sqlite::SqliteAssetStateStore::open(
                        ":memory:",
                    )
                    .unwrap(),
                ),
                4,
            )),
            operation_gate(),
        ))
    }
    fn wind_down_service() -> Arc<WindDownService> {
        Arc::new(WindDownService::new(
            Arc::new(AssetStateService::new(
                Arc::new(
                    crate::infrastructure::asset_state_sqlite::SqliteAssetStateStore::open(
                        ":memory:",
                    )
                    .unwrap(),
                ),
                4,
            )),
            Arc::new(TestToken),
            Arc::new(
                crate::infrastructure::wind_down_sqlite::SqliteWindDownAuditStore::open(":memory:")
                    .unwrap(),
            ),
        ))
    }

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
        router(RouterDependencies {
            token_service: service,
            observations: ObservationBroadcaster::new(4),
            esg_observations: esg,
            esg_store: store,
            reserve_monitor: ReserveMonitor::new(4),
            reserve_adjustment_service: Arc::new(ReserveAdjustmentService::new(Arc::new(
                TestReserveAdjustment,
            ))),
            asset_state_service: Arc::new(AssetStateService::new(
                Arc::new(
                    crate::infrastructure::asset_state_sqlite::SqliteAssetStateStore::open(
                        ":memory:",
                    )
                    .unwrap(),
                ),
                4,
            )),
            issuance_service: issuance_service(),
            redemption_service: redemption_service(),
            wind_down_service: wind_down_service(),
            casp_reporting_service: Arc::new(CaspReportingService::new(
                Arc::new(TestCaspSource),
                Arc::new(
                    crate::infrastructure::casp_reporting_sqlite::SqliteCaspReportStore::open(
                        ":memory:",
                    )
                    .unwrap(),
                ),
            )),
        })
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
    async fn asset_state_endpoint_returns_the_persisted_backend_decision() {
        let response = test_router(EsgBroadcaster::new(4))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/asset-state")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["state"], "data_unavailable");
        assert_eq!(json["policyVersion"], "reserve-coverage-v1");
    }

    #[tokio::test]
    async fn wind_down_endpoint_waits_for_executor_and_returns_terminal_state() {
        let response = test_router(EsgBroadcaster::new(4))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/asset-state/wind-down")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"operationId":"wind-api-1","reason":"authority decision"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: AssetState = serde_json::from_slice(&body).unwrap();
        assert_eq!(value.state, crate::domain::AssetStateCode::WindDown);
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

    #[tokio::test]
    async fn issuance_endpoint_is_idempotent_and_returns_bank_instructions() {
        let app = test_router(EsgBroadcaster::new(4));
        let body = r#"{"operationId":"purchase-1","recipientAddress":"0x0000000000000000000000000000000000000001","amountUsdMinor":"1250"}"#;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/issuance-orders")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(json["status"], "awaiting_fiat");
        assert_eq!(json["tokenAmountRaw"], "12500000");
        assert_eq!(json["bankIdempotencyKey"], "issuance-purchase-1");
    }
}
