use crate::error::HttpError;
use crate::http::AppState;
use axum::{
    extract::{rejection::JsonRejection, FromRequest},
    http::StatusCode,
    Json,
};
use foundry_plugins::{AppError, ValidationRules};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::{Deref, DerefMut};

/// Wrapper around `axum::Json` that maps extractor rejections into `HttpError`.
pub struct AppJson<T>(pub T);

impl<T> AppJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> AppJson<T>
where
    T: Serialize,
{
    pub async fn validate(
        &self,
        state: &AppState,
        rules: ValidationRules,
    ) -> Result<(), HttpError> {
        let payload = serde_json::to_value(&self.0).map_err(|err| {
            HttpError::from(
                AppError::new("INVALID_PAYLOAD", err.to_string())
                    .with_status(StatusCode::BAD_REQUEST.as_u16()),
            )
        })?;

        state.ensure_valid(payload, rules).await?;
        Ok(())
    }
}

impl<T> Deref for AppJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AppJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequest<S> for AppJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = HttpError;

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(rejection) => Err(HttpError::from(json_rejection_to_app_error(rejection))),
        }
    }
}

fn json_rejection_to_app_error(rejection: JsonRejection) -> AppError {
    match rejection {
        JsonRejection::JsonDataError(err) => AppError::validation("body", err.to_string())
            .with_status(StatusCode::UNPROCESSABLE_ENTITY.as_u16()),
        JsonRejection::JsonSyntaxError(err) => AppError::new("INVALID_JSON", err.to_string())
            .with_status(StatusCode::BAD_REQUEST.as_u16()),
        JsonRejection::MissingJsonContentType(_) => AppError::new(
            "UNSUPPORTED_MEDIA_TYPE",
            "Expected request with `Content-Type: application/json`",
        )
        .with_status(StatusCode::UNSUPPORTED_MEDIA_TYPE.as_u16()),
        JsonRejection::BytesRejection(err) => AppError::new("PAYLOAD_ERROR", err.to_string())
            .with_status(StatusCode::BAD_REQUEST.as_u16()),
        other => AppError::new("INVALID_JSON", other.to_string())
            .with_status(StatusCode::BAD_REQUEST.as_u16()),
    }
}
