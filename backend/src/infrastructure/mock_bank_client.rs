use crate::{
    application::{
        BankTransactionReader, ConfirmedBankTransaction, IssuanceError, ReserveError, ReserveReader,
    },
    domain::BankReserve,
};
use async_trait::async_trait;

pub struct HttpReserveReader {
    client: reqwest::Client,
    url: String,
}
impl HttpReserveReader {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: format!(
                "{}/api/v1/reserve-accounts/reserve-rusd",
                base_url.trim_end_matches('/')
            ),
        }
    }
}
#[async_trait]
impl ReserveReader for HttpReserveReader {
    async fn read_reserve(&self) -> Result<BankReserve, ReserveError> {
        self.client
            .get(&self.url)
            .send()
            .await
            .map_err(bank)?
            .error_for_status()
            .map_err(bank)?
            .json()
            .await
            .map_err(bank)
    }
}
fn bank(error: impl std::fmt::Display) -> ReserveError {
    ReserveError::Bank(error.to_string())
}

pub struct HttpBankTransactionReader {
    client: reqwest::Client,
    base_url: String,
}
impl HttpBankTransactionReader {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }
}
#[async_trait]
impl BankTransactionReader for HttpBankTransactionReader {
    async fn find(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<ConfirmedBankTransaction>, IssuanceError> {
        let response = self
            .client
            .get(format!(
                "{}/api/v1/reserve-transactions/{idempotency_key}",
                self.base_url
            ))
            .send()
            .await
            .map_err(|e| IssuanceError::Bank(e.to_string()))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let value = response
            .error_for_status()
            .map_err(|e| IssuanceError::Bank(e.to_string()))?
            .json::<crate::mock_bank::ReserveTransaction>()
            .await
            .map_err(|e| IssuanceError::Bank(e.to_string()))?;
        Ok(Some(ConfirmedBankTransaction {
            operation_type: value.operation_type,
            amount_minor: value.amount_minor,
            reference: value.reference,
        }))
    }
}
