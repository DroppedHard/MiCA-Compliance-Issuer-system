use crate::{
    application::{IssuanceError, IssuanceEvidenceReader, ReserveReader, TokenReader},
    domain::{BankReserve, TokenSnapshot},
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct LiveIssuanceEvidenceReader {
    token: Arc<dyn TokenReader>,
    reserve: Arc<dyn ReserveReader>,
}

impl LiveIssuanceEvidenceReader {
    pub fn new(token: Arc<dyn TokenReader>, reserve: Arc<dyn ReserveReader>) -> Self {
        Self { token, reserve }
    }
}

#[async_trait]
impl IssuanceEvidenceReader for LiveIssuanceEvidenceReader {
    async fn read(&self) -> Result<(TokenSnapshot, BankReserve), IssuanceError> {
        let token = self
            .token
            .read_snapshot()
            .await
            .map_err(|error| IssuanceError::CoverageUnavailable(error.to_string()))?;
        let reserve = self
            .reserve
            .read_reserve()
            .await
            .map_err(|error| IssuanceError::CoverageUnavailable(error.to_string()))?;
        Ok((token, reserve))
    }
}
