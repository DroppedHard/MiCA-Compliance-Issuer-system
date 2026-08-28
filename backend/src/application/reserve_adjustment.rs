use crate::domain::BankReserve;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveAdjustmentDirection {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone)]
pub struct AdjustReserve {
    pub operation_id: String,
    pub direction: ReserveAdjustmentDirection,
    pub amount_usd: String,
    pub reason: String,
}

#[async_trait]
pub trait ReserveAdjustmentGateway: Send + Sync {
    async fn adjust(
        &self,
        operation_id: &str,
        direction: ReserveAdjustmentDirection,
        amount_minor: u64,
        reason: &str,
    ) -> Result<BankReserve, ReserveAdjustmentError>;
}

#[derive(Clone)]
pub struct ReserveAdjustmentService {
    gateway: Arc<dyn ReserveAdjustmentGateway>,
}

impl ReserveAdjustmentService {
    pub fn new(gateway: Arc<dyn ReserveAdjustmentGateway>) -> Self {
        Self { gateway }
    }

    pub async fn execute(
        &self,
        command: AdjustReserve,
    ) -> Result<BankReserve, ReserveAdjustmentError> {
        let operation_id = command.operation_id.trim();
        if operation_id.is_empty() || operation_id.len() > 128 {
            return Err(ReserveAdjustmentError::Invalid(
                "operationId must contain 1-128 characters".into(),
            ));
        }
        let reason = command.reason.trim();
        if reason.is_empty() || reason.len() > 500 {
            return Err(ReserveAdjustmentError::Invalid(
                "reason must contain 1-500 characters".into(),
            ));
        }
        let amount_minor = parse_usd_minor(&command.amount_usd)?;
        self.gateway
            .adjust(operation_id, command.direction, amount_minor, reason)
            .await
    }
}

fn parse_usd_minor(value: &str) -> Result<u64, ReserveAdjustmentError> {
    let value = value.trim();
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || parts.next().is_some()
    {
        return Err(invalid_amount());
    }
    let dollars = whole.parse::<u64>().map_err(|_| invalid_amount())?;
    let cents = match fraction {
        None => 0,
        Some(value) if value.len() == 1 && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            value.parse::<u64>().map_err(|_| invalid_amount())? * 10
        }
        Some(value) if value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            value.parse::<u64>().map_err(|_| invalid_amount())?
        }
        _ => return Err(invalid_amount()),
    };
    let amount = dollars
        .checked_mul(100)
        .and_then(|value| value.checked_add(cents))
        .filter(|value| *value > 0)
        .ok_or_else(invalid_amount)?;
    Ok(amount)
}

fn invalid_amount() -> ReserveAdjustmentError {
    ReserveAdjustmentError::Invalid(
        "amountUsd must be a positive decimal with at most two fractional digits".into(),
    )
}

#[derive(Debug, Error)]
pub enum ReserveAdjustmentError {
    #[error("invalid reserve adjustment: {0}")]
    Invalid(String),
    #[error("mock bank reserve adjustment failed: {0}")]
    Bank(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Gateway(Mutex<Option<(String, ReserveAdjustmentDirection, u64, String)>>);
    #[async_trait]
    impl ReserveAdjustmentGateway for Gateway {
        async fn adjust(
            &self,
            id: &str,
            direction: ReserveAdjustmentDirection,
            amount: u64,
            reason: &str,
        ) -> Result<BankReserve, ReserveAdjustmentError> {
            *self.0.lock().unwrap() = Some((id.into(), direction, amount, reason.into()));
            Ok(BankReserve {
                account_id: "reserve-rusd".into(),
                currency: "USD".into(),
                balance_minor: "100".into(),
                version: 2,
                as_of_unix_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn converts_usd_exactly_and_passes_auditable_context() {
        let gateway = Arc::new(Gateway(Mutex::new(None)));
        let service = ReserveAdjustmentService::new(gateway.clone());
        service
            .execute(AdjustReserve {
                operation_id: "test-1".into(),
                direction: ReserveAdjustmentDirection::Withdrawal,
                amount_usd: "12.30".into(),
                reason: "shortfall test".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            *gateway.0.lock().unwrap(),
            Some((
                "test-1".into(),
                ReserveAdjustmentDirection::Withdrawal,
                1230,
                "shortfall test".into()
            ))
        );
    }

    #[test]
    fn rejects_zero_negative_and_excess_precision() {
        for value in ["0", "-1", "1.001", "", ".50"] {
            assert!(parse_usd_minor(value).is_err(), "{value}");
        }
    }
}
