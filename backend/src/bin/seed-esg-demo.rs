use alloy::primitives::Address;
use crypto_asset_backend::infrastructure::sqlite::SqliteEsgStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database =
        std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/backend-usd.sqlite".to_owned());
    let raw_address = std::env::var("TOKEN_ADDRESS").map_err(|_| "TOKEN_ADDRESS is required")?;
    let address: Address = raw_address.parse()?;
    let contract = address.to_checksum(None);
    let chain_id = std::env::var("CHAIN_ID")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(31_337);
    let store = SqliteEsgStore::open(&database)?;
    let inserted =
        store.seed_demo_week(chain_id, &contract, time::OffsetDateTime::now_utc().date())?;
    println!("ESG demo week ready; inserted {inserted} missing days");
    Ok(())
}
