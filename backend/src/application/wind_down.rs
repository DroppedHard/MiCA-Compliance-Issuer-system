use crate::{application::AssetStateService, domain::AssetState};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindDownAudit {
    pub operation_id: String,
    pub reason: String,
    pub transaction_hash: Option<String>,
    pub confirmed_at_unix_ms: u64,
}

#[async_trait]
pub trait TokenLifecycle: Send + Sync {
    async fn enter_wind_down(&self) -> Result<Option<String>, WindDownError>;
}

pub trait WindDownAuditStore: Send + Sync {
    fn get(&self, operation_id: &str) -> Result<Option<WindDownAudit>, WindDownError>;
    fn append(&self, audit: &WindDownAudit) -> Result<(), WindDownError>;
}

#[derive(Clone)]
pub struct WindDownService {
    asset_state: Arc<AssetStateService>,
    token: Arc<dyn TokenLifecycle>,
    audit: Arc<dyn WindDownAuditStore>,
    lock: Arc<tokio::sync::Mutex<()>>,
}

impl WindDownService {
    pub fn new(
        asset_state: Arc<AssetStateService>,
        token: Arc<dyn TokenLifecycle>,
        audit: Arc<dyn WindDownAuditStore>,
    ) -> Self {
        Self {
            asset_state,
            token,
            audit,
            lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub async fn enter(
        &self,
        operation_id: &str,
        reason: &str,
    ) -> Result<AssetState, WindDownError> {
        validate(operation_id, reason)?;
        let _guard = self.lock.lock().await;
        if let Some(existing) = self.audit.get(operation_id)? {
            if existing.reason != reason {
                return Err(WindDownError::IdempotencyConflict);
            }
            return self.asset_state.current().map_err(state_error);
        }

        let transaction_hash = self.token.enter_wind_down().await?;
        let state = self
            .asset_state
            .enter_wind_down(reason)
            .map_err(state_error)?;
        self.audit.append(&WindDownAudit {
            operation_id: operation_id.to_owned(),
            reason: reason.to_owned(),
            transaction_hash,
            confirmed_at_unix_ms: unix_ms(),
        })?;
        Ok(state)
    }
}

fn validate(operation_id: &str, reason: &str) -> Result<(), WindDownError> {
    if operation_id.trim().is_empty() || operation_id.len() > 128 {
        return Err(WindDownError::Invalid(
            "operationId must contain 1-128 characters".into(),
        ));
    }
    if reason.trim().is_empty() || reason.len() > 500 {
        return Err(WindDownError::Invalid(
            "reason must contain 1-500 characters".into(),
        ));
    }
    Ok(())
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn state_error(error: impl std::fmt::Display) -> WindDownError {
    WindDownError::State(error.to_string())
}

#[derive(Debug, Error)]
pub enum WindDownError {
    #[error("invalid wind-down command: {0}")]
    Invalid(String),
    #[error("operationId is already associated with a different wind-down command")]
    IdempotencyConflict,
    #[error("wind-down blockchain command failed: {0}")]
    Blockchain(String),
    #[error("wind-down state update failed: {0}")]
    State(String),
    #[error("wind-down audit persistence failed: {0}")]
    Storage(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::AssetStateStore,
        domain::{AssetState, AssetStateCode},
    };
    use std::sync::Mutex;

    struct StateStore(Mutex<Option<AssetState>>);
    impl AssetStateStore for StateStore {
        fn load(&self) -> Result<Option<AssetState>, crate::application::AssetStateError> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save(&self, state: &AssetState) -> Result<(), crate::application::AssetStateError> {
            *self.0.lock().unwrap() = Some(state.clone());
            Ok(())
        }
    }

    struct Token {
        calls: Mutex<u8>,
        fail: bool,
    }
    #[async_trait]
    impl TokenLifecycle for Token {
        async fn enter_wind_down(&self) -> Result<Option<String>, WindDownError> {
            *self.calls.lock().unwrap() += 1;
            if self.fail {
                Err(WindDownError::Blockchain("reverted".into()))
            } else {
                Ok(Some("0xconfirmed".into()))
            }
        }
    }

    #[derive(Default)]
    struct AuditStore(Mutex<Vec<WindDownAudit>>);
    impl WindDownAuditStore for AuditStore {
        fn get(&self, id: &str) -> Result<Option<WindDownAudit>, WindDownError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|audit| audit.operation_id == id)
                .cloned())
        }
        fn append(&self, audit: &WindDownAudit) -> Result<(), WindDownError> {
            self.0.lock().unwrap().push(audit.clone());
            Ok(())
        }
    }

    fn initial_state() -> AssetState {
        AssetState {
            state: AssetStateCode::Active,
            reason: "covered".into(),
            reserve_coverage_percent: Some(110.0),
            evidence_at_unix_ms: Some(1),
            policy_version: "reserve-coverage-v1".into(),
            updated_at_unix_ms: 1,
        }
    }

    #[tokio::test]
    async fn persists_state_only_after_chain_confirmation_and_is_idempotent() {
        let token = Arc::new(Token {
            calls: Mutex::new(0),
            fail: false,
        });
        let audit = Arc::new(AuditStore::default());
        let service = WindDownService::new(
            Arc::new(AssetStateService::new(
                Arc::new(StateStore(Mutex::new(Some(initial_state())))),
                4,
            )),
            token.clone(),
            audit.clone(),
        );

        assert_eq!(
            service
                .enter("wind-1", "authority decision")
                .await
                .unwrap()
                .state,
            AssetStateCode::WindDown
        );
        service.enter("wind-1", "authority decision").await.unwrap();
        assert_eq!(*token.calls.lock().unwrap(), 1);
        assert_eq!(audit.0.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn failed_chain_command_does_not_publish_wind_down() {
        let asset_state = Arc::new(AssetStateService::new(
            Arc::new(StateStore(Mutex::new(Some(initial_state())))),
            4,
        ));
        let service = WindDownService::new(
            asset_state.clone(),
            Arc::new(Token {
                calls: Mutex::new(0),
                fail: true,
            }),
            Arc::new(AuditStore::default()),
        );

        assert!(matches!(
            service.enter("wind-1", "authority decision").await,
            Err(WindDownError::Blockchain(_))
        ));
        assert_eq!(asset_state.current().unwrap().state, AssetStateCode::Active);
    }
}
