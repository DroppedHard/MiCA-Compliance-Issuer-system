use alloy::primitives::Address;
use std::{env, net::SocketAddr, time::Duration};
use thiserror::Error;

pub mod compliance;
pub mod esg;

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";
const DEFAULT_HTTP_ADDRESS: &str = "127.0.0.1:3000";
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 10;
const MAX_POLL_INTERVAL_SECONDS: u64 = 10;
const CACHE_RETENTION_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub token_address: Address,
    pub http_address: SocketAddr,
    pub poll_interval: Duration,
    pub cache_retention: Duration,
    pub polling_max_staleness: Duration,
    pub database_path: String,
    pub mock_bank_url: String,
    pub issuer_private_key: String,
    pub initialize_reserve_on_startup: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("TOKEN_ADDRESS is required and must contain a deployed contract address")]
    MissingTokenAddress,
    #[error("TOKEN_ADDRESS is not a valid Ethereum address: {0}")]
    InvalidTokenAddress(String),
    #[error("HTTP_ADDRESS is not a valid socket address: {0}")]
    InvalidHttpAddress(String),
    #[error("POLL_INTERVAL_SECONDS must be an integer from 1 to 10, got: {0}")]
    InvalidPollInterval(String),
    #[error("ISSUER_PRIVATE_KEY is required for issuance settlement")]
    MissingIssuerPrivateKey,
    #[error("INITIALIZE_RESERVE_ON_STARTUP must be true or false, got: {0}")]
    InvalidReserveInitializationFlag(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_owned());
        let raw_token_address =
            env::var("TOKEN_ADDRESS").map_err(|_| ConfigError::MissingTokenAddress)?;
        let token_address = raw_token_address
            .parse()
            .map_err(|_| ConfigError::InvalidTokenAddress(raw_token_address))?;
        let raw_http_address =
            env::var("HTTP_ADDRESS").unwrap_or_else(|_| DEFAULT_HTTP_ADDRESS.to_owned());
        let http_address = raw_http_address
            .parse()
            .map_err(|_| ConfigError::InvalidHttpAddress(raw_http_address))?;
        let raw_poll_interval = env::var("POLL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| DEFAULT_POLL_INTERVAL_SECONDS.to_string());
        let poll_interval_seconds = raw_poll_interval
            .parse::<u64>()
            .ok()
            .filter(|seconds| (1..=MAX_POLL_INTERVAL_SECONDS).contains(seconds))
            .ok_or_else(|| ConfigError::InvalidPollInterval(raw_poll_interval.clone()))?;
        let poll_interval = Duration::from_secs(poll_interval_seconds);
        Ok(Self {
            rpc_url,
            token_address,
            http_address,
            poll_interval,
            cache_retention: Duration::from_secs(CACHE_RETENTION_SECONDS),
            polling_max_staleness: poll_interval.saturating_mul(3),
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "data/backend-usd.sqlite".to_owned()),
            mock_bank_url: env::var("MOCK_BANK_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3100".to_owned()),
            issuer_private_key: env::var("ISSUER_PRIVATE_KEY")
                .map_err(|_| ConfigError::MissingIssuerPrivateKey)?,
            initialize_reserve_on_startup: boolean("INITIALIZE_RESERVE_ON_STARTUP", true)?,
        })
    }
}
fn boolean(name: &'static str, default: bool) -> Result<bool, ConfigError> {
    let raw = env::var(name).unwrap_or_else(|_| default.to_string());
    match raw.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(ConfigError::InvalidReserveInitializationFlag(raw)),
    }
}
