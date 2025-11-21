//! Error code definitions
//!
//! All RustForge errors have a unique code (RF001-RF999) for easy documentation
//! and searching.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Error code enumeration
///
/// Each error type has a unique code for documentation and troubleshooting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // Database Errors (RF001-RF099)
    /// RF001: Database connection failed
    DatabaseConnection,
    /// RF002: Database query failed
    DatabaseQuery,
    /// RF003: Database migration failed
    DatabaseMigration,
    /// RF004: Database transaction failed
    DatabaseTransaction,
    /// RF005: Database pool exhausted
    DatabasePoolExhausted,

    // Validation Errors (RF100-RF199)
    /// RF100: Validation failed
    ValidationFailed,
    /// RF101: Field required
    ValidationRequired,
    /// RF102: Invalid email format
    ValidationEmail,
    /// RF103: Invalid length
    ValidationLength,
    /// RF104: Unique constraint violation
    ValidationUnique,
    /// RF105: Foreign key constraint violation
    ValidationExists,
    /// RF106: Invalid format
    ValidationFormat,

    // Authentication Errors (RF200-RF299)
    /// RF200: Authentication failed
    AuthenticationFailed,
    /// RF201: Invalid credentials
    AuthInvalidCredentials,
    /// RF202: Token expired
    AuthTokenExpired,
    /// RF203: Token invalid
    AuthTokenInvalid,
    /// RF204: User not found
    AuthUserNotFound,
    /// RF205: Account locked
    AuthAccountLocked,
    /// RF206: Email not verified
    AuthEmailNotVerified,

    // Authorization Errors (RF300-RF399)
    /// RF300: Access forbidden
    AuthorizationForbidden,
    /// RF301: Insufficient permissions
    AuthInsufficientPermissions,
    /// RF302: Role required
    AuthRoleRequired,
    /// RF303: Policy violation
    AuthPolicyViolation,

    // Cache Errors (RF400-RF499)
    /// RF400: Cache connection failed
    CacheConnection,
    /// RF401: Cache operation failed
    CacheOperation,
    /// RF402: Cache serialization failed
    CacheSerialization,

    // Queue Errors (RF500-RF599)
    /// RF500: Queue connection failed
    QueueConnection,
    /// RF501: Queue dispatch failed
    QueueDispatch,
    /// RF502: Job failed
    QueueJobFailed,
    /// RF503: Job timeout
    QueueJobTimeout,

    // HTTP Errors (RF600-RF699)
    /// RF600: HTTP request failed
    HttpRequestFailed,
    /// RF601: Invalid request body
    HttpInvalidBody,
    /// RF602: Route not found
    HttpRouteNotFound,
    /// RF603: Method not allowed
    HttpMethodNotAllowed,
    /// RF604: Rate limit exceeded
    HttpRateLimitExceeded,

    // Template Errors (RF700-RF799)
    /// RF700: Template not found
    TemplateNotFound,
    /// RF701: Template rendering failed
    TemplateRenderingFailed,
    /// RF702: Template compilation failed
    TemplateCompilationFailed,

    // Storage Errors (RF800-RF899)
    /// RF800: Storage connection failed
    StorageConnection,
    /// RF801: File not found
    StorageFileNotFound,
    /// RF802: File upload failed
    StorageUploadFailed,
    /// RF803: File download failed
    StorageDownloadFailed,
    /// RF804: Disk quota exceeded
    StorageDiskQuotaExceeded,

    // Mail Errors (RF850-RF899)
    /// RF850: Mail server connection failed
    MailConnection,
    /// RF851: Mail send failed
    MailSendFailed,

    // General Errors (RF900-RF999)
    /// RF900: Configuration error
    ConfigurationError,
    /// RF901: Environment variable missing
    EnvVarMissing,
    /// RF902: Internal server error
    InternalError,
    /// RF903: Service unavailable
    ServiceUnavailable,
    /// RF904: Timeout
    Timeout,
}

impl ErrorCode {
    /// Get the numeric code (e.g., "RF001")
    pub fn code(&self) -> &'static str {
        match self {
            // Database (RF001-RF099)
            Self::DatabaseConnection => "RF001",
            Self::DatabaseQuery => "RF002",
            Self::DatabaseMigration => "RF003",
            Self::DatabaseTransaction => "RF004",
            Self::DatabasePoolExhausted => "RF005",

            // Validation (RF100-RF199)
            Self::ValidationFailed => "RF100",
            Self::ValidationRequired => "RF101",
            Self::ValidationEmail => "RF102",
            Self::ValidationLength => "RF103",
            Self::ValidationUnique => "RF104",
            Self::ValidationExists => "RF105",
            Self::ValidationFormat => "RF106",

            // Authentication (RF200-RF299)
            Self::AuthenticationFailed => "RF200",
            Self::AuthInvalidCredentials => "RF201",
            Self::AuthTokenExpired => "RF202",
            Self::AuthTokenInvalid => "RF203",
            Self::AuthUserNotFound => "RF204",
            Self::AuthAccountLocked => "RF205",
            Self::AuthEmailNotVerified => "RF206",

            // Authorization (RF300-RF399)
            Self::AuthorizationForbidden => "RF300",
            Self::AuthInsufficientPermissions => "RF301",
            Self::AuthRoleRequired => "RF302",
            Self::AuthPolicyViolation => "RF303",

            // Cache (RF400-RF499)
            Self::CacheConnection => "RF400",
            Self::CacheOperation => "RF401",
            Self::CacheSerialization => "RF402",

            // Queue (RF500-RF599)
            Self::QueueConnection => "RF500",
            Self::QueueDispatch => "RF501",
            Self::QueueJobFailed => "RF502",
            Self::QueueJobTimeout => "RF503",

            // HTTP (RF600-RF699)
            Self::HttpRequestFailed => "RF600",
            Self::HttpInvalidBody => "RF601",
            Self::HttpRouteNotFound => "RF602",
            Self::HttpMethodNotAllowed => "RF603",
            Self::HttpRateLimitExceeded => "RF604",

            // Template (RF700-RF799)
            Self::TemplateNotFound => "RF700",
            Self::TemplateRenderingFailed => "RF701",
            Self::TemplateCompilationFailed => "RF702",

            // Storage (RF800-RF899)
            Self::StorageConnection => "RF800",
            Self::StorageFileNotFound => "RF801",
            Self::StorageUploadFailed => "RF802",
            Self::StorageDownloadFailed => "RF803",
            Self::StorageDiskQuotaExceeded => "RF804",

            // Mail (RF850-RF899)
            Self::MailConnection => "RF850",
            Self::MailSendFailed => "RF851",

            // General (RF900-RF999)
            Self::ConfigurationError => "RF900",
            Self::EnvVarMissing => "RF901",
            Self::InternalError => "RF902",
            Self::ServiceUnavailable => "RF903",
            Self::Timeout => "RF904",
        }
    }

    /// Get the documentation URL for this error
    pub fn docs_url(&self) -> String {
        format!("https://docs.rustforge.dev/errors/{}", self.code())
    }

    /// Get a human-readable title for this error
    pub fn title(&self) -> &'static str {
        match self {
            // Database
            Self::DatabaseConnection => "Database Connection Failed",
            Self::DatabaseQuery => "Database Query Failed",
            Self::DatabaseMigration => "Database Migration Failed",
            Self::DatabaseTransaction => "Database Transaction Failed",
            Self::DatabasePoolExhausted => "Database Pool Exhausted",

            // Validation
            Self::ValidationFailed => "Validation Failed",
            Self::ValidationRequired => "Field Required",
            Self::ValidationEmail => "Invalid Email",
            Self::ValidationLength => "Invalid Length",
            Self::ValidationUnique => "Value Already Exists",
            Self::ValidationExists => "Referenced Entity Not Found",
            Self::ValidationFormat => "Invalid Format",

            // Authentication
            Self::AuthenticationFailed => "Authentication Failed",
            Self::AuthInvalidCredentials => "Invalid Credentials",
            Self::AuthTokenExpired => "Token Expired",
            Self::AuthTokenInvalid => "Invalid Token",
            Self::AuthUserNotFound => "User Not Found",
            Self::AuthAccountLocked => "Account Locked",
            Self::AuthEmailNotVerified => "Email Not Verified",

            // Authorization
            Self::AuthorizationForbidden => "Access Forbidden",
            Self::AuthInsufficientPermissions => "Insufficient Permissions",
            Self::AuthRoleRequired => "Role Required",
            Self::AuthPolicyViolation => "Policy Violation",

            // Cache
            Self::CacheConnection => "Cache Connection Failed",
            Self::CacheOperation => "Cache Operation Failed",
            Self::CacheSerialization => "Cache Serialization Failed",

            // Queue
            Self::QueueConnection => "Queue Connection Failed",
            Self::QueueDispatch => "Job Dispatch Failed",
            Self::QueueJobFailed => "Job Failed",
            Self::QueueJobTimeout => "Job Timeout",

            // HTTP
            Self::HttpRequestFailed => "HTTP Request Failed",
            Self::HttpInvalidBody => "Invalid Request Body",
            Self::HttpRouteNotFound => "Route Not Found",
            Self::HttpMethodNotAllowed => "Method Not Allowed",
            Self::HttpRateLimitExceeded => "Rate Limit Exceeded",

            // Template
            Self::TemplateNotFound => "Template Not Found",
            Self::TemplateRenderingFailed => "Template Rendering Failed",
            Self::TemplateCompilationFailed => "Template Compilation Failed",

            // Storage
            Self::StorageConnection => "Storage Connection Failed",
            Self::StorageFileNotFound => "File Not Found",
            Self::StorageUploadFailed => "File Upload Failed",
            Self::StorageDownloadFailed => "File Download Failed",
            Self::StorageDiskQuotaExceeded => "Disk Quota Exceeded",

            // Mail
            Self::MailConnection => "Mail Server Connection Failed",
            Self::MailSendFailed => "Failed to Send Email",

            // General
            Self::ConfigurationError => "Configuration Error",
            Self::EnvVarMissing => "Environment Variable Missing",
            Self::InternalError => "Internal Server Error",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::Timeout => "Operation Timeout",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_format() {
        assert_eq!(ErrorCode::DatabaseConnection.code(), "RF001");
        assert_eq!(ErrorCode::ValidationFailed.code(), "RF100");
        assert_eq!(ErrorCode::AuthenticationFailed.code(), "RF200");
    }

    #[test]
    fn test_error_code_title() {
        assert_eq!(
            ErrorCode::DatabaseConnection.title(),
            "Database Connection Failed"
        );
        assert_eq!(ErrorCode::ValidationEmail.title(), "Invalid Email");
    }

    #[test]
    fn test_docs_url() {
        assert_eq!(
            ErrorCode::DatabaseConnection.docs_url(),
            "https://docs.rustforge.dev/errors/RF001"
        );
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::DatabaseConnection.to_string(), "RF001");
    }
}
