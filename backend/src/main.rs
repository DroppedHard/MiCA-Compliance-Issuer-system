use std::sync::Arc;

use crypto_asset_backend::{
    api,
    application::{
        AssetStateService, CachedTokenQueryService, ChainPollingService, EsgBroadcaster,
        IssuanceService, ObservationBroadcaster, OperationGate, PollingMonitor, RedemptionService,
        ReserveInitializer, ReserveMonitor, ReservePollingService, SnapshotCache, TokenReader,
        WindDownService, initial_reserve_target_minor,
    },
    config::Config,
    infrastructure::{
        asset_state_sqlite::SqliteAssetStateStore,
        cache::InMemorySnapshotCache,
        ethereum::AlloyTokenReader,
        issuance_sqlite::SqliteIssuanceStore,
        mock_bank_client::{HttpBankTransactionReader, HttpReserveReader},
        operation_decision_sqlite::SqliteOperationDecisionStore,
        redemption_sqlite::SqliteRedemptionStore,
        sqlite::SqliteEsgStore,
        token_issuer::AlloyTokenIssuer,
        wind_down_sqlite::SqliteWindDownAuditStore,
    },
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    init_tracing();
    let config = Config::from_env()?;
    let token_reader = AlloyTokenReader::connect(&config.rpc_url, config.token_address).await?;
    let bank_operations = Arc::new(HttpBankTransactionReader::new(&config.mock_bank_url));
    if config.initialize_reserve_on_startup {
        let initial_snapshot = token_reader.read_snapshot().await?;
        let initial_reserve = initial_reserve_target_minor(
            &initial_snapshot.total_supply_raw,
            initial_snapshot.decimals,
        )?;
        bank_operations.initialize_reserve(initial_reserve).await?;
        info!(
            target_balance_minor = initial_reserve,
            "issuer initialized mockBank reserve at 110% of token supply"
        );
    } else {
        info!("issuer startup reserve initialization is disabled by configuration");
    }
    let cache: Arc<dyn SnapshotCache> =
        Arc::new(InMemorySnapshotCache::new(config.cache_retention));
    let monitor = Arc::new(PollingMonitor::new(config.polling_max_staleness));
    let observations = ObservationBroadcaster::new(32);
    let esg_observations = EsgBroadcaster::new(32);
    let esg_store: Arc<dyn crypto_asset_backend::application::EsgStore> =
        Arc::new(SqliteEsgStore::open(&config.database_path)?);
    let poller = ChainPollingService::new(
        Arc::new(token_reader),
        Arc::clone(&cache),
        Arc::clone(&monitor),
        config.poll_interval,
        observations.clone(),
        Arc::clone(&esg_store),
        esg_observations.clone(),
    );
    tokio::spawn(poller.run());
    let reserve_monitor = ReserveMonitor::new(32);
    let asset_state_service = Arc::new(AssetStateService::new(
        Arc::new(SqliteAssetStateStore::open(&config.database_path)?),
        32,
    ));
    let reserve_poller = ReservePollingService::new(
        Arc::new(HttpReserveReader::new(&config.mock_bank_url)),
        Arc::clone(&cache),
        reserve_monitor.clone(),
        asset_state_service.clone(),
        config.poll_interval,
    );
    tokio::spawn(reserve_poller.run());
    let token_service = Arc::new(CachedTokenQueryService::new(cache, monitor));
    let token_operator = Arc::new(
        AlloyTokenIssuer::connect(
            &config.rpc_url,
            config.token_address,
            &config.issuer_private_key,
        )
        .await?,
    );
    let operation_gate = Arc::new(OperationGate::new(Arc::new(
        SqliteOperationDecisionStore::open(&config.database_path)?,
    )));
    let issuance_service = Arc::new(IssuanceService::new(
        Arc::new(SqliteIssuanceStore::open(&config.database_path)?),
        bank_operations.clone(),
        token_operator.clone(),
        asset_state_service.clone(),
        operation_gate.clone(),
    ));
    let redemption_service = Arc::new(RedemptionService::new(
        Arc::new(SqliteRedemptionStore::open(&config.database_path)?),
        token_operator.clone(),
        bank_operations,
        asset_state_service.clone(),
        operation_gate,
    ));
    let wind_down_service = Arc::new(WindDownService::new(
        asset_state_service.clone(),
        token_operator,
        Arc::new(SqliteWindDownAuditStore::open(&config.database_path)?),
    ));
    let app = api::router(api::RouterDependencies {
        token_service,
        observations,
        esg_observations,
        esg_store,
        reserve_monitor,
        asset_state_service,
        issuance_service,
        redemption_service,
        wind_down_service,
    });
    let listener = TcpListener::bind(config.http_address).await?;
    info!(address = %config.http_address, "HTTP server started");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
