use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorContextField {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub status: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ErrorContextField>,
    #[serde(skip_serializing, skip_deserializing)]
    source: Option<Arc<dyn StdError + Send + Sync>>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            context: Vec::new(),
            source: None,
        }
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND".into(),
            message: format!("{} not found", resource.into()),
            status: StatusCode::NOT_FOUND.as_u16(),
            context: Vec::new(),
            source: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            code: "UNAUTHORIZED".into(),
            message: "Unauthorized".into(),
            status: StatusCode::UNAUTHORIZED.as_u16(),
            context: Vec::new(),
            source: None,
        }
    }

    pub fn forbidden() -> Self {
        Self {
            code: "FORBIDDEN".into(),
            message: "Forbidden".into(),
            status: StatusCode::FORBIDDEN.as_u16(),
            context: Vec::new(),
            source: None,
        }
    }

    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: "VALIDATION".into(),
            message: "Validation failed".into(),
            status: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
            context: vec![ErrorContextField {
                key: field.into(),
                value: message.into(),
            }],
            source: None,
        }
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".into(),
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            context: Vec::new(),
            source: None,
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push(ErrorContextField {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.source = Some(Arc::new(source));
        self
    }

    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn context(&self) -> &[ErrorContextField] {
        &self.context
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|arc| arc.as_ref() as _)
    }
}
