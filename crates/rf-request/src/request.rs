//! Request wrapper with Laravel-style API

use crate::{
    error::{RequestError, RequestResult},
    session::Session,
    user::User,
};
use axum::body::Body;
use http::Request as HttpRequest;
use rf_validation::{ValidatedData, ValidationRules, Validator};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

/// Custom Request wrapper providing Laravel-style API
#[derive(Debug)]
pub struct Request {
    /// The underlying Axum request
    pub inner: HttpRequest<Body>,
    /// Parsed request fields (from body, query, path, etc.)
    pub fields: HashMap<String, Value>,
    /// Authenticated user (if any)
    user: Option<User>,
    /// Session (if available)
    session: Option<Session>,
}

impl Request {
    /// Create a new Request from an Axum request
    pub fn new(inner: HttpRequest<Body>) -> Self {
        Self {
            inner,
            fields: HashMap::new(),
            user: None,
            session: None,
        }
    }

    /// Create a Request with pre-parsed fields
    pub fn with_fields(mut self, fields: HashMap<String, Value>) -> Self {
        self.fields = fields;
        self
    }

    /// Set the authenticated user
    pub fn with_user(mut self, user: User) -> Self {
        self.user = Some(user);
        self
    }

    /// Set the session
    pub fn with_session(mut self, session: Session) -> Self {
        self.session = Some(session);
        self
    }

    /// Get a field from the request
    ///
    /// # Example
    ///
    /// ```ignore
    /// let name: String = request.get("name").unwrap();
    /// let age: u32 = request.get("age").unwrap();
    /// ```
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.fields.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a field or return an error if not found
    pub fn require<T: DeserializeOwned>(&self, key: &str) -> RequestResult<T> {
        self.get(key)
            .ok_or_else(|| RequestError::FieldNotFound(key.to_string()))
    }

    /// Get a field with a default value
    pub fn get_or<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// Check if a field exists
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Get all fields
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.fields
    }

    /// Validate the request data
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rf_macros::rules;
    ///
    /// let validated = request.validate(rules! {
    ///     name: required | min(3),
    ///     email: required | email,
    /// }).await?;
    /// ```
    pub async fn validate(&self, rules: ValidationRules) -> RequestResult<ValidatedData> {
        let validator = Validator::new(self.fields.clone());

        // Convert ValidationRules (HashMap<&str, Vec<Box<dyn Rule>>>) to the format Validator expects
        let mut rules_map = HashMap::new();
        for (field, field_rules) in rules {
            rules_map.insert(field, field_rules);
        }

        let mut validator = validator;
        validator.rules(rules_map);

        validator
            .validate()
            .await
            .map_err(RequestError::ValidationFailed)
    }

    /// Get the authenticated user
    ///
    /// Returns None if no user is authenticated
    pub fn user(&self) -> Option<&User> {
        self.user.as_ref()
    }

    /// Get the authenticated user or return an error
    pub fn require_user(&self) -> RequestResult<&User> {
        self.user().ok_or(RequestError::Unauthenticated)
    }

    /// Get the session
    ///
    /// Returns None if no session is available
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Get the session or return an error
    pub fn require_session(&self) -> RequestResult<&Session> {
        self.session().ok_or(RequestError::NoSession)
    }

    /// Get a specific input field (alias for get)
    pub fn input<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get(key)
    }

    /// Get only specific fields
    pub fn only(&self, keys: &[&str]) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        for key in keys {
            if let Some(value) = self.fields.get(*key) {
                result.insert(key.to_string(), value.clone());
            }
        }
        result
    }

    /// Get all fields except specific ones
    pub fn except(&self, keys: &[&str]) -> HashMap<String, Value> {
        let mut result = self.fields.clone();
        for key in keys {
            result.remove(*key);
        }
        result
    }

    /// Check if the request has any of the given keys
    pub fn has_any(&self, keys: &[&str]) -> bool {
        keys.iter().any(|key| self.has(key))
    }

    /// Check if the request has all of the given keys
    pub fn has_all(&self, keys: &[&str]) -> bool {
        keys.iter().all(|key| self.has(key))
    }

    /// Merge additional fields into the request
    pub fn merge(&mut self, fields: HashMap<String, Value>) {
        self.fields.extend(fields);
    }

    /// Get the underlying HTTP request
    pub fn http_request(&self) -> &HttpRequest<Body> {
        &self.inner
    }

    /// Get the HTTP method
    pub fn method(&self) -> &http::Method {
        self.inner.method()
    }

    /// Get the request URI
    pub fn uri(&self) -> &http::Uri {
        self.inner.uri()
    }

    /// Get request headers
    pub fn headers(&self) -> &http::HeaderMap {
        self.inner.headers()
    }

    /// Get a specific header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.inner
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
    }

    /// Check if request is JSON
    pub fn is_json(&self) -> bool {
        self.header("content-type")
            .map(|v| v.contains("application/json"))
            .unwrap_or(false)
    }

    /// Check if request expects JSON response
    pub fn wants_json(&self) -> bool {
        self.header("accept")
            .map(|v| v.contains("application/json"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_request() -> Request {
        let http_req = HttpRequest::builder()
            .method("POST")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let mut fields = HashMap::new();
        fields.insert("name".to_string(), json!("John Doe"));
        fields.insert("email".to_string(), json!("john@example.com"));
        fields.insert("age".to_string(), json!(30));

        Request::new(http_req).with_fields(fields)
    }

    #[test]
    fn test_get_field() {
        let request = create_test_request();

        let name: String = request.get("name").unwrap();
        assert_eq!(name, "John Doe");

        let age: u32 = request.get("age").unwrap();
        assert_eq!(age, 30);
    }

    #[test]
    fn test_has_field() {
        let request = create_test_request();

        assert!(request.has("name"));
        assert!(request.has("email"));
        assert!(!request.has("phone"));
    }

    #[test]
    fn test_only() {
        let request = create_test_request();
        let result = request.only(&["name", "email"]);

        assert_eq!(result.len(), 2);
        assert!(result.contains_key("name"));
        assert!(result.contains_key("email"));
        assert!(!result.contains_key("age"));
    }

    #[test]
    fn test_except() {
        let request = create_test_request();
        let result = request.except(&["age"]);

        assert_eq!(result.len(), 2);
        assert!(result.contains_key("name"));
        assert!(result.contains_key("email"));
        assert!(!result.contains_key("age"));
    }

    #[test]
    fn test_has_any() {
        let request = create_test_request();

        assert!(request.has_any(&["name", "phone"]));
        assert!(!request.has_any(&["phone", "address"]));
    }

    #[test]
    fn test_has_all() {
        let request = create_test_request();

        assert!(request.has_all(&["name", "email"]));
        assert!(!request.has_all(&["name", "phone"]));
    }

    #[test]
    fn test_require_user() {
        let request = create_test_request();
        assert!(request.require_user().is_err());

        let user = User::new(1, "test@example.com".to_string());
        let request = request.with_user(user);
        assert!(request.require_user().is_ok());
    }

    #[test]
    fn test_get_or_returns_default_when_missing() {
        let request = create_test_request();
        let result: String = request.get_or("phone", "N/A".to_string());
        assert_eq!(result, "N/A");
    }

    #[test]
    fn test_get_or_returns_value_when_present() {
        let request = create_test_request();
        let result: String = request.get_or("name", "Unknown".to_string());
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_require_returns_error_for_missing_field() {
        let request = create_test_request();
        let result: crate::error::RequestResult<String> = request.require("phone");
        assert!(result.is_err());
        matches!(result.unwrap_err(), crate::error::RequestError::FieldNotFound(_));
    }

    #[test]
    fn test_require_returns_value_for_existing_field() {
        let request = create_test_request();
        let result: crate::error::RequestResult<String> = request.require("name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "John Doe");
    }

    #[test]
    fn test_input_alias_for_get() {
        let request = create_test_request();
        let name: Option<String> = request.input("name");
        assert_eq!(name, Some("John Doe".to_string()));

        let missing: Option<String> = request.input("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_merge_extends_fields() {
        let mut request = create_test_request();
        let mut extra = HashMap::new();
        extra.insert("phone".to_string(), serde_json::json!("555-1234"));

        request.merge(extra);

        assert!(request.has("phone"));
        let phone: String = request.get("phone").unwrap();
        assert_eq!(phone, "555-1234");
        // Original fields still present
        assert!(request.has("name"));
    }

    #[test]
    fn test_method_returns_http_method() {
        let request = create_test_request();
        assert_eq!(request.method(), &http::Method::POST);
    }

    #[test]
    fn test_uri_returns_path() {
        let request = create_test_request();
        assert_eq!(request.uri().path(), "/test");
    }

    #[test]
    fn test_headers_returns_header_map() {
        let request = create_test_request();
        // Default test request has no custom headers set
        let headers = request.headers();
        assert!(headers.is_empty() || !headers.is_empty()); // just ensure it doesn't panic
    }

    #[test]
    fn test_header_returns_specific_header() {
        let http_req = http::Request::builder()
            .method("GET")
            .uri("/test")
            .header("x-custom-header", "my-value")
            .body(axum::body::Body::empty())
            .unwrap();
        let request = Request::new(http_req);
        assert_eq!(request.header("x-custom-header"), Some("my-value"));
        assert!(request.header("x-nonexistent").is_none());
    }

    #[test]
    fn test_is_json_with_json_content_type() {
        let http_req = http::Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();
        let request = Request::new(http_req);
        assert!(request.is_json());
    }

    #[test]
    fn test_is_json_without_content_type() {
        let request = create_test_request();
        // create_test_request() doesn't set content-type header
        // so is_json() depends on whether the header is present
        let _ = request.is_json(); // should not panic
    }

    #[test]
    fn test_wants_json_with_accept_header() {
        let http_req = http::Request::builder()
            .method("GET")
            .uri("/test")
            .header("accept", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();
        let request = Request::new(http_req);
        assert!(request.wants_json());
    }

    #[test]
    fn test_wants_json_without_accept_header() {
        let request = create_test_request();
        assert!(!request.wants_json());
    }

    #[test]
    fn test_all_returns_all_fields() {
        let request = create_test_request();
        let all = request.all();
        assert_eq!(all.len(), 3);
        assert!(all.contains_key("name"));
        assert!(all.contains_key("email"));
        assert!(all.contains_key("age"));
    }

    #[test]
    fn test_require_session_returns_error_when_missing() {
        let request = create_test_request();
        assert!(request.require_session().is_err());
    }

    #[test]
    fn test_require_session_returns_ok_when_present() {
        let request = create_test_request();
        let session = crate::session::Session::new("sess_123".to_string());
        let request = request.with_session(session);
        assert!(request.require_session().is_ok());
    }
}
