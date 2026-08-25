use crate::{
    application::{
        BankTransactionReader, ConfirmedBankTransaction, IssuanceError, PayoutBank,
        RedemptionError, ReserveError, ReserveInitializer, ReserveReader,
    },
    domain::BankReserve,
};
use async_trait::async_trait;

pub struct HttpReserveReader {
    client: reqwest::Client,
    url: String,
}
#[async_trait]
impl PayoutBank for HttpBankTransactionReader {
    async fn pay_usd(&self, id: &str, amount_minor: u64) -> Result<(), RedemptionError> {
        let body = serde_json::json!({"amountMinor":amount_minor.to_string(),"reference":id,"idempotencyKey":format!("redemption-{id}")});
        self.client
            .post(format!(
                "{}/api/v1/reserve-accounts/reserve-rusd/withdrawals",
                self.base_url
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| RedemptionError::Bank(e.to_string()))?
            .error_for_status()
            .map_err(|e| RedemptionError::Bank(e.to_string()))?;
        Ok(())
    }
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
impl ReserveInitializer for HttpBankTransactionReader {
    async fn initialize_reserve(&self, target: u64) -> Result<(), ReserveError> {
        self.client.put(format!("{}/api/v1/admin/reserve-accounts/reserve-rusd/initialize",self.base_url)).json(&serde_json::json!({"targetBalanceMinor":target.to_string(),"reference":"issuer-startup-110-percent"})).send().await.map_err(bank)?.error_for_status().map_err(bank)?;
        Ok(())
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
