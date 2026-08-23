use std::sync::Arc;

use crypto_asset_backend::{
    api,
    application::{
        CachedTokenQueryService, ChainPollingService, EsgBroadcaster, IssuanceService,
        ObservationBroadcaster, PollingMonitor, ReserveMonitor, ReservePollingService,
        SnapshotCache,
    },
    config::Config,
    infrastructure::{
        cache::InMemorySnapshotCache,
        ethereum::AlloyTokenReader,
        issuance_sqlite::SqliteIssuanceStore,
        mock_bank_client::{HttpBankTransactionReader, HttpReserveReader},
        sqlite::SqliteEsgStore,
        token_issuer::AlloyTokenIssuer,
    },
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let config = Config::from_env()?;
    let token_reader = AlloyTokenReader::connect(&config.rpc_url, config.token_address).await?;
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
    let reserve_poller = ReservePollingService::new(
        Arc::new(HttpReserveReader::new(&config.mock_bank_url)),
        Arc::clone(&cache),
        reserve_monitor.clone(),
        config.poll_interval,
    );
    tokio::spawn(reserve_poller.run());
    let token_service = Arc::new(CachedTokenQueryService::new(cache, monitor));
    let issuance_service = Arc::new(IssuanceService::new(
        Arc::new(SqliteIssuanceStore::open(&config.database_path)?),
        Arc::new(HttpBankTransactionReader::new(&config.mock_bank_url)),
        Arc::new(
            AlloyTokenIssuer::connect(
                &config.rpc_url,
                config.token_address,
                &config.issuer_private_key,
            )
            .await?,
        ),
    ));
    let app = api::router(
        token_service,
        observations,
        esg_observations,
        esg_store,
        reserve_monitor,
        issuance_service,
    );
    let listener = TcpListener::bind(config.http_address).await?;
    info!(address = %config.http_address, "HTTP server started");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
