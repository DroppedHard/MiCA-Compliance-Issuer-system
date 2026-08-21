use alloy::primitives::Address;
use std::{env, net::SocketAddr};
use thiserror::Error;

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";
const DEFAULT_HTTP_ADDRESS: &str = "127.0.0.1:3000";

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub token_address: Address,
    pub http_address: SocketAddr,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("TOKEN_ADDRESS is required and must contain a deployed contract address")]
    MissingTokenAddress,
    #[error("TOKEN_ADDRESS is not a valid Ethereum address: {0}")]
    InvalidTokenAddress(String),
    #[error("HTTP_ADDRESS is not a valid socket address: {0}")]
    InvalidHttpAddress(String),
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
        Ok(Self {
            rpc_url,
            token_address,
            http_address,
        })
    }
}
