use crate::application::{
    AddressRestrictionService, AssetStateService, CachedTokenQueryService, CaspReportingService,
    EsgBroadcaster, EsgStore, IssuanceService, ObservationBroadcaster, RedemptionService,
    ReserveAdjustmentService, ReserveMonitor, WindDownService,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
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
    pub address_restriction_service: Arc<AddressRestrictionService>,
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
    pub address_restriction_service: Arc<AddressRestrictionService>,
}
impl From<RouterDependencies> for AppState {
    fn from(v: RouterDependencies) -> Self {
        Self {
            token_service: v.token_service,
            observations: v.observations,
            esg_observations: v.esg_observations,
            esg_store: v.esg_store,
            reserve_monitor: v.reserve_monitor,
            reserve_adjustment_service: v.reserve_adjustment_service,
            asset_state_service: v.asset_state_service,
            issuance_service: v.issuance_service,
            redemption_service: v.redemption_service,
            wind_down_service: v.wind_down_service,
            casp_reporting_service: v.casp_reporting_service,
            address_restriction_service: v.address_restriction_service,
        }
    }
}
