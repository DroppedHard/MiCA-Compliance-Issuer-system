use crate::api::validators::{
    ValidateRequest, ValidationError,
    amount::{validate_decimal_amount, validate_minor_amount},
    text::{EvmAddress, OperationId, Reason},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddressRestrictionRequest {
    pub address: EvmAddress,
    pub reason: Reason,
}
impl ValidateRequest for AddressRestrictionRequest {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReserveAdjustmentRequest {
    pub operation_id: OperationId,
    pub direction: ReserveAdjustmentRequestDirection,
    pub amount_usd: String,
    pub reason: Reason,
}
impl ValidateRequest for ReserveAdjustmentRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_decimal_amount("amountUsd", &self.amount_usd)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReserveAdjustmentRequestDirection {
    Deposit,
    Withdrawal,
}
#[derive(Deserialize)]
pub(crate) struct CaspRangeQuery {
    pub from: String,
    pub to: String,
}
impl ValidateRequest for CaspRangeQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_iso_date("from", &self.from)?;
        validate_iso_date("to", &self.to)
    }
}
#[derive(Deserialize)]
pub(crate) struct QuarterQuery {
    pub year: i32,
    pub quarter: u8,
}
impl ValidateRequest for QuarterQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        if (1..=4).contains(&self.quarter) {
            Ok(())
        } else {
            Err(ValidationError(
                "quarter musi należeć do zakresu 1–4".into(),
            ))
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindDownRequest {
    pub operation_id: OperationId,
    pub reason: Reason,
}
impl ValidateRequest for WindDownRequest {}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateRedemptionRequest {
    pub operation_id: OperationId,
    pub holder_address: EvmAddress,
    pub token_amount_raw: String,
}
impl ValidateRequest for CreateRedemptionRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_minor_amount("tokenAmountRaw", &self.token_amount_raw)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateIssuanceRequest {
    pub operation_id: OperationId,
    pub recipient_address: EvmAddress,
    pub amount_usd_minor: String,
}
impl ValidateRequest for CreateIssuanceRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_minor_amount("amountUsdMinor", &self.amount_usd_minor)
    }
}

fn validate_iso_date(field: &str, value: &str) -> Result<(), ValidationError> {
    let bytes = value.as_bytes();
    let valid_shape = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    if valid_shape {
        Ok(())
    } else {
        Err(ValidationError(format!(
            "{field} musi mieć format RRRR-MM-DD"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_reporting_date_before_calling_the_service() {
        let query = CaspRangeQuery {
            from: "2026/09/04".into(),
            to: "2026-09-04".into(),
        };
        assert!(query.validate().is_err());
    }

    #[test]
    fn rejects_quarter_outside_the_calendar_range() {
        let query = QuarterQuery {
            year: 2026,
            quarter: 5,
        };
        assert!(query.validate().is_err());
    }
}
