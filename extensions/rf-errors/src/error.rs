//! RustForge error types
//!
//! Comprehensive error types with context, error codes, and friendly messages.

use crate::code::ErrorCode;
use crate::context::ErrorContext;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Main RustForge error type
#[derive(Debug, Error)]
pub enum RustForgeError {
    /// Database errors
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(#[from] AuthenticationError),

    /// Authorization errors
    #[error("Authorization error: {0}")]
    Authorization(#[from] AuthorizationError),

    /// Cache errors
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// Queue errors
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),

    /// HTTP errors
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),

    /// Template errors
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    /// Storage errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Mail errors
    #[error("Mail error: {0}")]
    Mail(#[from] MailError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigurationError),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl RustForgeError {
    /// Get the error code
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Database(e) => e.code(),
            Self::Validation(e) => e.code(),
            Self::Authentication(e) => e.code(),
            Self::Authorization(e) => e.code(),
            Self::Cache(e) => e.code(),
            Self::Queue(e) => e.code(),
            Self::Http(e) => e.code(),
            Self::Template(e) => e.code(),
            Self::Storage(e) => e.code(),
            Self::Mail(e) => e.code(),
            Self::Configuration(e) => e.code(),
            Self::Internal(_) => ErrorCode::InternalError,
        }
    }

    /// Get error context
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::Database(e) => Some(&e.context),
            Self::Validation(e) => Some(&e.context),
            Self::Authentication(e) => Some(&e.context),
            Self::Authorization(e) => Some(&e.context),
            Self::Cache(e) => Some(&e.context),
            Self::Queue(e) => Some(&e.context),
            Self::Http(e) => Some(&e.context),
            Self::Template(e) => Some(&e.context),
            Self::Storage(e) => Some(&e.context),
            Self::Mail(e) => Some(&e.context),
            Self::Configuration(e) => Some(&e.context),
            Self::Internal(_) => None,
        }
    }

    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Database(_) => 500,
            Self::Validation(_) => 422,
            Self::Authentication(_) => 401,
            Self::Authorization(_) => 403,
            Self::Cache(_) => 500,
            Self::Queue(_) => 500,
            Self::Http(e) => e.status_code(),
            Self::Template(_) => 500,
            Self::Storage(_) => 500,
            Self::Mail(_) => 500,
            Self::Configuration(_) => 500,
            Self::Internal(_) => 500,
        }
    }
}

/// Database error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseError {
    pub kind: DatabaseErrorKind,
    pub message: String,
    pub context: ErrorContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseErrorKind {
    Connection {
        host: String,
        database: String,
        user: String,
    },
    Query {
        query: String,
        error: String,
    },
    Migration {
        version: String,
        error: String,
    },
    Transaction {
        error: String,
    },
    PoolExhausted {
        max_connections: usize,
    },
}

impl DatabaseError {
    pub fn connection(
        host: impl Into<String>,
        database: impl Into<String>,
        user: impl Into<String>,
    ) -> Self {
        Self {
            kind: DatabaseErrorKind::Connection {
                host: host.into(),
                database: database.into(),
                user: user.into(),
            },
            message: "Failed to connect to database".to_string(),
            context: ErrorContext::new().with_tag("database"),
        }
    }

    pub fn query(query: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            kind: DatabaseErrorKind::Query {
                query: query.into(),
                error: error.into(),
            },
            message: "Database query failed".to_string(),
            context: ErrorContext::new().with_tag("database"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match &self.kind {
            DatabaseErrorKind::Connection { .. } => ErrorCode::DatabaseConnection,
            DatabaseErrorKind::Query { .. } => ErrorCode::DatabaseQuery,
            DatabaseErrorKind::Migration { .. } => ErrorCode::DatabaseMigration,
            DatabaseErrorKind::Transaction { .. } => ErrorCode::DatabaseTransaction,
            DatabaseErrorKind::PoolExhausted { .. } => ErrorCode::DatabasePoolExhausted,
        }
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code())
    }
}

impl std::error::Error for DatabaseError {}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: Option<String>,
    pub context: ErrorContext,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            value: None,
            context: ErrorContext::new().with_tag("validation"),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn code(&self) -> ErrorCode {
        ErrorCode::ValidationFailed
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Validation failed for field '{}': {}",
            self.field, self.message
        )
    }
}

impl std::error::Error for ValidationError {}

/// Authentication error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationError {
    pub kind: AuthenticationErrorKind,
    pub message: String,
    pub context: ErrorContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationErrorKind {
    InvalidCredentials,
    TokenExpired,
    TokenInvalid,
    UserNotFound,
    AccountLocked,
    EmailNotVerified,
}

impl AuthenticationError {
    pub fn invalid_credentials() -> Self {
        Self {
            kind: AuthenticationErrorKind::InvalidCredentials,
            message: "Invalid username or password".to_string(),
            context: ErrorContext::new().with_tag("authentication"),
        }
    }

    pub fn token_expired() -> Self {
        Self {
            kind: AuthenticationErrorKind::TokenExpired,
            message: "Authentication token has expired".to_string(),
            context: ErrorContext::new().with_tag("authentication"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self.kind {
            AuthenticationErrorKind::InvalidCredentials => ErrorCode::AuthInvalidCredentials,
            AuthenticationErrorKind::TokenExpired => ErrorCode::AuthTokenExpired,
            AuthenticationErrorKind::TokenInvalid => ErrorCode::AuthTokenInvalid,
            AuthenticationErrorKind::UserNotFound => ErrorCode::AuthUserNotFound,
            AuthenticationErrorKind::AccountLocked => ErrorCode::AuthAccountLocked,
            AuthenticationErrorKind::EmailNotVerified => ErrorCode::AuthEmailNotVerified,
        }
    }
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code())
    }
}

impl std::error::Error for AuthenticationError {}

/// Authorization error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationError {
    pub reason: String,
    pub required_permission: Option<String>,
    pub context: ErrorContext,
}

impl AuthorizationError {
    pub fn forbidden(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            required_permission: None,
            context: ErrorContext::new().with_tag("authorization"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        ErrorCode::AuthorizationForbidden
    }
}

impl fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Access forbidden: {}", self.reason)
    }
}

impl std::error::Error for AuthorizationError {}

/// Cache error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheError {
    pub operation: String,
    pub message: String,
    pub context: ErrorContext,
}

impl CacheError {
    pub fn connection_failed(message: impl Into<String>) -> Self {
        Self {
            operation: "connect".to_string(),
            message: message.into(),
            context: ErrorContext::new().with_tag("cache"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        if self.operation == "connect" {
            ErrorCode::CacheConnection
        } else {
            ErrorCode::CacheOperation
        }
    }
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cache operation '{}' failed: {}",
            self.operation, self.message
        )
    }
}

impl std::error::Error for CacheError {}

/// Queue error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueError {
    pub operation: String,
    pub job_type: Option<String>,
    pub message: String,
    pub context: ErrorContext,
}

impl QueueError {
    pub fn dispatch_failed(job_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            operation: "dispatch".to_string(),
            job_type: Some(job_type.into()),
            message: message.into(),
            context: ErrorContext::new().with_tag("queue"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self.operation.as_str() {
            "connect" => ErrorCode::QueueConnection,
            "dispatch" => ErrorCode::QueueDispatch,
            _ => ErrorCode::QueueJobFailed,
        }
    }
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Queue operation '{}' failed: {}",
            self.operation, self.message
        )
    }
}

impl std::error::Error for QueueError {}

/// HTTP error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
    pub context: ErrorContext,
}

impl HttpError {
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self {
            status: 404,
            message: format!("Resource not found: {}", resource.into()),
            context: ErrorContext::new().with_tag("http"),
        }
    }

    pub fn rate_limit_exceeded() -> Self {
        Self {
            status: 429,
            message: "Rate limit exceeded".to_string(),
            context: ErrorContext::new().with_tag("http"),
        }
    }

    pub fn status_code(&self) -> u16 {
        self.status
    }

    pub fn code(&self) -> ErrorCode {
        match self.status {
            404 => ErrorCode::HttpRouteNotFound,
            429 => ErrorCode::HttpRateLimitExceeded,
            _ => ErrorCode::HttpRequestFailed,
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP {} - {}", self.status, self.message)
    }
}

impl std::error::Error for HttpError {}

/// Template error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateError {
    pub template: String,
    pub message: String,
    pub context: ErrorContext,
}

impl TemplateError {
    pub fn not_found(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            message: "Template not found".to_string(),
            context: ErrorContext::new().with_tag("template"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        if self.message.contains("not found") {
            ErrorCode::TemplateNotFound
        } else if self.message.contains("compilation") {
            ErrorCode::TemplateCompilationFailed
        } else {
            ErrorCode::TemplateRenderingFailed
        }
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Template '{}': {}", self.template, self.message)
    }
}

impl std::error::Error for TemplateError {}

/// Storage error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageError {
    pub operation: String,
    pub path: Option<String>,
    pub message: String,
    pub context: ErrorContext,
}

impl StorageError {
    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self {
            operation: "read".to_string(),
            path: Some(path.into()),
            message: "File not found".to_string(),
            context: ErrorContext::new().with_tag("storage"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        if self.message.contains("not found") {
            ErrorCode::StorageFileNotFound
        } else {
            ErrorCode::StorageConnection
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref path) = self.path {
            write!(
                f,
                "Storage operation '{}' on '{}': {}",
                self.operation, path, self.message
            )
        } else {
            write!(
                f,
                "Storage operation '{}': {}",
                self.operation, self.message
            )
        }
    }
}

impl std::error::Error for StorageError {}

/// Mail error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailError {
    pub operation: String,
    pub message: String,
    pub context: ErrorContext,
}

impl MailError {
    pub fn send_failed(message: impl Into<String>) -> Self {
        Self {
            operation: "send".to_string(),
            message: message.into(),
            context: ErrorContext::new().with_tag("mail"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        ErrorCode::MailSendFailed
    }
}

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mail operation '{}' failed: {}",
            self.operation, self.message
        )
    }
}

impl std::error::Error for MailError {}

/// Configuration error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationError {
    pub key: String,
    pub message: String,
    pub context: ErrorContext,
}

impl ConfigurationError {
    pub fn missing_env_var(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            key: key.clone(),
            message: format!("Environment variable '{}' is not set", key),
            context: ErrorContext::new().with_tag("configuration"),
        }
    }

    pub fn code(&self) -> ErrorCode {
        if self.message.contains("not set") {
            ErrorCode::EnvVarMissing
        } else {
            ErrorCode::ConfigurationError
        }
    }
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Configuration error for '{}': {}",
            self.key, self.message
        )
    }
}

impl std::error::Error for ConfigurationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_creation() {
        let err = DatabaseError::connection("localhost:5432", "mydb", "postgres");
        assert_eq!(err.code(), ErrorCode::DatabaseConnection);
        // Display should show message and error code
        let display = err.to_string();
        assert!(display.contains("database") || display.contains("RF001"));
    }

    #[test]
    fn test_validation_error_creation() {
        let err = ValidationError::new("email", "Invalid email format").with_value("not-an-email");
        assert_eq!(err.field, "email");
        assert_eq!(err.value, Some("not-an-email".to_string()));
    }

    #[test]
    fn test_authentication_error() {
        let err = AuthenticationError::invalid_credentials();
        assert_eq!(err.code(), ErrorCode::AuthInvalidCredentials);
    }

    #[test]
    fn test_http_error_status_code() {
        let err = HttpError::not_found("User");
        assert_eq!(err.status_code(), 404);
        assert_eq!(err.code(), ErrorCode::HttpRouteNotFound);
    }

    #[test]
    fn test_rustforge_error_code() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        assert_eq!(err.code(), ErrorCode::DatabaseConnection);
        assert_eq!(err.status_code(), 500);
    }
}
