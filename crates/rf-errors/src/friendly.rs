//! Friendly error messages
//!
//! Generates user-friendly, actionable error messages with helpful context
//! and troubleshooting steps.

use crate::code::ErrorCode;
use crate::error::*;
use std::fmt;

/// Friendly error message trait
pub trait FriendlyError {
    /// Get a user-friendly description
    fn friendly_message(&self) -> String;

    /// Get possible causes
    fn possible_causes(&self) -> Vec<String>;

    /// Get suggested fixes
    fn suggested_fixes(&self) -> Vec<String>;

    /// Get documentation URL
    fn docs_url(&self) -> Option<String>;

    /// Get current configuration (if applicable)
    fn current_config(&self) -> Option<Vec<(String, String)>>;
}

impl FriendlyError for DatabaseError {
    fn friendly_message(&self) -> String {
        match &self.kind {
            DatabaseErrorKind::Connection { .. } => {
                "The application couldn't connect to the database.".to_string()
            }
            DatabaseErrorKind::Query { .. } => {
                "A database query failed to execute.".to_string()
            }
            DatabaseErrorKind::Migration { .. } => {
                "Database migration failed.".to_string()
            }
            DatabaseErrorKind::Transaction { .. } => {
                "Database transaction failed.".to_string()
            }
            DatabaseErrorKind::PoolExhausted { .. } => {
                "All database connections are in use.".to_string()
            }
        }
    }

    fn possible_causes(&self) -> Vec<String> {
        match &self.kind {
            DatabaseErrorKind::Connection { .. } => vec![
                "Database server is not running".to_string(),
                "Incorrect credentials in .env file".to_string(),
                "Network/firewall blocking connection".to_string(),
                "Database name doesn't exist".to_string(),
            ],
            DatabaseErrorKind::Query { .. } => vec![
                "SQL syntax error".to_string(),
                "Table or column doesn't exist".to_string(),
                "Type mismatch in query parameters".to_string(),
                "Constraint violation".to_string(),
            ],
            DatabaseErrorKind::Migration { .. } => vec![
                "Migration file is corrupt or invalid".to_string(),
                "Database schema conflicts".to_string(),
                "Insufficient permissions".to_string(),
            ],
            DatabaseErrorKind::Transaction { .. } => vec![
                "Deadlock detected".to_string(),
                "Constraint violation".to_string(),
                "Connection lost during transaction".to_string(),
            ],
            DatabaseErrorKind::PoolExhausted { .. } => vec![
                "Too many concurrent requests".to_string(),
                "Connections not being properly released".to_string(),
                "Pool size too small for load".to_string(),
            ],
        }
    }

    fn suggested_fixes(&self) -> Vec<String> {
        match &self.kind {
            DatabaseErrorKind::Connection { host, database, .. } => vec![
                format!("Check if database server is running: systemctl status postgresql"),
                format!("Verify DATABASE_URL in .env file"),
                format!("Test connection: psql -h {} -d {}", host, database),
                format!("Check firewall rules allow connection to {}", host),
            ],
            DatabaseErrorKind::Query { query, .. } => vec![
                "Review the SQL query syntax".to_string(),
                "Check table and column names".to_string(),
                format!("Run query manually: {}", query),
                "Enable query logging for more details".to_string(),
            ],
            DatabaseErrorKind::Migration { version, .. } => vec![
                format!("Review migration file for version {}", version),
                "Check database schema state".to_string(),
                "Try rolling back last migration".to_string(),
                "Run migrations in fresh database to test".to_string(),
            ],
            DatabaseErrorKind::Transaction { .. } => vec![
                "Retry the operation".to_string(),
                "Check for deadlock conditions".to_string(),
                "Review transaction isolation level".to_string(),
            ],
            DatabaseErrorKind::PoolExhausted { max_connections } => vec![
                format!("Increase pool size in config (current: {})", max_connections),
                "Review code for connection leaks".to_string(),
                "Add connection timeout handling".to_string(),
                "Scale database server if needed".to_string(),
            ],
        }
    }

    fn docs_url(&self) -> Option<String> {
        Some(self.code().docs_url())
    }

    fn current_config(&self) -> Option<Vec<(String, String)>> {
        match &self.kind {
            DatabaseErrorKind::Connection { host, database, user } => Some(vec![
                ("Host".to_string(), host.clone()),
                ("Database".to_string(), database.clone()),
                ("User".to_string(), user.clone()),
            ]),
            DatabaseErrorKind::PoolExhausted { max_connections } => Some(vec![
                ("Max Connections".to_string(), max_connections.to_string()),
            ]),
            _ => None,
        }
    }
}

impl FriendlyError for ValidationError {
    fn friendly_message(&self) -> String {
        format!("The field '{}' failed validation.", self.field)
    }

    fn possible_causes(&self) -> Vec<String> {
        vec![
            "Invalid input format".to_string(),
            "Required field is missing".to_string(),
            "Value doesn't meet constraints".to_string(),
        ]
    }

    fn suggested_fixes(&self) -> Vec<String> {
        vec![
            format!("Check the format of '{}'", self.field),
            "Review validation rules".to_string(),
            if let Some(ref val) = self.value {
                format!("Current value '{}' is invalid", val)
            } else {
                "Provide a value for this field".to_string()
            },
        ]
    }

    fn docs_url(&self) -> Option<String> {
        Some(self.code().docs_url())
    }

    fn current_config(&self) -> Option<Vec<(String, String)>> {
        if let Some(ref val) = self.value {
            Some(vec![("Current Value".to_string(), val.clone())])
        } else {
            None
        }
    }
}

impl FriendlyError for AuthenticationError {
    fn friendly_message(&self) -> String {
        match self.kind {
            AuthenticationErrorKind::InvalidCredentials => {
                "The username or password is incorrect.".to_string()
            }
            AuthenticationErrorKind::TokenExpired => {
                "Your session has expired. Please log in again.".to_string()
            }
            AuthenticationErrorKind::TokenInvalid => {
                "The authentication token is invalid.".to_string()
            }
            AuthenticationErrorKind::UserNotFound => {
                "User account not found.".to_string()
            }
            AuthenticationErrorKind::AccountLocked => {
                "This account has been locked.".to_string()
            }
            AuthenticationErrorKind::EmailNotVerified => {
                "Please verify your email address before logging in.".to_string()
            }
        }
    }

    fn possible_causes(&self) -> Vec<String> {
        match self.kind {
            AuthenticationErrorKind::InvalidCredentials => vec![
                "Wrong username or email".to_string(),
                "Wrong password".to_string(),
                "Account doesn't exist".to_string(),
            ],
            AuthenticationErrorKind::TokenExpired => vec![
                "Session timeout".to_string(),
                "Token lifetime exceeded".to_string(),
            ],
            AuthenticationErrorKind::AccountLocked => vec![
                "Too many failed login attempts".to_string(),
                "Account manually locked by admin".to_string(),
                "Suspicious activity detected".to_string(),
            ],
            _ => vec![],
        }
    }

    fn suggested_fixes(&self) -> Vec<String> {
        match self.kind {
            AuthenticationErrorKind::InvalidCredentials => vec![
                "Double-check your credentials".to_string(),
                "Try password reset if needed".to_string(),
            ],
            AuthenticationErrorKind::TokenExpired => vec![
                "Log in again to get a new session".to_string(),
            ],
            AuthenticationErrorKind::EmailNotVerified => vec![
                "Check your email for verification link".to_string(),
                "Resend verification email".to_string(),
            ],
            AuthenticationErrorKind::AccountLocked => vec![
                "Contact support to unlock account".to_string(),
                "Wait 30 minutes and try again".to_string(),
            ],
            _ => vec![],
        }
    }

    fn docs_url(&self) -> Option<String> {
        Some(self.code().docs_url())
    }

    fn current_config(&self) -> Option<Vec<(String, String)>> {
        None
    }
}

impl FriendlyError for RustForgeError {
    fn friendly_message(&self) -> String {
        match self {
            Self::Database(e) => e.friendly_message(),
            Self::Validation(e) => e.friendly_message(),
            Self::Authentication(e) => e.friendly_message(),
            Self::Authorization(e) => format!("Access forbidden: {}", e.reason),
            Self::Cache(e) => format!("Cache operation failed: {}", e.message),
            Self::Queue(e) => format!("Queue operation failed: {}", e.message),
            Self::Http(e) => e.message.clone(),
            Self::Template(e) => format!("Template error: {}", e.message),
            Self::Storage(e) => format!("Storage error: {}", e.message),
            Self::Mail(e) => format!("Mail operation failed: {}", e.message),
            Self::Configuration(e) => e.message.clone(),
            Self::Internal(e) => format!("Internal error: {}", e),
        }
    }

    fn possible_causes(&self) -> Vec<String> {
        match self {
            Self::Database(e) => e.possible_causes(),
            Self::Validation(e) => e.possible_causes(),
            Self::Authentication(e) => e.possible_causes(),
            _ => vec![],
        }
    }

    fn suggested_fixes(&self) -> Vec<String> {
        match self {
            Self::Database(e) => e.suggested_fixes(),
            Self::Validation(e) => e.suggested_fixes(),
            Self::Authentication(e) => e.suggested_fixes(),
            _ => vec![],
        }
    }

    fn docs_url(&self) -> Option<String> {
        Some(self.code().docs_url())
    }

    fn current_config(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::Database(e) => e.current_config(),
            Self::Validation(e) => e.current_config(),
            _ => None,
        }
    }
}

/// Format a friendly error message
pub fn format_friendly_error(error: &RustForgeError) -> String {
    let mut output = String::new();

    // Title
    output.push_str(&format!("\n{}\n", error.code().title()));
    output.push_str(&format!("\n{}\n", error.friendly_message()));

    // Possible causes
    let causes = error.possible_causes();
    if !causes.is_empty() {
        output.push_str("\nPossible causes:\n");
        for cause in causes {
            output.push_str(&format!("  • {}\n", cause));
        }
    }

    // Current configuration
    if let Some(config) = error.current_config() {
        output.push_str("\nCurrent configuration:\n");
        for (key, value) in config {
            output.push_str(&format!("  • {}: {}\n", key, value));
        }
    }

    // Suggested fixes
    let fixes = error.suggested_fixes();
    if !fixes.is_empty() {
        output.push_str("\nTo fix:\n");
        for (i, fix) in fixes.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", i + 1, fix));
        }
    }

    // Documentation link
    if let Some(url) = error.docs_url() {
        output.push_str(&format!("\nFor more help: {}\n", url));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_friendly_message() {
        let err = DatabaseError::connection("localhost:5432", "mydb", "postgres");
        assert!(err.friendly_message().contains("couldn't connect"));
    }

    #[test]
    fn test_database_error_possible_causes() {
        let err = DatabaseError::connection("localhost", "db", "user");
        let causes = err.possible_causes();
        assert!(!causes.is_empty());
        assert!(causes.iter().any(|c| c.contains("not running")));
    }

    #[test]
    fn test_database_error_suggested_fixes() {
        let err = DatabaseError::connection("localhost", "db", "user");
        let fixes = err.suggested_fixes();
        assert!(!fixes.is_empty());
        assert!(fixes.iter().any(|f| f.contains("systemctl") || f.contains("psql")));
    }

    #[test]
    fn test_validation_error_friendly() {
        let err = ValidationError::new("email", "Invalid format")
            .with_value("not-an-email");
        assert!(err.friendly_message().contains("email"));
        assert!(err.suggested_fixes().iter().any(|f| f.contains("not-an-email")));
    }

    #[test]
    fn test_format_friendly_error() {
        let db_err = DatabaseError::connection("localhost:5432", "rustforge_dev", "postgres");
        let err = RustForgeError::Database(db_err);

        let formatted = format_friendly_error(&err);

        assert!(formatted.contains("Database Connection Failed"));
        assert!(formatted.contains("Possible causes:"));
        assert!(formatted.contains("To fix:"));
        assert!(formatted.contains("https://docs.rustforge.dev"));
    }

    #[test]
    fn test_auth_error_friendly() {
        let err = AuthenticationError::invalid_credentials();
        assert!(err.friendly_message().contains("incorrect"));
        assert!(!err.possible_causes().is_empty());
        assert!(!err.suggested_fixes().is_empty());
    }
}
