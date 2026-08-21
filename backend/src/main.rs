use std::sync::Arc;

use crypto_asset_backend::{
    api, application::TokenQueryService, config::Config, infrastructure::ethereum::AlloyTokenReader,
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let config = Config::from_env()?;
    let token_reader = AlloyTokenReader::connect(&config.rpc_url, config.token_address).await?;
    let token_service = Arc::new(TokenQueryService::new(Arc::new(token_reader)));
    let app = api::router(token_service);
    let listener = TcpListener::bind(config.http_address).await?;
    info!(address = %config.http_address, "HTTP server started");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
