use crate::domain::{EsgEstimate, EsgObservation, TokenObservation, TokenSnapshot};
use async_trait::async_trait;
use thiserror::Error;

/// Infrastructure-independent boundary for reading the token.
#[async_trait]
pub trait TokenReader: Send + Sync {
    async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError>;
    async fn count_transfer_transactions(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<u64, TokenReadError>;
}

pub trait EsgStore: Send + Sync {
    fn last_processed_block(
        &self,
        chain_id: u64,
        contract: &str,
    ) -> Result<Option<u64>, EsgStoreError>;
    fn record_observation(
        &self,
        chain_id: u64,
        contract: &str,
        block: u64,
        date_utc: &str,
        transaction_count: u64,
    ) -> Result<EsgObservation, EsgStoreError>;
    fn recent_estimates(
        &self,
        chain_id: u64,
        contract: &str,
        limit: u8,
    ) -> Result<Vec<EsgEstimate>, EsgStoreError>;
}

#[derive(Debug, Error)]
pub enum EsgStoreError {
    #[error("ESG persistence failed: {0}")]
    Storage(String),
}

#[derive(Debug, Error)]
pub enum TokenReadError {
    #[error("blockchain RPC request failed: {0}")]
    Rpc(String),
}

/// Storage boundary used by polling (write) and HTTP queries (read).
#[async_trait]
pub trait SnapshotCache: Send + Sync {
    async fn write(&self, observation: TokenObservation) -> Result<(), CacheError>;
    async fn latest(&self) -> Result<Option<TokenObservation>, CacheError>;
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("snapshot cache failed: {0}")]
    Storage(String),
}
