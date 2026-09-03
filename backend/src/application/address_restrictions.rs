use alloy::primitives::Address;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddressRestriction {
    pub address: String,
    pub reason: String,
    pub active: bool,
    pub transaction_hash: Option<String>,
    pub updated_at_unix_ms: u64,
}

pub trait AddressRestrictionStore: Send + Sync {
    fn list(&self) -> Result<Vec<AddressRestriction>, AddressRestrictionError>;
    fn save(&self, entry: &AddressRestriction) -> Result<(), AddressRestrictionError>;
}

#[async_trait]
pub trait AddressRestrictionChain: Send + Sync {
    async fn set_frozen(
        &self,
        address: Address,
        frozen: bool,
    ) -> Result<Option<String>, AddressRestrictionError>;
}

#[derive(Clone)]
pub struct AddressRestrictionService {
    store: Arc<dyn AddressRestrictionStore>,
    chain: Arc<dyn AddressRestrictionChain>,
}

impl AddressRestrictionService {
    pub fn new(
        store: Arc<dyn AddressRestrictionStore>,
        chain: Arc<dyn AddressRestrictionChain>,
    ) -> Self {
        Self { store, chain }
    }

    pub fn list(&self) -> Result<Vec<AddressRestriction>, AddressRestrictionError> {
        self.store.list()
    }

    pub async fn block(
        &self,
        address: &str,
        reason: &str,
    ) -> Result<AddressRestriction, AddressRestrictionError> {
        let parsed = parse_address(address)?;
        let reason = validate_reason(reason)?;
        let transaction_hash = self.chain.set_frozen(parsed, true).await?;
        let entry = AddressRestriction {
            address: parsed.to_checksum(None),
            reason,
            active: true,
            transaction_hash,
            updated_at_unix_ms: now(),
        };
        self.store.save(&entry)?;
        Ok(entry)
    }

    pub async fn unblock(
        &self,
        address: &str,
    ) -> Result<AddressRestriction, AddressRestrictionError> {
        let parsed = parse_address(address)?;
        let transaction_hash = self.chain.set_frozen(parsed, false).await?;
        let entry = AddressRestriction {
            address: parsed.to_checksum(None),
            reason: "Blokada usunięta przez administratora emitenta".into(),
            active: false,
            transaction_hash,
            updated_at_unix_ms: now(),
        };
        self.store.save(&entry)?;
        Ok(entry)
    }
}

fn parse_address(value: &str) -> Result<Address, AddressRestrictionError> {
    value
        .trim()
        .parse()
        .map_err(|_| AddressRestrictionError::Invalid("podaj prawidłowy adres Ethereum".into()))
}
fn validate_reason(value: &str) -> Result<String, AddressRestrictionError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 500 {
        Err(AddressRestrictionError::Invalid(
            "uzasadnienie musi mieć od 1 do 500 znaków".into(),
        ))
    } else {
        Ok(value.into())
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Error)]
pub enum AddressRestrictionError {
    #[error("nieprawidłowa blokada adresu: {0}")]
    Invalid(String),
    #[error("nie udało się zapisać blokady adresu: {0}")]
    Storage(String),
    #[error("nie udało się zsynchronizować blokady z kontraktem: {0}")]
    Blockchain(String),
}
