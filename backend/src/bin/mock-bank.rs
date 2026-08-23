use crypto_asset_backend::mock_bank::{self, MockBankStore};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = env::var("MOCK_BANK_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3100".to_owned())
        .parse()?;
    let database = env::var("MOCK_BANK_DATABASE_PATH")
        .unwrap_or_else(|_| "data/mock-bank-usd.sqlite".to_owned());
    let initial = env::var("MOCK_BANK_INITIAL_BALANCE_MINOR")
        .unwrap_or_else(|_| "500000".to_owned())
        .parse()?;
    let listener = TcpListener::bind(address).await?;
    println!("mockBank listening on http://{address} with database {database}");
    axum::serve(
        listener,
        mock_bank::router(Arc::new(MockBankStore::open(&database, initial)?)),
    )
    .await?;
    Ok(())
}
