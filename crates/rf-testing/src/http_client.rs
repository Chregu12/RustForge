//! HTTP test client for testing Axum applications
//!
//! Provides a fluent API for making HTTP requests and asserting responses.

use crate::{TestError, TestResult};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tower::ServiceExt;

/// HTTP test client for making requests to an Axum application
pub struct TestClient {
    router: Router,
    headers: HashMap<String, String>,
}

impl TestClient {
    /// Create a new test client with the given router
    pub fn new(router: Router) -> Self {
        Self {
            router,
            headers: HashMap::new(),
        }
    }

    /// Add a header to all subsequent requests
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set authorization header with bearer token
    pub fn with_token(self, token: impl Into<String>) -> Self {
        self.with_header("Authorization", format!("Bearer {}", token.into()))
    }

    /// Set authorization header with basic auth
    pub fn with_basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self {
        let credentials = base64::encode(format!("{}:{}", username.into(), password.into()));
        self.with_header("Authorization", format!("Basic {}", credentials))
    }

    /// Make a GET request
    pub fn get(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "GET", uri.as_ref())
    }

    /// Make a POST request
    pub fn post(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "POST", uri.as_ref())
    }

    /// Make a PUT request
    pub fn put(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "PUT", uri.as_ref())
    }

    /// Make a PATCH request
    pub fn patch(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "PATCH", uri.as_ref())
    }

    /// Make a DELETE request
    pub fn delete(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "DELETE", uri.as_ref())
    }

    /// Make a HEAD request
    pub fn head(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "HEAD", uri.as_ref())
    }

    /// Make an OPTIONS request
    pub fn options(&self, uri: impl AsRef<str>) -> RequestBuilder<'_> {
        RequestBuilder::new(self, "OPTIONS", uri.as_ref())
    }
}

/// Builder for constructing and sending HTTP requests
pub struct RequestBuilder<'a> {
    client: &'a TestClient,
    method: &'static str,
    uri: String,
    headers: HashMap<String, String>,
    body: Option<Bytes>,
}

impl<'a> RequestBuilder<'a> {
    fn new(client: &'a TestClient, method: &'static str, uri: &str) -> Self {
        Self {
            client,
            method,
            uri: uri.to_string(),
            headers: client.headers.clone(),
            body: None,
        }
    }

    /// Add a header to this request
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set JSON body
    pub fn json<T: Serialize>(mut self, body: &T) -> TestResult<Self> {
        let json = serde_json::to_vec(body)
            .map_err(|e| TestError::Other(format!("Failed to serialize JSON: {}", e)))?;
        self.body = Some(json.into());
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Set form data body
    pub fn form<T: Serialize>(mut self, body: &T) -> TestResult<Self> {
        let form = serde_urlencoded::to_string(body)
            .map_err(|e| TestError::Other(format!("Failed to serialize form: {}", e)))?;
        self.body = Some(form.into_bytes().into());
        self.headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        Ok(self)
    }

    /// Set raw body
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Send the request and get a response
    pub async fn send(self) -> TestResult<TestResponseBuilder> {
        let mut request = Request::builder().method(self.method).uri(&self.uri);

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let body = self.body.unwrap_or_else(|| Bytes::new());
        let request = request
            .body(Body::from(body))
            .map_err(|e| TestError::Other(format!("Failed to build request: {}", e)))?;

        let response = self
            .client
            .router
            .clone()
            .oneshot(request)
            .await
            .map_err(|e| TestError::Other(format!("Request failed: {}", e)))?;

        Ok(TestResponseBuilder::new(response))
    }
}

/// Builder for asserting on HTTP responses
pub struct TestResponseBuilder {
    response: axum::response::Response,
    body: Option<Bytes>,
}

impl TestResponseBuilder {
    fn new(response: axum::response::Response) -> Self {
        Self {
            response,
            body: None,
        }
    }

    /// Get the response status code
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
    }

    /// Get the response body as bytes
    pub async fn body_bytes(&mut self) -> TestResult<&Bytes> {
        if self.body.is_none() {
            let body = axum::body::to_bytes(
                std::mem::replace(&mut self.response.body_mut(), Body::empty()),
                usize::MAX,
            )
            .await
            .map_err(|e| TestError::Other(format!("Failed to read body: {}", e)))?;
            self.body = Some(body);
        }
        Ok(self.body.as_ref().unwrap())
    }

    /// Get the response body as a string
    pub async fn body_string(&mut self) -> TestResult<String> {
        let bytes = self.body_bytes().await?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| TestError::Other(format!("Body is not valid UTF-8: {}", e)))
    }

    /// Get the response body as JSON
    pub async fn body_json<T: for<'de> Deserialize<'de>>(&mut self) -> TestResult<T> {
        let bytes = self.body_bytes().await?;
        serde_json::from_slice(bytes)
            .map_err(|e| TestError::Other(format!("Failed to parse JSON: {}", e)))
    }

    /// Assert the status code
    pub fn assert_status(self, expected: StatusCode) -> Self {
        let actual = self.response.status();
        assert_eq!(
            actual, expected,
            "Expected status {}, got {}",
            expected, actual
        );
        self
    }

    /// Assert status is 200 OK
    pub fn assert_ok(self) -> Self {
        self.assert_status(StatusCode::OK)
    }

    /// Assert status is 201 Created
    pub fn assert_created(self) -> Self {
        self.assert_status(StatusCode::CREATED)
    }

    /// Assert status is 204 No Content
    pub fn assert_no_content(self) -> Self {
        self.assert_status(StatusCode::NO_CONTENT)
    }

    /// Assert status is 400 Bad Request
    pub fn assert_bad_request(self) -> Self {
        self.assert_status(StatusCode::BAD_REQUEST)
    }

    /// Assert status is 401 Unauthorized
    pub fn assert_unauthorized(self) -> Self {
        self.assert_status(StatusCode::UNAUTHORIZED)
    }

    /// Assert status is 403 Forbidden
    pub fn assert_forbidden(self) -> Self {
        self.assert_status(StatusCode::FORBIDDEN)
    }

    /// Assert status is 404 Not Found
    pub fn assert_not_found(self) -> Self {
        self.assert_status(StatusCode::NOT_FOUND)
    }

    /// Assert status is 422 Unprocessable Entity
    pub fn assert_unprocessable(self) -> Self {
        self.assert_status(StatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Assert status is 500 Internal Server Error
    pub fn assert_server_error(self) -> Self {
        self.assert_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Assert a header exists with the given value
    pub fn assert_header(self, name: &str, expected: &str) -> Self {
        let actual = self.header(name);
        assert_eq!(
            actual,
            Some(expected),
            "Expected header '{}' to be '{}', got '{:?}'",
            name,
            expected,
            actual
        );
        self
    }

    /// Assert a header exists
    pub fn assert_header_exists(self, name: &str) -> Self {
        assert!(
            self.header(name).is_some(),
            "Expected header '{}' to exist",
            name
        );
        self
    }

    /// Assert a header does not exist
    pub fn assert_header_missing(self, name: &str) -> Self {
        assert!(
            self.header(name).is_none(),
            "Expected header '{}' to not exist",
            name
        );
        self
    }

    /// Assert the response body as JSON
    pub async fn assert_json(mut self, expected: Value) -> Self {
        let actual: Value = self.body_json().await.expect("Failed to parse JSON");
        assert_eq!(
            actual,
            expected,
            "JSON mismatch:\nExpected: {}\nActual: {}",
            serde_json::to_string_pretty(&expected).unwrap(),
            serde_json::to_string_pretty(&actual).unwrap()
        );
        self
    }

    /// Assert JSON path exists and has the expected value
    pub async fn assert_json_path(mut self, path: &str, expected: Value) -> Self {
        let body: Value = self.body_json().await.expect("Failed to parse JSON");
        let results = jsonpath_lib::select(&body, path).expect("Invalid JSON path");
        let actual = results
            .first()
            .expect(&format!("Path '{}' not found", path));

        assert_eq!(
            *actual, &expected,
            "JSON path '{}' mismatch:\nExpected: {}\nActual: {}",
            path, expected, actual
        );
        self
    }

    /// Assert JSON structure matches (keys exist)
    pub async fn assert_json_structure(mut self, paths: &[&str]) -> Self {
        let body: Value = self.body_json().await.expect("Failed to parse JSON");

        for path in paths {
            let parts: Vec<&str> = path.split('.').collect();
            let mut current = &body;

            for part in parts {
                current = current
                    .get(part)
                    .expect(&format!("JSON path '{}' not found at '{}'", path, part));
            }
        }

        self
    }

    /// Assert the response body contains a substring
    pub async fn assert_body_contains(mut self, needle: &str) -> Self {
        let body = self.body_string().await.expect("Failed to get body");
        assert!(
            body.contains(needle),
            "Expected body to contain '{}', but it didn't.\nBody: {}",
            needle,
            body
        );
        self
    }

    /// Assert the response body does not contain a substring
    pub async fn assert_body_not_contains(mut self, needle: &str) -> Self {
        let body = self.body_string().await.expect("Failed to get body");
        assert!(
            !body.contains(needle),
            "Expected body to not contain '{}', but it did.\nBody: {}",
            needle,
            body
        );
        self
    }
}

mod base64 {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(input: String) -> String {
        let bytes = input.as_bytes();
        let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);

        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

            let triple = (b0 << 16) | (b1 << 8) | b2;

            result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

            if chunk.len() > 1 {
                result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(CHARS[(triple & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }

        result
    }
}

// Mock jsonpath for JSON path assertions (in production, use jsonpath-rust or similar)
mod jsonpath_lib {
    use serde_json::Value;

    pub fn select<'a>(value: &'a Value, path: &str) -> Result<Vec<&'a Value>, String> {
        // Simple implementation for basic paths like "$.data.name"
        let parts: Vec<&str> = path.trim_start_matches("$.").split('.').collect();
        let mut current = value;

        for part in parts {
            current = current
                .get(part)
                .ok_or_else(|| format!("Path segment '{}' not found", part))?;
        }

        Ok(vec![current])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json};
    use serde_json::json;

    #[tokio::test]
    async fn test_get_request() {
        async fn handler() -> Json<Value> {
            Json(json!({"message": "hello"}))
        }

        let app = Router::new().route("/test", get(handler));
        let client = TestClient::new(app);

        let mut response = client.get("/test").send().await.unwrap().assert_ok();

        let body: Value = response.body_json().await.unwrap();
        assert_eq!(body, json!({"message": "hello"}));
    }

    #[tokio::test]
    async fn test_post_json() {
        use axum::routing::post;

        async fn handler(Json(payload): Json<Value>) -> Json<Value> {
            Json(payload)
        }

        let app = Router::new().route("/echo", post(handler));
        let client = TestClient::new(app);

        client
            .post("/echo")
            .json(&json!({"name": "John"}))
            .unwrap()
            .send()
            .await
            .unwrap()
            .assert_ok()
            .assert_json(json!({"name": "John"}))
            .await;
    }

    #[tokio::test]
    async fn test_headers() {
        async fn handler() -> Json<Value> {
            Json(json!({"status": "ok"}))
        }

        let app = Router::new().route("/test", get(handler));
        let client = TestClient::new(app).with_header("X-Custom", "value");

        client
            .get("/test")
            .send()
            .await
            .unwrap()
            .assert_ok()
            .assert_header("content-type", "application/json");
    }

    #[tokio::test]
    async fn test_status_assertions() {
        async fn not_found() -> StatusCode {
            StatusCode::NOT_FOUND
        }

        let app = Router::new().route("/404", get(not_found));
        let client = TestClient::new(app);

        client.get("/404").send().await.unwrap().assert_not_found();
    }

    #[tokio::test]
    async fn test_json_path_assertion() {
        async fn handler() -> Json<Value> {
            Json(json!({
                "data": {
                    "user": {
                        "name": "John Doe"
                    }
                }
            }))
        }

        let app = Router::new().route("/user", get(handler));
        let client = TestClient::new(app);

        client
            .get("/user")
            .send()
            .await
            .unwrap()
            .assert_ok()
            .assert_json_path("$.data.user.name", json!("John Doe"))
            .await;
    }
}
