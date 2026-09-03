use crate::{
    api::{
        requests::{
            AddressRestrictionRequest, ReserveAdjustmentRequest, ReserveAdjustmentRequestDirection,
            WindDownRequest,
        },
        responses::ApiError,
        state::AppState,
        validators::ValidatedJson,
    },
    application::{AddressRestriction, AdjustReserve, ReserveAdjustmentDirection},
    domain::{AssetState, BankReserve},
};
use axum::{
    Json,
    extract::{Path, State},
};
pub(crate) async fn list_address_restrictions(
    State(s): State<AppState>,
) -> Result<Json<Vec<AddressRestriction>>, ApiError> {
    s.address_restriction_service
        .list()
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn block_address(
    State(s): State<AppState>,
    ValidatedJson(r): ValidatedJson<AddressRestrictionRequest>,
) -> Result<Json<AddressRestriction>, ApiError> {
    let address: String = r.address.into();
    s.address_restriction_service
        .block(&address, r.reason.as_ref())
        .await
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn unblock_address(
    State(s): State<AppState>,
    Path(address): Path<crate::api::validators::text::EvmAddress>,
) -> Result<Json<AddressRestriction>, ApiError> {
    let address: String = address.into();
    s.address_restriction_service
        .unblock(&address)
        .await
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn adjust_reserve(
    State(s): State<AppState>,
    ValidatedJson(r): ValidatedJson<ReserveAdjustmentRequest>,
) -> Result<Json<BankReserve>, ApiError> {
    let direction = match r.direction {
        ReserveAdjustmentRequestDirection::Deposit => ReserveAdjustmentDirection::Deposit,
        ReserveAdjustmentRequestDirection::Withdrawal => ReserveAdjustmentDirection::Withdrawal,
    };
    s.reserve_adjustment_service
        .execute(AdjustReserve {
            operation_id: r.operation_id.into(),
            direction,
            amount_usd: r.amount_usd,
            reason: r.reason.into(),
        })
        .await
        .map(Json)
        .map_err(Into::into)
}
pub(crate) async fn enter_wind_down(
    State(s): State<AppState>,
    ValidatedJson(r): ValidatedJson<WindDownRequest>,
) -> Result<Json<AssetState>, ApiError> {
    s.wind_down_service
        .enter(r.operation_id.as_ref(), r.reason.as_ref())
        .await
        .map(Json)
        .map_err(Into::into)
}
