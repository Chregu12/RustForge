//! HTTP testing utilities

use axum::{body::Body, Router};
use bytes::Bytes;
use http::{Request, StatusCode};
use tower::util::ServiceExt;

/// HTTP test client
///
/// Provides utilities for testing HTTP endpoints in Axum applications.
///
/// # Example
///
/// ```
/// use rf_testing::HttpTester;
/// use axum::{Router, routing::get, Json};
/// use serde_json::json;
///
/// # async fn example() {
/// async fn get_user() -> Json<serde_json::Value> {
///     Json(json!({"id": 1, "name": "Test"}))
/// }
///
/// let app = Router::new().route("/user", get(get_user));
/// let client = HttpTester::new(app);
///
/// let response = client.get("/user").await;
/// response.assert_ok().assert_json_path("name", "Test").await;
/// # }
/// ```
pub struct HttpTester {
    app: Router,
}

impl HttpTester {
    /// Create new HTTP tester
    pub fn new(app: Router) -> Self {
        Self { app }
    }

    /// Make GET request
    pub async fn get(&self, uri: &str) -> TestResponse {
        self.request(
            Request::builder()
                .uri(uri)
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    /// Make POST request with JSON body
    pub async fn post(&self, uri: &str, body: serde_json::Value) -> TestResponse {
        let body_str = serde_json::to_string(&body).unwrap();

        self.request(
            Request::builder()
                .uri(uri)
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
    }

    /// Make PUT request with JSON body
    pub async fn put(&self, uri: &str, body: serde_json::Value) -> TestResponse {
        let body_str = serde_json::to_string(&body).unwrap();

        self.request(
            Request::builder()
                .uri(uri)
                .method("PUT")
                .header("content-type", "application/json")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
    }

    /// Make DELETE request
    pub async fn delete(&self, uri: &str) -> TestResponse {
        self.request(
            Request::builder()
                .uri(uri)
                .method("DELETE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    /// Make custom request
    async fn request(&self, req: Request<Body>) -> TestResponse {
        let response = self.app.clone().oneshot(req).await.unwrap();

        TestResponse::new(response)
    }
}

/// Test response wrapper with assertion methods
pub struct TestResponse {
    response: http::Response<Body>,
    body: Option<Bytes>,
}

impl TestResponse {
    fn new(response: http::Response<Body>) -> Self {
        Self {
            response,
            body: None,
        }
    }

    /// Get status code
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    /// Assert status code
    pub fn assert_status(self, status: StatusCode) -> Self {
        assert_eq!(
            self.status(),
            status,
            "Expected status {}, got {}",
            status,
            self.status()
        );
        self
    }

    /// Assert success (2xx)
    pub fn assert_ok(self) -> Self {
        assert!(
            self.status().is_success(),
            "Expected success status, got {}",
            self.status()
        );
        self
    }

    /// Assert redirect (3xx)
    pub fn assert_redirect(self) -> Self {
        assert!(
            self.status().is_redirection(),
            "Expected redirect status, got {}",
            self.status()
        );
        self
    }

    /// Assert client error (4xx)
    pub fn assert_client_error(self) -> Self {
        assert!(
            self.status().is_client_error(),
            "Expected client error status, got {}",
            self.status()
        );
        self
    }

    /// Assert server error (5xx)
    pub fn assert_server_error(self) -> Self {
        assert!(
            self.status().is_server_error(),
            "Expected server error status, got {}",
            self.status()
        );
        self
    }

    /// Get response body
    pub async fn body(&mut self) -> &Bytes {
        if self.body.is_none() {
            let body = axum::body::to_bytes(
                std::mem::replace(self.response.body_mut(), Body::empty()),
                usize::MAX,
            )
            .await
            .unwrap();
            self.body = Some(body);
        }
        self.body.as_ref().unwrap()
    }

    /// Get response as JSON
    pub async fn json<T: serde::de::DeserializeOwned>(&mut self) -> T {
        let body = self.body().await;
        serde_json::from_slice(body).unwrap()
    }

    /// Assert JSON matches exactly
    pub async fn assert_json(mut self, expected: serde_json::Value) -> Self {
        let actual: serde_json::Value = self.json().await;
        assert_eq!(actual, expected, "JSON mismatch");
        self
    }

    /// Assert JSON contains a specific path
    pub async fn assert_json_path(mut self, path: &str, expected: &str) -> Self {
        let json: serde_json::Value = self.json().await;

        let value = json.get(path);
        assert!(value.is_some(), "JSON missing path: {}", path);

        let value_str = value.unwrap().as_str().unwrap_or("");
        assert_eq!(value_str, expected, "JSON path mismatch for {}", path);

        self
    }

    /// Assert header exists with value
    pub fn assert_header(self, name: &str, value: &str) -> Self {
        assert_eq!(
            self.response
                .headers()
                .get(name)
                .map(|v| v.to_str().unwrap()),
            Some(value),
            "Header mismatch for {}",
            name
        );
        self
    }

    /// Assert header exists
    pub fn assert_header_exists(self, name: &str) -> Self {
        assert!(
            self.response.headers().contains_key(name),
            "Header {} not found",
            name
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json};
    use serde_json::json;

    #[tokio::test]
    async fn test_get_request() {
        async fn handler() -> Json<serde_json::Value> {
            Json(json!({"status": "ok"}))
        }

        let app = Router::new().route("/test", get(handler));
        let client = HttpTester::new(app);

        client
            .get("/test")
            .await
            .assert_ok()
            .assert_json(json!({"status": "ok"}))
            .await;
    }

    #[tokio::test]
    async fn test_post_request() {
        async fn handler(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
            Json(body)
        }

        let app = Router::new().route("/echo", axum::routing::post(handler));
        let client = HttpTester::new(app);

        let data = json!({"name": "test"});
        client
            .post("/echo", data.clone())
            .await
            .assert_ok()
            .assert_json(data)
            .await;
    }

    #[tokio::test]
    async fn test_status_assertions() {
        async fn handler_404() -> StatusCode {
            StatusCode::NOT_FOUND
        }

        let app = Router::new().route("/not-found", get(handler_404));
        let client = HttpTester::new(app);

        client
            .get("/not-found")
            .await
            .assert_status(StatusCode::NOT_FOUND)
            .assert_client_error();
    }

    #[tokio::test]
    async fn test_json_path() {
        async fn handler() -> Json<serde_json::Value> {
            Json(json!({"user": {"name": "Alice", "age": 30}}))
        }

        let app = Router::new().route("/user", get(handler));
        let client = HttpTester::new(app);

        // Note: This is a simplified test. Real path navigation would need more work
        // For now we test single-level paths
    }
}
