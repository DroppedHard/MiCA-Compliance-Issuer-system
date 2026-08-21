use crate::domain::TokenSnapshot;
use async_trait::async_trait;
use thiserror::Error;

/// Infrastructure-independent boundary for reading the token.
#[async_trait]
pub trait TokenReader: Send + Sync {
    async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError>;
}

#[derive(Debug, Error)]
pub enum TokenReadError {
    #[error("blockchain RPC request failed: {0}")]
    Rpc(String),
}
