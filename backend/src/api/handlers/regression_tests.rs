#![cfg(test)]
//! Testy regresyjne korzystają z produkcyjnego routera, a nie z jego kopii.

use crate::{
    api::{RouterDependencies, responses::ApiError},
    application::{
        AddressRestrictionError, AddressRestrictionService, AssetStateService,
        CachedTokenQueryService, CaspReportingError, CaspReportingService, EsgBroadcaster,
        EsgStore, IssuanceError, IssuanceService, ObservationBroadcaster, RedemptionError,
        RedemptionService, ReserveAdjustmentDirection, ReserveAdjustmentError,
        ReserveAdjustmentService, ReserveMonitor, WindDownError, WindDownService,
    },
    domain::AssetState,
};
use axum::{Router, http::StatusCode};
use std::{sync::Arc, time::Duration};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            AddressRestrictionChain, BankTransactionReader, ConfirmedBankTransaction,
            IssuanceStore, MintResult, OperationGate, PayoutBank, PollingMonitor, RedemptionToken,
            ReserveAdjustmentGateway, SnapshotCache, TokenIssuer, TokenLifecycle,
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
        response::IntoResponse,
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

        async fn refund_to_casp(&self, _: &str, _: u64) -> Result<(), IssuanceError> {
            Ok(())
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
    impl AddressRestrictionChain for TestToken {
        async fn set_frozen(
            &self,
            _: Address,
            _: bool,
        ) -> Result<Option<String>, AddressRestrictionError> {
            Ok(Some("0xrestriction".into()))
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
        crate::api::router(RouterDependencies {
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
            address_restriction_service: Arc::new(AddressRestrictionService::new(
                Arc::new(
                    crate::infrastructure::address_restriction_sqlite::SqliteAddressRestrictionStore::open(":memory:").unwrap(),
                ),
                Arc::new(TestToken),
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
    async fn demo_threshold_endpoint_creates_enforceable_full_quarter_evidence() {
        let response = test_router(EsgBroadcaster::new(4))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/demo/casp-threshold-breach?year=2026&quarter=2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["calendarDayCount"], 91);
        assert_eq!(json["averageDailyOperationCount"], 1_000_001.0);
        assert_eq!(json["thresholdEnforceable"], true);
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

    #[tokio::test]
    async fn blocked_issuance_has_a_stable_code_and_polish_user_message() {
        let response = ApiError::from(IssuanceError::IssuanceBlocked(
            "reserve coverage is below 100%".into(),
        ))
        .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(json["code"], "issuance_blocked");
        assert_eq!(
            json["userMessage"],
            "Emisja rUSD jest obecnie zablokowana przez emitenta."
        );
    }
}
