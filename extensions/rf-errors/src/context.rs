//! Error context tracking
//!
//! Provides context information for errors including file location,
//! variable values, and error metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Error location information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLocation {
    /// File path where the error occurred
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Function or method name
    pub function: Option<String>,
}

impl ErrorLocation {
    /// Create a new error location
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            function: None,
        }
    }

    /// Add function name
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }
}

impl fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref func) = self.function {
            write!(f, "{}:{} in {}", self.file, self.line, func)
        } else {
            write!(f, "{}:{}", self.file, self.line)
        }
    }
}

/// Macro to create error location from current position
#[macro_export]
macro_rules! error_location {
    () => {
        $crate::context::ErrorLocation::new(file!(), line!(), column!())
    };
    ($func:expr) => {
        $crate::context::ErrorLocation::new(file!(), line!(), column!()).with_function($func)
    };
}

/// Error context information
///
/// Contains metadata about an error including location, user context,
/// request information, and custom values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Unique error ID for correlation
    pub error_id: String,

    /// Error location
    pub location: Option<ErrorLocation>,

    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Request ID if this error occurred during a request
    pub request_id: Option<String>,

    /// User ID if user is authenticated
    pub user_id: Option<String>,

    /// Request path
    pub path: Option<String>,

    /// HTTP method
    pub method: Option<String>,

    /// Environment (development, production, etc.)
    pub environment: String,

    /// Custom context values (sanitized)
    pub values: HashMap<String, serde_json::Value>,

    /// Tags for categorization
    pub tags: Vec<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self {
            error_id: Uuid::new_v4().to_string(),
            location: None,
            timestamp: chrono::Utc::now(),
            request_id: None,
            user_id: None,
            path: None,
            method: None,
            environment: std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
            values: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Set error location
    pub fn with_location(mut self, location: ErrorLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set request path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set HTTP method
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Add a context value (will be sanitized)
    pub fn with_value(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        let key = key.into();

        // Sanitize sensitive fields
        if Self::is_sensitive_key(&key) {
            self.values
                .insert(key, serde_json::Value::String("***REDACTED***".to_string()));
        } else if let Ok(json_value) = serde_json::to_value(value) {
            self.values.insert(key, Self::sanitize_value(json_value));
        }

        self
    }

    /// Add multiple context values
    pub fn with_values(mut self, values: HashMap<String, serde_json::Value>) -> Self {
        for (key, value) in values {
            self = self.with_value(key, value);
        }
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment == "development" || self.environment == "local"
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Check if a key contains sensitive information
    fn is_sensitive_key(key: &str) -> bool {
        let sensitive_keywords = [
            "password",
            "passwd",
            "pwd",
            "secret",
            "token",
            "key",
            "api_key",
            "apikey",
            "auth",
            "credential",
            "private",
            "session",
            "cookie",
            "ssn",
            "credit_card",
            "cvv",
        ];

        let key_lower = key.to_lowercase();
        sensitive_keywords
            .iter()
            .any(|keyword| key_lower.contains(keyword))
    }

    /// Sanitize a JSON value by redacting sensitive fields
    fn sanitize_value(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let sanitized: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .map(|(k, v)| {
                        if Self::is_sensitive_key(&k) {
                            (k, serde_json::Value::String("***REDACTED***".to_string()))
                        } else {
                            (k, Self::sanitize_value(v))
                        }
                    })
                    .collect();
                serde_json::Value::Object(sanitized)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::sanitize_value).collect())
            }
            other => other,
        }
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_location_creation() {
        let loc = ErrorLocation::new("src/main.rs", 42, 10);
        assert_eq!(loc.file, "src/main.rs");
        assert_eq!(loc.line, 42);
        assert_eq!(loc.column, 10);
        assert!(loc.function.is_none());
    }

    #[test]
    fn test_error_location_with_function() {
        let loc = ErrorLocation::new("src/main.rs", 42, 10).with_function("handle_request");
        assert_eq!(loc.function, Some("handle_request".to_string()));
    }

    #[test]
    fn test_error_location_display() {
        let loc = ErrorLocation::new("src/main.rs", 42, 10);
        assert_eq!(loc.to_string(), "src/main.rs:42");

        let loc_with_func = loc.with_function("main");
        assert_eq!(loc_with_func.to_string(), "src/main.rs:42 in main");
    }

    #[test]
    fn test_error_context_creation() {
        let ctx = ErrorContext::new();
        assert!(!ctx.error_id.is_empty());
        assert!(ctx.location.is_none());
        assert!(ctx.request_id.is_none());
    }

    #[test]
    fn test_error_context_with_location() {
        let loc = ErrorLocation::new("src/main.rs", 42, 10);
        let ctx = ErrorContext::new().with_location(loc.clone());
        assert_eq!(ctx.location, Some(loc));
    }

    #[test]
    fn test_error_context_with_request_info() {
        let ctx = ErrorContext::new()
            .with_request_id("req_123")
            .with_path("/api/users")
            .with_method("GET")
            .with_user_id("user_456");

        assert_eq!(ctx.request_id, Some("req_123".to_string()));
        assert_eq!(ctx.path, Some("/api/users".to_string()));
        assert_eq!(ctx.method, Some("GET".to_string()));
        assert_eq!(ctx.user_id, Some("user_456".to_string()));
    }

    #[test]
    fn test_sensitive_key_detection() {
        assert!(ErrorContext::is_sensitive_key("password"));
        assert!(ErrorContext::is_sensitive_key("api_key"));
        assert!(ErrorContext::is_sensitive_key("secret_token"));
        assert!(ErrorContext::is_sensitive_key("user_password"));
        assert!(!ErrorContext::is_sensitive_key("username"));
        assert!(!ErrorContext::is_sensitive_key("email"));
    }

    #[test]
    fn test_value_sanitization() {
        let mut values = HashMap::new();
        values.insert("username".to_string(), serde_json::json!("john"));
        values.insert("password".to_string(), serde_json::json!("secret123"));

        let ctx = ErrorContext::new().with_values(values);

        // Username should be present
        assert_eq!(
            ctx.values.get("username").unwrap(),
            &serde_json::json!("john")
        );

        // Password should be redacted
        assert_eq!(
            ctx.values.get("password").unwrap(),
            &serde_json::json!("***REDACTED***")
        );
    }

    #[test]
    fn test_nested_value_sanitization() {
        // Test that nested sensitive values are properly sanitized
        let user_data = serde_json::json!({
            "id": 123,
            "email": "user@example.com",
            "password": "secret123",  // Direct field
            "nested": {
                "api_key": "key_abc"  // Nested sensitive field
            }
        });

        let ctx = ErrorContext::new().with_value("user", user_data);

        let user = ctx.values.get("user").unwrap();
        assert_eq!(user["id"], 123);
        assert_eq!(user["email"], "user@example.com");

        // Direct password should be redacted
        assert_eq!(user["password"], "***REDACTED***");

        // Nested api_key should also be redacted
        assert_eq!(user["nested"]["api_key"], "***REDACTED***");
    }

    #[test]
    fn test_environment_detection() {
        std::env::set_var("APP_ENV", "development");
        let ctx = ErrorContext::new();
        assert!(ctx.is_development());
        assert!(!ctx.is_production());

        std::env::set_var("APP_ENV", "production");
        let ctx = ErrorContext::new();
        assert!(!ctx.is_development());
        assert!(ctx.is_production());

        std::env::remove_var("APP_ENV");
    }

    #[test]
    fn test_tags() {
        let ctx = ErrorContext::new()
            .with_tag("database")
            .with_tag("critical");

        assert_eq!(ctx.tags.len(), 2);
        assert!(ctx.tags.contains(&"database".to_string()));
        assert!(ctx.tags.contains(&"critical".to_string()));
    }

    #[test]
    fn test_error_location_macro() {
        let loc = error_location!();
        assert!(loc.file.contains("context.rs"));
        assert!(loc.line > 0);
    }
}
