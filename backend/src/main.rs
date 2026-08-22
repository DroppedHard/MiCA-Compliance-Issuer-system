use std::sync::Arc;

use crypto_asset_backend::{
    api,
    application::{
        CachedTokenQueryService, ChainPollingService, EsgBroadcaster, ObservationBroadcaster,
        PollingMonitor, SnapshotCache,
    },
    config::Config,
    infrastructure::{
        cache::InMemorySnapshotCache, ethereum::AlloyTokenReader, sqlite::SqliteEsgStore,
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
    let token_service = Arc::new(CachedTokenQueryService::new(cache, monitor));
    let app = api::router(token_service, observations, esg_observations, esg_store);
    let listener = TcpListener::bind(config.http_address).await?;
    info!(address = %config.http_address, "HTTP server started");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
