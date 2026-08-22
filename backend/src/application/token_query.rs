use super::{PollingMonitor, PollingStatus, SnapshotCache};
use crate::domain::TokenObservation;
use std::sync::Arc;
use thiserror::Error;

/// Read-only use case used by HTTP. It never contacts the blockchain directly.
pub struct CachedTokenQueryService {
    cache: Arc<dyn SnapshotCache>,
    monitor: Arc<PollingMonitor>,
}

impl CachedTokenQueryService {
    pub fn new(cache: Arc<dyn SnapshotCache>, monitor: Arc<PollingMonitor>) -> Self {
        Self { cache, monitor }
    }

    pub async fn polling_status(&self) -> PollingStatus {
        self.monitor.status().await
    }

    pub async fn get_latest(&self) -> Result<TokenObservation, QueryError> {
        let status = self.monitor.status().await;
        if !status.is_healthy {
            return Err(QueryError::PollingUnavailable(
                status
                    .last_error
                    .unwrap_or_else(|| "poller has no recent successful read".to_owned()),
            ));
        }
        self.cache
            .latest()
            .await
            .map_err(|error| QueryError::Cache(error.to_string()))?
            .ok_or(QueryError::CacheEmpty)
    }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("blockchain polling is unavailable: {0}")]
    PollingUnavailable(String),
    #[error("snapshot cache is empty")]
    CacheEmpty,
    #[error("{0}")]
    Cache(String),
}
