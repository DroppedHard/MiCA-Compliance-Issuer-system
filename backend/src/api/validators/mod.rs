pub mod amount;
pub mod text;

use crate::api::responses::ApiError;
use axum::{
    extract::{FromRequest, FromRequestParts, Query, Request},
    response::IntoResponse,
};
use axum_serde::Sonic;
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub(crate) struct ValidationError(pub String);
pub(crate) trait ValidateRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        Ok(())
    }
}
pub(crate) struct ValidatedJson<T>(pub T);
pub(crate) struct ValidatedQuery<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + ValidateRequest,
{
    type Rejection = axum::response::Response;
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Sonic(value) = Sonic::<T>::from_request(req, state)
            .await
            .map_err(|error| ApiError::BadRequest(error.to_string()).into_response())?;
        value
            .validate()
            .map_err(|e| ApiError::BadRequest(e.0).into_response())?;
        Ok(Self(value))
    }
}

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + ValidateRequest,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|error| ApiError::BadRequest(error.to_string()).into_response())?;
        value
            .validate()
            .map_err(|error| ApiError::BadRequest(error.0).into_response())?;
        Ok(Self(value))
    }
}
