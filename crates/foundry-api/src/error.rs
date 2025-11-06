use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use foundry_application::ApplicationError;
use foundry_plugins::{AppError, ErrorContextField};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<FieldError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl From<(AppError, Option<String>)> for ErrorResponse {
    fn from((error, timestamp): (AppError, Option<String>)) -> Self {
        ErrorResponse {
            code: error.code,
            message: error.message,
            status: error.status,
            errors: error.context.into_iter().map(FieldError::from).collect(),
            timestamp,
        }
    }
}

impl From<ErrorContextField> for FieldError {
    fn from(value: ErrorContextField) -> Self {
        FieldError {
            field: value.key,
            message: value.value,
        }
    }
}

#[derive(Debug)]
pub struct HttpError(pub AppError);

impl From<AppError> for HttpError {
    fn from(value: AppError) -> Self {
        HttpError(value)
    }
}

impl From<ApplicationError> for HttpError {
    fn from(err: ApplicationError) -> Self {
        use axum::http::StatusCode;

        let app_error = match err {
            ApplicationError::CommandNotFound(command) => AppError::new(
                "COMMAND_NOT_FOUND",
                format!("Command `{command}` wurde nicht gefunden"),
            )
            .with_status(StatusCode::NOT_FOUND.as_u16()),
            ApplicationError::CommandAlreadyRegistered(command) => AppError::new(
                "COMMAND_ALREADY_REGISTERED",
                format!("Command `{command}` ist bereits registriert"),
            )
            .with_status(StatusCode::CONFLICT.as_u16()),
            ApplicationError::CommandExecution(inner) => AppError::new(
                "COMMAND_EXECUTION_ERROR",
                format!("Command Execution Error: {inner}"),
            )
            .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            ApplicationError::StorageError(message) => {
                AppError::new("STORAGE_ERROR", format!("Storage Error: {message}"))
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
            }
            ApplicationError::RegistryCorrupted => {
                AppError::new("REGISTRY_CORRUPTED", "Registry corrupted: lock poisoned")
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
            }
            ApplicationError::LockPoisoned(msg) => {
                AppError::new("LOCK_POISONED", format!("Lock poisoned: {msg}"))
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
            }
        };

        HttpError(app_error)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = self.0.status_code();
        let timestamp = Some(Utc::now().to_rfc3339());
        let body = ErrorResponse::from((self.0, timestamp));
        (status, Json(body)).into_response()
    }
}

pub type HttpResult<T> = Result<T, HttpError>;
