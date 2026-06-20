//! Redirect helpers for HTTP responses.
//!
//! This module provides Laravel-style redirect functions with support for
//! flash messages, input flashing, and error messages.

use axum::response::{IntoResponse, Redirect, Response};
use axum::http::{StatusCode, header};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

/// Flash message storage (simplified - in production this would use sessions)
static FLASH_MESSAGES: Lazy<RwLock<HashMap<String, FlashData>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

/// Flash data stored in session
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct FlashData {
    pub messages: HashMap<String, String>,
    pub old_input: HashMap<String, String>,
    pub errors: HashMap<String, Vec<String>>,
}


/// A redirect response with fluent API for adding flash data.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_global_helpers::redirect;
///
/// // Simple redirect
/// let response = redirect("/dashboard");
///
/// // Redirect with success message
/// let response = redirect("/posts")
///     .with("success", "Post created successfully!");
///
/// // Redirect with errors
/// let response = redirect("/form")
///     .with_errors(vec![
///         ("email", vec!["Invalid email format"]),
///         ("password", vec!["Password too short"]),
///     ]);
/// ```
pub struct RedirectResponse {
    to: String,
    status: StatusCode,
    flash_data: FlashData,
}

impl RedirectResponse {
    /// Create a new redirect response to the given path.
    pub fn to(path: impl Into<String>) -> Self {
        Self {
            to: path.into(),
            status: StatusCode::SEE_OTHER,
            flash_data: FlashData::default(),
        }
    }

    /// Create a redirect response to the previous page.
    pub fn back() -> Self {
        // In production, this would read from the request's Referer header
        Self::to("/")
    }

    /// Create a redirect to a named route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_global_helpers::RedirectResponse;
    ///
    /// let response = RedirectResponse::route("users.show", vec![("id", "123")]);
    /// ```
    pub fn route(name: impl Into<String>, params: Vec<(&str, &str)>) -> Self {
        // In production, this would use the route registry
        let name = name.into();
        let mut path = format!("/{}", name);

        if !params.is_empty() {
            path.push('?');
            let query = params
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        urlencoding::encode(k),
                        urlencoding::encode(v)
                    )
                })
                .collect::<Vec<_>>()
                .join("&");
            path.push_str(&query);
        }

        Self::to(path)
    }

    /// Set the HTTP status code for the redirect.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_global_helpers::redirect;
    /// use axum::http::StatusCode;
    ///
    /// let response = redirect("/new-location")
    ///     .status(StatusCode::MOVED_PERMANENTLY);
    /// ```
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add a flash message to the redirect.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_global_helpers::redirect;
    ///
    /// let response = redirect("/posts")
    ///     .with("success", "Post created!")
    ///     .with("info", "Email sent to author");
    /// ```
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.flash_data.messages.insert(key.into(), value.into());
        self
    }

    /// Flash the current input for the next request.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_global_helpers::redirect;
    ///
    /// let mut input = std::collections::HashMap::new();
    /// input.insert("email".to_string(), "user@example.com".to_string());
    ///
    /// let response = redirect("/form")
    ///     .with_input(input);
    /// ```
    pub fn with_input(mut self, input: HashMap<String, String>) -> Self {
        self.flash_data.old_input = input;
        self
    }

    /// Flash validation errors for the next request.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_global_helpers::redirect;
    ///
    /// let response = redirect("/form")
    ///     .with_errors(vec![
    ///         ("email", vec!["Invalid email format"]),
    ///         ("password", vec!["Password too short"]),
    ///     ]);
    /// ```
    pub fn with_errors<K, V>(mut self, errors: Vec<(K, Vec<V>)>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        for (key, messages) in errors {
            let key = key.into();
            let messages: Vec<String> = messages.into_iter().map(|m| m.into()).collect();
            self.flash_data.errors.insert(key, messages);
        }
        self
    }

    /// Add validation errors from a HashMap.
    pub fn with_errors_map(mut self, errors: HashMap<String, Vec<String>>) -> Self {
        self.flash_data.errors = errors;
        self
    }

    /// Flash a success message.
    pub fn with_success(self, message: impl Into<String>) -> Self {
        self.with("success", message)
    }

    /// Flash an error message.
    pub fn with_error(self, message: impl Into<String>) -> Self {
        self.with("error", message)
    }

    /// Flash a warning message.
    pub fn with_warning(self, message: impl Into<String>) -> Self {
        self.with("warning", message)
    }

    /// Flash an info message.
    pub fn with_info(self, message: impl Into<String>) -> Self {
        self.with("info", message)
    }
}

impl IntoResponse for RedirectResponse {
    fn into_response(self) -> Response {
        // Store flash data (in production, this would use sessions)
        // For now, we use a simple static storage
        let session_id = uuid::Uuid::new_v4().to_string();
        FLASH_MESSAGES.write().insert(session_id.clone(), self.flash_data);

        // Create redirect response
        let redirect = Redirect::to(&self.to);
        let mut response = redirect.into_response();

        // Set custom status if not the default
        if self.status != StatusCode::SEE_OTHER {
            *response.status_mut() = self.status;
        }

        // Add session cookie (simplified)
        if let Ok(cookie) = format!("session_id={}", session_id).parse() {
            response.headers_mut().insert(header::SET_COOKIE, cookie);
        }

        response
    }
}

/// Create a redirect response to the given path.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_global_helpers::redirect;
///
/// let response = redirect("/dashboard");
/// ```
pub fn redirect(to: impl Into<String>) -> RedirectResponse {
    RedirectResponse::to(to)
}

/// Create a redirect response to the previous page.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_global_helpers::back;
///
/// let response = back();
/// ```
pub fn back() -> RedirectResponse {
    RedirectResponse::back()
}

/// Get flash data for a session (helper for testing).
pub fn get_flash_data(session_id: &str) -> Option<FlashData> {
    FLASH_MESSAGES.read().get(session_id).cloned()
}

/// Clear flash data for a session.
pub fn clear_flash_data(session_id: &str) {
    FLASH_MESSAGES.write().remove(session_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_to() {
        let response = RedirectResponse::to("/dashboard");
        assert_eq!(response.to, "/dashboard");
        assert_eq!(response.status, StatusCode::SEE_OTHER);
    }

    #[test]
    fn test_redirect_back() {
        let response = RedirectResponse::back();
        assert_eq!(response.to, "/");
    }

    #[test]
    fn test_redirect_with_message() {
        let response = RedirectResponse::to("/posts")
            .with("success", "Post created!");

        assert_eq!(
            response.flash_data.messages.get("success"),
            Some(&"Post created!".to_string())
        );
    }

    #[test]
    fn test_redirect_with_errors() {
        let response = RedirectResponse::to("/form")
            .with_errors(vec![
                ("email", vec!["Invalid email"]),
                ("password", vec!["Too short"]),
            ]);

        assert_eq!(response.flash_data.errors.len(), 2);
        assert!(response.flash_data.errors.contains_key("email"));
        assert!(response.flash_data.errors.contains_key("password"));
    }

    #[test]
    fn test_redirect_with_input() {
        let mut input = HashMap::new();
        input.insert("email".to_string(), "user@example.com".to_string());

        let response = RedirectResponse::to("/form").with_input(input);

        assert_eq!(
            response.flash_data.old_input.get("email"),
            Some(&"user@example.com".to_string())
        );
    }

    #[test]
    fn test_redirect_status() {
        let response = RedirectResponse::to("/new-location")
            .status(StatusCode::MOVED_PERMANENTLY);

        assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
    }

    #[test]
    fn test_redirect_with_success() {
        let response = RedirectResponse::to("/posts")
            .with_success("Created successfully!");

        assert_eq!(
            response.flash_data.messages.get("success"),
            Some(&"Created successfully!".to_string())
        );
    }

    #[test]
    fn test_redirect_helper() {
        let response = redirect("/test");
        assert_eq!(response.to, "/test");
    }

    #[test]
    fn test_back_helper() {
        let response = back();
        assert_eq!(response.to, "/");
    }

    #[test]
    fn test_redirect_chaining() {
        let response = RedirectResponse::to("/form")
            .with_success("Form submitted!")
            .with_warning("Some fields were corrected")
            .with_error("Please review the errors");

        assert_eq!(response.flash_data.messages.len(), 3);
    }
}
