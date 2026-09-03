use crate::{
    api::{
        requests::{CreateIssuanceRequest, CreateRedemptionRequest},
        responses::ApiError,
        state::AppState,
        validators::ValidatedJson,
    },
    application::CreateIssuance,
    domain::{IssuanceOrder, RedemptionOrder},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
pub(crate) async fn create_issuance(
    State(s): State<AppState>,
    ValidatedJson(r): ValidatedJson<CreateIssuanceRequest>,
) -> Result<(StatusCode, Json<IssuanceOrder>), ApiError> {
    let o = s.issuance_service.create(CreateIssuance {
        operation_id: r.operation_id.into(),
        recipient_address: r.recipient_address.into(),
        amount_usd_minor: r.amount_usd_minor,
    })?;
    Ok((StatusCode::CREATED, Json(o)))
}
pub(crate) async fn get_issuance(
    State(s): State<AppState>,
    Path(id): Path<crate::api::validators::text::OperationId>,
) -> Result<Json<IssuanceOrder>, ApiError> {
    s.issuance_service
        .get(id.as_ref())
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn settle_issuance(
    State(s): State<AppState>,
    Path(id): Path<crate::api::validators::text::OperationId>,
) -> Result<Json<IssuanceOrder>, ApiError> {
    s.issuance_service
        .settle(id.as_ref())
        .await
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn create_redemption(
    State(s): State<AppState>,
    ValidatedJson(r): ValidatedJson<CreateRedemptionRequest>,
) -> Result<(StatusCode, Json<RedemptionOrder>), ApiError> {
    let o = s.redemption_service.create(
        r.operation_id.into(),
        r.holder_address.into(),
        r.token_amount_raw,
    )?;
    Ok((StatusCode::CREATED, Json(o)))
}
pub(crate) async fn get_redemption(
    State(s): State<AppState>,
    Path(id): Path<crate::api::validators::text::OperationId>,
) -> Result<Json<RedemptionOrder>, ApiError> {
    s.redemption_service
        .get(id.as_ref())
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn settle_redemption(
    State(s): State<AppState>,
    Path(id): Path<crate::api::validators::text::OperationId>,
) -> Result<Json<RedemptionOrder>, ApiError> {
    s.redemption_service
        .settle(id.as_ref())
        .await
        .map(Json)
        .map_err(Into::into)
}
