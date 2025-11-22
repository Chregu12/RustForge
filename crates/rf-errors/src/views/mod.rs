//! Error page views
//!
//! Provides customizable error pages for different HTTP status codes.

use crate::context::ErrorContext;
use crate::error::RustForgeError;
use std::collections::HashMap;

/// Error pages handler
pub struct ErrorPages {
    environment: String,
    custom_pages: HashMap<u16, String>,
}

impl ErrorPages {
    /// Create a new error pages handler
    pub fn new() -> Self {
        Self {
            environment: std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
            custom_pages: HashMap::new(),
        }
    }

    /// Set custom error page template for a status code
    pub fn set_page(mut self, status_code: u16, template_path: impl Into<String>) -> Self {
        self.custom_pages.insert(status_code, template_path.into());
        self
    }

    /// Render error page for the given error
    pub fn render(&self, error: &RustForgeError) -> String {
        let status_code = error.status_code();

        // Check if custom page exists
        if let Some(template_path) = self.custom_pages.get(&status_code) {
            // In a real implementation, this would render the Blade template
            return self.render_custom_template(template_path, error);
        }

        // Use default error page
        self.render_default_page(error)
    }

    /// Render custom template (placeholder - would use rf-blade in real impl)
    fn render_custom_template(&self, _template_path: &str, error: &RustForgeError) -> String {
        // This would integrate with rf-blade when feature is enabled
        self.render_default_page(error)
    }

    /// Render default error page
    fn render_default_page(&self, error: &RustForgeError) -> String {
        let status_code = error.status_code();
        let is_dev = self.environment == "development" || self.environment == "local";

        if is_dev {
            self.render_dev_page(error)
        } else {
            self.render_prod_page(error)
        }
    }

    /// Render development error page
    fn render_dev_page(&self, error: &RustForgeError) -> String {
        let status_code = error.status_code();
        let title = get_status_title(status_code);
        let error_id = error
            .context()
            .map(|c| c.error_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Error {status_code} - {title}</title>
    <style>
        {dev_styles}
    </style>
</head>
<body>
    <div class="error-container">
        <div class="error-header">
            <div class="error-code">{status_code}</div>
            <div class="error-title">{title}</div>
        </div>

        <div class="error-details">
            <div class="section">
                <h3>Error Details</h3>
                <div class="detail-item">
                    <span class="label">Error ID:</span>
                    <span class="value">{error_id}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Error Code:</span>
                    <span class="value">{error_code}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Message:</span>
                    <span class="value">{message}</span>
                </div>
            </div>

            {context_section}

            <div class="section">
                <h3>Stack Trace</h3>
                <pre class="stack-trace">{stack_trace}</pre>
            </div>
        </div>

        <div class="footer">
            <a href="/" class="btn">← Go to Homepage</a>
        </div>
    </div>
</body>
</html>"#,
            status_code = status_code,
            title = title,
            error_id = error_id,
            error_code = error.code().code(),
            message = html_escape(&error.to_string()),
            context_section = self.render_context_section(error),
            stack_trace = "Enable RUST_BACKTRACE=1 for full trace",
            dev_styles = include_str!("styles/dev.css"),
        )
    }

    /// Render production error page
    fn render_prod_page(&self, error: &RustForgeError) -> String {
        let status_code = error.status_code();
        let title = get_status_title(status_code);
        let message = get_user_friendly_message(status_code);
        let error_id = error
            .context()
            .map(|c| c.error_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Error {status_code} - {title}</title>
    <style>
        {prod_styles}
    </style>
</head>
<body>
    <div class="error-container">
        <div class="error-code">{status_code}</div>
        <h1 class="error-title">{title}</h1>
        <p class="error-message">{message}</p>
        <p class="request-id">Request ID: {error_id}</p>
        <div class="actions">
            <a href="/" class="btn btn-primary">Go to Homepage</a>
            <a href="javascript:history.back()" class="btn btn-secondary">Go Back</a>
        </div>
    </div>
</body>
</html>"#,
            status_code = status_code,
            title = title,
            message = message,
            error_id = error_id,
            prod_styles = include_str!("styles/prod.css"),
        )
    }

    /// Render context section for dev mode
    fn render_context_section(&self, error: &RustForgeError) -> String {
        let Some(ctx) = error.context() else {
            return String::new();
        };

        let mut html = String::from(r#"<div class="section"><h3>Request Context</h3>"#);

        if let Some(ref path) = ctx.path {
            html.push_str(&format!(
                r#"<div class="detail-item"><span class="label">Path:</span><span class="value">{}</span></div>"#,
                html_escape(path)
            ));
        }

        if let Some(ref method) = ctx.method {
            html.push_str(&format!(
                r#"<div class="detail-item"><span class="label">Method:</span><span class="value">{}</span></div>"#,
                html_escape(method)
            ));
        }

        if let Some(ref user_id) = ctx.user_id {
            html.push_str(&format!(
                r#"<div class="detail-item"><span class="label">User ID:</span><span class="value">{}</span></div>"#,
                html_escape(user_id)
            ));
        }

        html.push_str("</div>");
        html
    }
}

impl Default for ErrorPages {
    fn default() -> Self {
        Self::new()
    }
}

/// Get title for HTTP status code
fn get_status_title(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    }
}

/// Get user-friendly message for status code
fn get_user_friendly_message(status: u16) -> &'static str {
    match status {
        400 => "The request could not be understood by the server. Please check your input.",
        401 => "You need to be authenticated to access this resource.",
        403 => "You don't have permission to access this resource.",
        404 => "The page you're looking for doesn't exist. It may have been moved or deleted.",
        405 => "This method is not allowed for the requested resource.",
        408 => "The request took too long to process. Please try again.",
        422 => "The data you provided couldn't be processed. Please check your input.",
        429 => "You've made too many requests. Please slow down and try again later.",
        500 => "Something went wrong on our end. We're working to fix it.",
        502 => "We couldn't reach our servers. Please try again in a moment.",
        503 => "The service is temporarily unavailable. Please try again later.",
        504 => "The server took too long to respond. Please try again.",
        _ => "An error occurred while processing your request.",
    }
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::*;

    #[test]
    fn test_error_pages_creation() {
        let pages = ErrorPages::new();
        assert!(pages.custom_pages.is_empty());
    }

    #[test]
    fn test_set_custom_page() {
        let pages = ErrorPages::new().set_page(404, "errors/404.blade.php");

        assert_eq!(
            pages.custom_pages.get(&404),
            Some(&"errors/404.blade.php".to_string())
        );
    }

    #[test]
    fn test_render_404() {
        let pages = ErrorPages::new();
        let err = HttpError::not_found("User");
        let rf_err = RustForgeError::Http(err);

        let html = pages.render(&rf_err);

        assert!(html.contains("404"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_render_500() {
        let pages = ErrorPages::new();
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let html = pages.render(&err);

        assert!(html.contains("500"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_status_titles() {
        assert_eq!(get_status_title(404), "Not Found");
        assert_eq!(get_status_title(500), "Internal Server Error");
        assert_eq!(get_status_title(403), "Forbidden");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_dev_vs_prod_rendering() {
        std::env::set_var("APP_ENV", "production");
        let pages = ErrorPages::new();
        let err = HttpError::not_found("Page");
        let rf_err = RustForgeError::Http(err);

        let html = pages.render(&rf_err);
        // Production page should not contain stack trace section
        assert!(!html.contains("Stack Trace"));

        std::env::remove_var("APP_ENV");
    }
}
