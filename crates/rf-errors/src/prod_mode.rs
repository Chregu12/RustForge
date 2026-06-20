//! Production mode error handling
//!
//! Provides secure error responses that don't leak sensitive information
//! while maintaining error tracking and debugging capabilities through logs.

use crate::error::RustForgeError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Production error response
///
/// Generic error response safe for public consumption.
/// Sensitive details are logged separately with error_id correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProdErrorResponse {
    /// Error message (generic, safe for users)
    pub error: ErrorMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Generic error message
    pub message: String,

    /// Error code for client-side handling
    pub code: String,

    /// Request/Error ID for support correlation
    pub request_id: String,

    /// Timestamp
    pub timestamp: String,

    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

/// Production error display
pub struct ProdErrorDisplay<'a> {
    error: &'a RustForgeError,
    include_status: bool,
}

impl<'a> ProdErrorDisplay<'a> {
    /// Create a new production error display
    pub fn new(error: &'a RustForgeError) -> Self {
        Self {
            error,
            include_status: true,
        }
    }

    /// Disable status code in response
    pub fn without_status(mut self) -> Self {
        self.include_status = false;
        self
    }

    /// Format as JSON error response
    pub fn to_json_response(&self) -> ProdErrorResponse {
        let (message, code) = self.get_safe_message_and_code();

        let error_id = self
            .error
            .context()
            .map(|ctx| ctx.error_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        ProdErrorResponse {
            error: ErrorMessage {
                message,
                code,
                request_id: error_id,
                timestamp: chrono::Utc::now().to_rfc3339(),
                status: if self.include_status {
                    Some(self.error.status_code())
                } else {
                    None
                },
            },
        }
    }

    /// Format as HTML error page response
    pub fn to_html_response(&self) -> String {
        let (message, _code) = self.get_safe_message_and_code();
        let status_code = self.error.status_code();

        let error_id = self
            .error
            .context()
            .map(|ctx| ctx.error_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Error {}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
        }}
        .error-container {{
            text-align: center;
            padding: 40px;
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 20px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
            max-width: 500px;
        }}
        .error-code {{
            font-size: 72px;
            font-weight: bold;
            margin: 0;
            opacity: 0.9;
        }}
        .error-message {{
            font-size: 24px;
            margin: 20px 0;
        }}
        .error-description {{
            font-size: 16px;
            opacity: 0.8;
            margin: 20px 0;
        }}
        .request-id {{
            font-size: 12px;
            opacity: 0.6;
            margin-top: 30px;
            font-family: monospace;
        }}
        .back-link {{
            display: inline-block;
            margin-top: 30px;
            padding: 12px 24px;
            background: rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            text-decoration: none;
            color: #fff;
            font-weight: 600;
            transition: all 0.3s;
        }}
        .back-link:hover {{
            background: rgba(255, 255, 255, 0.3);
            transform: translateY(-2px);
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <h1 class="error-code">{}</h1>
        <h2 class="error-message">{}</h2>
        <p class="error-description">{}</p>
        <p class="request-id">Request ID: {}</p>
        <a href="/" class="back-link">Go to Homepage</a>
    </div>
</body>
</html>"#,
            status_code,
            status_code,
            self.get_status_title(status_code),
            message,
            error_id
        )
    }

    /// Get safe message and code for public consumption
    fn get_safe_message_and_code(&self) -> (String, String) {
        let code = self.error.code();

        // Map error types to generic public messages
        let message = match self.error.status_code() {
            400 => "The request was invalid. Please check your input and try again.".to_string(),
            401 => "Authentication is required to access this resource.".to_string(),
            403 => "You don't have permission to access this resource.".to_string(),
            404 => "The requested resource was not found.".to_string(),
            422 => "The provided data failed validation.".to_string(),
            429 => "Too many requests. Please try again later.".to_string(),
            500 => "An unexpected error occurred. Please try again later.".to_string(),
            503 => "The service is temporarily unavailable. Please try again later.".to_string(),
            _ => "An error occurred while processing your request.".to_string(),
        };

        (message, code.code().to_string())
    }

    /// Get title for status code
    fn get_status_title(&self, status: u16) -> &'static str {
        match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            422 => "Validation Failed",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Error",
        }
    }

    /// Log error with full details (for internal use)
    pub fn log_error(&self) {
        let error_id = self
            .error
            .context()
            .map(|ctx| ctx.error_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Log with structured data
        tracing::error!(
            error_id = %error_id,
            error_code = %self.error.code().code(),
            error_type = ?self.error,
            status_code = self.error.status_code(),
            "Production error occurred"
        );

        // Log context if available
        if let Some(ctx) = self.error.context() {
            tracing::debug!(
                error_id = %error_id,
                location = ?ctx.location,
                user_id = ?ctx.user_id,
                request_id = ?ctx.request_id,
                path = ?ctx.path,
                method = ?ctx.method,
                "Error context"
            );
        }
    }
}

impl<'a> fmt::Display for ProdErrorDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let response = self.to_json_response();
        write!(
            f,
            "{} (Request ID: {})",
            response.error.message, response.error.request_id
        )
    }
}

/// Format error for production JSON API response
pub fn format_prod_json(error: &RustForgeError) -> String {
    let display = ProdErrorDisplay::new(error);
    display.log_error();
    serde_json::to_string_pretty(&display.to_json_response())
        .unwrap_or_else(|_| r#"{"error": {"message": "An error occurred"}}"#.to_string())
}

/// Format error for production HTML response
pub fn format_prod_html(error: &RustForgeError) -> String {
    let display = ProdErrorDisplay::new(error);
    display.log_error();
    display.to_html_response()
}

/// Get production error response struct
pub fn get_prod_response(error: &RustForgeError) -> ProdErrorResponse {
    let display = ProdErrorDisplay::new(error);
    display.log_error();
    display.to_json_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::*;

    #[test]
    fn test_prod_display_creation() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let display = ProdErrorDisplay::new(&err);
        assert!(display.include_status);
    }

    #[test]
    fn test_json_response() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        assert!(!response.error.message.is_empty());
        assert!(!response.error.request_id.is_empty());
        assert_eq!(response.error.code, "RF001");
        assert_eq!(response.error.status, Some(500));
    }

    #[test]
    fn test_json_response_no_sensitive_data() {
        let db_err = DatabaseError::connection("secret-host", "secret-db", "secret-user");
        let err = RustForgeError::Database(db_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        // Should NOT contain sensitive connection details
        assert!(!response.error.message.contains("secret-host"));
        assert!(!response.error.message.contains("secret-db"));
        assert!(!response.error.message.contains("secret-user"));
    }

    #[test]
    fn test_html_response() {
        // Use distinctive sentinels: a short name like "db" collides with the
        // random hex Request ID (d and b are hex digits), making the assertion
        // spuriously fail.
        let db_err = DatabaseError::connection("secrethost", "secretdbname", "secretuser");
        let err = RustForgeError::Database(db_err);

        let html = ProdErrorDisplay::new(&err).to_html_response();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("500"));
        assert!(html.contains("Request ID:"));
        // Should not contain sensitive data
        assert!(!html.contains("secrethost"));
        assert!(!html.contains("secretdbname"));
        assert!(!html.contains("secretuser"));
    }

    #[test]
    fn test_validation_error_response() {
        let val_err = ValidationError::new("email", "Invalid format");
        let err = RustForgeError::Validation(val_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        assert_eq!(response.error.status, Some(422));
        assert!(response.error.message.contains("validation"));
    }

    #[test]
    fn test_auth_error_response() {
        let auth_err = AuthenticationError::invalid_credentials();
        let err = RustForgeError::Authentication(auth_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        assert_eq!(response.error.status, Some(401));
        assert!(response.error.message.contains("Authentication"));
    }

    #[test]
    fn test_format_prod_json() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let json = format_prod_json(&err);

        assert!(json.contains("\"error\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"request_id\""));
    }

    #[test]
    fn test_format_prod_html() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let html = format_prod_html(&err);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("500"));
    }

    #[test]
    fn test_404_error() {
        let http_err = HttpError::not_found("User");
        let err = RustForgeError::Http(http_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        assert_eq!(response.error.status, Some(404));
        assert!(response.error.message.contains("not found"));
    }

    #[test]
    fn test_rate_limit_error() {
        let http_err = HttpError::rate_limit_exceeded();
        let err = RustForgeError::Http(http_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        assert_eq!(response.error.status, Some(429));
        assert!(response.error.message.contains("Too many requests"));
    }
}
