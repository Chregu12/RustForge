//! Error handling for GraphQL
//!
//! Provides structured error types and error extensions.

use async_graphql::{Error, ErrorExtensions};
use serde::{Deserialize, Serialize};
use std::fmt;

/// GraphQL error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Bad user input
    BadRequest,
    /// Unauthorized access
    Unauthorized,
    /// Forbidden access
    Forbidden,
    /// Resource not found
    NotFound,
    /// Internal server error
    InternalServerError,
    /// Validation error
    ValidationError,
    /// Database error
    DatabaseError,
    /// Rate limit exceeded
    RateLimitExceeded,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::BadRequest => write!(f, "BAD_REQUEST"),
            ErrorCode::Unauthorized => write!(f, "UNAUTHORIZED"),
            ErrorCode::Forbidden => write!(f, "FORBIDDEN"),
            ErrorCode::NotFound => write!(f, "NOT_FOUND"),
            ErrorCode::InternalServerError => write!(f, "INTERNAL_SERVER_ERROR"),
            ErrorCode::ValidationError => write!(f, "VALIDATION_ERROR"),
            ErrorCode::DatabaseError => write!(f, "DATABASE_ERROR"),
            ErrorCode::RateLimitExceeded => write!(f, "RATE_LIMIT_EXCEEDED"),
        }
    }
}

/// Extended error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorExtension {
    /// Error code
    pub code: ErrorCode,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Create a GraphQL error with an error code
pub fn error_with_code(message: impl Into<String>, code: ErrorCode) -> Error {
    Error::new(message).extend_with(|_, e| {
        e.set("code", code.to_string());
    })
}

/// Create a validation error
pub fn validation_error(message: impl Into<String>, field: Option<&str>) -> Error {
    let mut error = error_with_code(message, ErrorCode::ValidationError);

    if let Some(field) = field {
        error = error.extend_with(|_, e| {
            e.set("field", field);
        });
    }

    error
}

/// Create an unauthorized error
pub fn unauthorized_error(message: impl Into<String>) -> Error {
    error_with_code(message, ErrorCode::Unauthorized)
}

/// Create a forbidden error
pub fn forbidden_error(message: impl Into<String>) -> Error {
    error_with_code(message, ErrorCode::Forbidden)
}

/// Create a not found error
pub fn not_found_error(resource: &str, id: impl fmt::Display) -> Error {
    error_with_code(
        format!("{} with id {} not found", resource, id),
        ErrorCode::NotFound,
    )
}

/// Create a database error
pub fn database_error(message: impl Into<String>) -> Error {
    error_with_code(message, ErrorCode::DatabaseError)
}

/// Result type alias for GraphQL operations
pub type GraphQLResult<T> = async_graphql::Result<T>;

/// Extension trait for Result to add GraphQL error conversion
pub trait ResultExt<T> {
    /// Convert to GraphQL result with error code
    fn to_graphql_result(self, code: ErrorCode) -> GraphQLResult<T>;

    /// Convert to GraphQL result with custom message and code
    fn to_graphql_result_with_message(
        self,
        message: impl Into<String>,
        code: ErrorCode,
    ) -> GraphQLResult<T>;
}

impl<T, E: fmt::Display> ResultExt<T> for Result<T, E> {
    fn to_graphql_result(self, code: ErrorCode) -> GraphQLResult<T> {
        self.map_err(|e| error_with_code(e.to_string(), code))
    }

    fn to_graphql_result_with_message(
        self,
        message: impl Into<String>,
        code: ErrorCode,
    ) -> GraphQLResult<T> {
        self.map_err(|_| error_with_code(message, code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ErrorCode::BadRequest.to_string(), "BAD_REQUEST");
        assert_eq!(ErrorCode::Unauthorized.to_string(), "UNAUTHORIZED");
        assert_eq!(ErrorCode::NotFound.to_string(), "NOT_FOUND");
    }

    #[test]
    fn test_error_with_code() {
        let error = error_with_code("Test error", ErrorCode::BadRequest);
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_validation_error() {
        let error = validation_error("Invalid email", Some("email"));
        assert_eq!(error.message, "Invalid email");
    }

    #[test]
    fn test_not_found_error() {
        let error = not_found_error("User", 42);
        assert_eq!(error.message, "User with id 42 not found");
    }

    #[test]
    fn test_result_ext() {
        let result: Result<i32, &str> = Err("Database error");
        let graphql_result = result.to_graphql_result(ErrorCode::DatabaseError);

        assert!(graphql_result.is_err());
    }

    #[test]
    fn test_result_ext_with_message() {
        let result: Result<i32, &str> = Err("Internal error");
        let graphql_result = result.to_graphql_result_with_message(
            "Custom message",
            ErrorCode::InternalServerError,
        );

        assert!(graphql_result.is_err());
        if let Err(e) = graphql_result {
            assert_eq!(e.message, "Custom message");
        }
    }
}
