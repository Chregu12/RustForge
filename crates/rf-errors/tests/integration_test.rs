//! Integration tests for rf-errors
//!
//! Comprehensive test suite covering all error handling functionality.

use rf_errors::*;

mod error_codes {
    use super::*;

    #[test]
    fn test_all_error_codes_have_unique_values() {
        use std::collections::HashSet;
        let codes = vec![
            ErrorCode::DatabaseConnection,
            ErrorCode::DatabaseQuery,
            ErrorCode::ValidationFailed,
            ErrorCode::AuthenticationFailed,
            ErrorCode::HttpRouteNotFound,
        ];

        let mut seen = HashSet::new();
        for code in codes {
            assert!(
                seen.insert(code.code()),
                "Duplicate error code: {}",
                code.code()
            );
        }
    }

    #[test]
    fn test_error_code_format_is_consistent() {
        let code = ErrorCode::DatabaseConnection;
        assert!(code.code().starts_with("RF"));
        assert_eq!(code.code().len(), 5); // "RFxxx"
    }

    #[test]
    fn test_error_code_docs_url() {
        let code = ErrorCode::DatabaseConnection;
        let url = code.docs_url();
        assert!(url.contains("https://"));
        assert!(url.contains("RF001"));
    }

    #[test]
    fn test_error_code_titles() {
        assert_eq!(
            ErrorCode::DatabaseConnection.title(),
            "Database Connection Failed"
        );
        assert_eq!(ErrorCode::ValidationEmail.title(), "Invalid Email");
        assert_eq!(
            ErrorCode::AuthenticationFailed.title(),
            "Authentication Failed"
        );
    }
}

mod error_context {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let ctx = ErrorContext::new();
        assert!(!ctx.error_id.is_empty());
        assert!(ctx.location.is_none());
        assert!(ctx.values.is_empty());
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
    fn test_error_context_sanitizes_passwords() {
        let ctx = ErrorContext::new()
            .with_value("username", "john")
            .with_value("password", "secret123");

        assert_eq!(
            ctx.values.get("username").unwrap(),
            &serde_json::json!("john")
        );
        assert_eq!(
            ctx.values.get("password").unwrap(),
            &serde_json::json!("***REDACTED***")
        );
    }

    #[test]
    fn test_error_context_sanitizes_nested_values() {
        let user_data = serde_json::json!({
            "id": 123,
            "password": "secret",  // Direct field
            "nested": {
                "api_key": "key123"  // Nested sensitive field
            }
        });

        let ctx = ErrorContext::new().with_value("user", user_data);
        let user = ctx.values.get("user").unwrap();

        assert_eq!(user["password"], "***REDACTED***");
        assert_eq!(user["nested"]["api_key"], "***REDACTED***");
    }

    #[test]
    fn test_error_context_environment_detection() {
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
    fn test_error_context_tags() {
        let ctx = ErrorContext::new()
            .with_tag("database")
            .with_tag("critical");

        assert_eq!(ctx.tags.len(), 2);
        assert!(ctx.tags.contains(&"database".to_string()));
    }
}

mod database_errors {
    use super::*;

    #[test]
    fn test_database_connection_error() {
        let err = error::DatabaseError::connection("localhost:5432", "mydb", "postgres");
        assert_eq!(err.code(), ErrorCode::DatabaseConnection);
        // Check that display contains either "database" or error code
        let display = err.to_string();
        assert!(display.contains("database") || display.contains("RF001"));
    }

    #[test]
    fn test_database_query_error() {
        let err = error::DatabaseError::query("SELECT * FROM users", "syntax error");
        assert_eq!(err.code(), ErrorCode::DatabaseQuery);
    }

    #[test]
    fn test_database_error_into_rustforge_error() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        assert_eq!(err.code(), ErrorCode::DatabaseConnection);
        assert_eq!(err.status_code(), 500);
    }
}

mod validation_errors {
    use super::*;

    #[test]
    fn test_validation_error_creation() {
        let err = error::ValidationError::new("email", "Invalid email format");
        assert_eq!(err.field, "email");
        assert_eq!(err.message, "Invalid email format");
        assert!(err.value.is_none());
    }

    #[test]
    fn test_validation_error_with_value() {
        let err = error::ValidationError::new("email", "Invalid format").with_value("not-an-email");
        assert_eq!(err.value, Some("not-an-email".to_string()));
    }
}

mod friendly_errors {
    use super::*;

    #[test]
    fn test_database_error_friendly_message() {
        let err = error::DatabaseError::connection("localhost", "db", "user");
        let friendly = err.friendly_message();
        assert!(!friendly.is_empty());
        assert!(friendly.contains("database"));
    }

    #[test]
    fn test_database_error_possible_causes() {
        let err = error::DatabaseError::connection("localhost", "db", "user");
        let causes = err.possible_causes();
        assert!(!causes.is_empty());
    }

    #[test]
    fn test_database_error_suggested_fixes() {
        let err = error::DatabaseError::connection("localhost", "db", "user");
        let fixes = err.suggested_fixes();
        assert!(!fixes.is_empty());
    }

    #[test]
    fn test_database_error_current_config() {
        let err = error::DatabaseError::connection("localhost:5432", "mydb", "postgres");
        let config = err.current_config();
        assert!(config.is_some());

        let config = config.unwrap();
        assert!(config
            .iter()
            .any(|(k, v)| k == "Host" && v == "localhost:5432"));
        assert!(config.iter().any(|(k, v)| k == "Database" && v == "mydb"));
    }

    #[test]
    fn test_format_friendly_error() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        let formatted = friendly::format_friendly_error(&err);

        assert!(formatted.contains("Database Connection Failed"));
        assert!(formatted.contains("Possible causes:"));
        assert!(formatted.contains("To fix:"));
    }
}

mod dev_mode_display {
    use super::*;

    #[test]
    fn test_dev_display_creation() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let display = DevErrorDisplay::new(&err);
        assert!(display.format_terminal().contains("RustForge Error"));
    }

    #[test]
    fn test_dev_display_without_backtrace() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let output = DevErrorDisplay::new(&err)
            .without_backtrace()
            .format_terminal();

        assert!(!output.is_empty());
    }

    #[test]
    fn test_format_dev_error() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let formatted = dev_mode::format_dev_error(&err);
        assert!(!formatted.is_empty());
    }
}

mod prod_mode_display {
    use super::*;

    #[test]
    fn test_prod_display_json_response() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        assert!(!response.error.message.is_empty());
        assert!(!response.error.request_id.is_empty());
        assert_eq!(response.error.code, "RF001");
        assert_eq!(response.error.status, Some(500));
    }

    #[test]
    fn test_prod_display_no_sensitive_data() {
        let db_err = error::DatabaseError::connection("secret-host", "secret-db", "secret-user");
        let err = RustForgeError::Database(db_err);

        let response = ProdErrorDisplay::new(&err).to_json_response();

        // Should NOT contain sensitive connection details
        assert!(!response.error.message.contains("secret-host"));
        assert!(!response.error.message.contains("secret-db"));
        assert!(!response.error.message.contains("secret-user"));
    }

    #[test]
    fn test_prod_display_html_response() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let html = ProdErrorDisplay::new(&err).to_html_response();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("500"));
        // Should not contain sensitive data
        assert!(!html.contains("localhost"));
    }

    #[test]
    fn test_format_prod_json() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let json = prod_mode::format_prod_json(&err);

        assert!(json.contains("\"error\""));
        assert!(json.contains("\"message\""));
    }

    #[test]
    fn test_format_prod_html() {
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let html = prod_mode::format_prod_html(&err);

        assert!(html.contains("<!DOCTYPE html>"));
    }
}

mod error_reporting {
    use super::*;

    #[test]
    fn test_error_level_should_report() {
        use reporting::ErrorLevel;

        assert!(ErrorLevel::Critical.should_report_status(500));
        assert!(!ErrorLevel::Critical.should_report_status(404));
        assert!(ErrorLevel::Error.should_report_status(404));
    }

    #[tokio::test]
    async fn test_logging_reporter() {
        use reporting::LoggingReporter;

        let reporter = LoggingReporter::new();
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        let ctx = ErrorContext::new();

        // Should not panic
        reporter.report(&err, &ctx).await;
    }

    #[tokio::test]
    async fn test_multi_reporter() {
        use reporting::{LoggingReporter, MultiReporter};

        let reporter = MultiReporter::new().add_reporter(Box::new(LoggingReporter::new()));

        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        let ctx = ErrorContext::new();

        reporter.report(&err, &ctx).await;
    }
}

#[cfg(feature = "error-pages")]
mod error_pages {
    use super::*;

    #[test]
    fn test_error_pages_creation() {
        let pages = ErrorPages::new();
        let db_err = error::DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let html = pages.render(&err);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("500"));
    }

    #[test]
    fn test_error_pages_custom_page() {
        let pages = ErrorPages::new().set_page(404, "errors/404.blade.php");

        let http_err = error::HttpError::not_found("User");
        let err = RustForgeError::Http(http_err);

        let html = pages.render(&err);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("404"));
    }

    #[test]
    fn test_render_different_status_codes() {
        let pages = ErrorPages::new();

        // 404
        let err = RustForgeError::Http(error::HttpError::not_found("Page"));
        let html = pages.render(&err);
        assert!(html.contains("404"));

        // 500
        let err = RustForgeError::Database(error::DatabaseError::connection("", "", ""));
        let html = pages.render(&err);
        assert!(html.contains("500"));

        // 401
        let err = RustForgeError::Authentication(error::AuthenticationError::invalid_credentials());
        let html = pages.render(&err);
        assert!(html.contains("401"));
    }
}
