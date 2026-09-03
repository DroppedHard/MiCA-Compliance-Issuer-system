use crate::application::{
    AddressRestrictionError, CaspReportingError, IssuanceError, QueryError, RedemptionError,
    ReserveAdjustmentError, WindDownError,
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
pub(crate) enum ApiError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    IssuanceBlocked(String),
    Unavailable(String),
    Internal(String),
}
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    code: &'static str,
    #[serde(rename = "userMessage", skip_serializing_if = "Option::is_none")]
    user_message: Option<&'static str>,
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, code, user_message) = match self {
            Self::BadRequest(v) => (StatusCode::BAD_REQUEST, v, "bad_request", None),
            Self::NotFound(v) => (StatusCode::NOT_FOUND, v, "not_found", None),
            Self::Conflict(v) => (StatusCode::CONFLICT, v, "conflict", None),
            Self::IssuanceBlocked(v) => (
                StatusCode::CONFLICT,
                v,
                "issuance_blocked",
                Some("Emisja rUSD jest obecnie zablokowana przez emitenta."),
            ),
            Self::Unavailable(v) => (
                StatusCode::SERVICE_UNAVAILABLE,
                v,
                "service_unavailable",
                None,
            ),
            Self::Internal(v) => (StatusCode::INTERNAL_SERVER_ERROR, v, "internal_error", None),
        };
        (
            status,
            Json(ErrorBody {
                error,
                code,
                user_message,
            }),
        )
            .into_response()
    }
}
impl From<IssuanceError> for ApiError {
    fn from(e: IssuanceError) -> Self {
        match e {
            IssuanceError::Invalid(_) => Self::BadRequest(e.to_string()),
            IssuanceError::NotFound => Self::NotFound(e.to_string()),
            IssuanceError::IssuanceBlocked(_) => Self::IssuanceBlocked(e.to_string()),
            IssuanceError::IdempotencyConflict
            | IssuanceError::FiatNotConfirmed
            | IssuanceError::BankMismatch
            | IssuanceError::SettlementInProgress => Self::Conflict(e.to_string()),
            IssuanceError::Bank(_) => Self::Unavailable(e.to_string()),
            _ => Self::Internal(e.to_string()),
        }
    }
}
impl From<RedemptionError> for ApiError {
    fn from(e: RedemptionError) -> Self {
        match e {
            RedemptionError::Invalid(_) => Self::BadRequest(e.to_string()),
            RedemptionError::NotFound => Self::NotFound(e.to_string()),
            RedemptionError::IdempotencyConflict => Self::Conflict(e.to_string()),
            _ => Self::Internal(e.to_string()),
        }
    }
}
impl From<WindDownError> for ApiError {
    fn from(e: WindDownError) -> Self {
        match e {
            WindDownError::Invalid(_) => Self::BadRequest(e.to_string()),
            WindDownError::IdempotencyConflict => Self::Conflict(e.to_string()),
            WindDownError::Blockchain(_) => Self::Unavailable(e.to_string()),
            _ => Self::Internal(e.to_string()),
        }
    }
}
impl From<ReserveAdjustmentError> for ApiError {
    fn from(e: ReserveAdjustmentError) -> Self {
        match e {
            ReserveAdjustmentError::Invalid(_) => Self::BadRequest(e.to_string()),
            ReserveAdjustmentError::Bank(_) => Self::Unavailable(e.to_string()),
        }
    }
}
impl From<QueryError> for ApiError {
    fn from(e: QueryError) -> Self {
        match e {
            QueryError::PollingUnavailable(v) => Self::Unavailable(v),
            QueryError::CacheEmpty => Self::Unavailable(e.to_string()),
            QueryError::Cache(v) => Self::Internal(v),
        }
    }
}
impl From<CaspReportingError> for ApiError {
    fn from(e: CaspReportingError) -> Self {
        match e {
            CaspReportingError::InvalidRange | CaspReportingError::SourceContract(_) => {
                Self::BadRequest(e.to_string())
            }
            CaspReportingError::Source(_) => Self::Unavailable(e.to_string()),
            _ => Self::Internal(e.to_string()),
        }
    }
}
impl From<AddressRestrictionError> for ApiError {
    fn from(e: AddressRestrictionError) -> Self {
        match e {
            AddressRestrictionError::Invalid(_) => Self::BadRequest(e.to_string()),
            AddressRestrictionError::Blockchain(_) => Self::Unavailable(e.to_string()),
            AddressRestrictionError::Storage(_) => Self::Internal(e.to_string()),
        }
    }
}
