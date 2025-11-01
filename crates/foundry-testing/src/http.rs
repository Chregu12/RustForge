use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

/// Test client for making HTTP requests in tests
pub struct TestClient {
    router: Router,
}

impl TestClient {
    /// Create a new test client with the given router
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    /// Make a GET request
    pub async fn get(&self, uri: &str) -> TestResponse {
        let request = Request::builder()
            .uri(uri)
            .method("GET")
            .body(Body::empty())
            .unwrap();

        self.send_request(request).await
    }

    /// Make a POST request with JSON body
    pub async fn post<T: Serialize>(&self, uri: &str, body: &T) -> TestResponse {
        let json = serde_json::to_string(body).unwrap();
        let request = Request::builder()
            .uri(uri)
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(json))
            .unwrap();

        self.send_request(request).await
    }

    /// Make a PUT request with JSON body
    pub async fn put<T: Serialize>(&self, uri: &str, body: &T) -> TestResponse {
        let json = serde_json::to_string(body).unwrap();
        let request = Request::builder()
            .uri(uri)
            .method("PUT")
            .header("content-type", "application/json")
            .body(Body::from(json))
            .unwrap();

        self.send_request(request).await
    }

    /// Make a DELETE request
    pub async fn delete(&self, uri: &str) -> TestResponse {
        let request = Request::builder()
            .uri(uri)
            .method("DELETE")
            .body(Body::empty())
            .unwrap();

        self.send_request(request).await
    }

    /// Make a PATCH request with JSON body
    pub async fn patch<T: Serialize>(&self, uri: &str, body: &T) -> TestResponse {
        let json = serde_json::to_string(body).unwrap();
        let request = Request::builder()
            .uri(uri)
            .method("PATCH")
            .header("content-type", "application/json")
            .body(Body::from(json))
            .unwrap();

        self.send_request(request).await
    }

    /// Send a custom request
    async fn send_request(&self, request: Request<Body>) -> TestResponse {
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to send request");

        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        let body_string = String::from_utf8(body_bytes.to_vec())
            .expect("Response body is not valid UTF-8");

        TestResponse {
            status,
            body: body_string,
        }
    }
}

/// Test response wrapper
pub struct TestResponse {
    pub status: StatusCode,
    pub body: String,
}

impl TestResponse {
    /// Get the status code
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the response body as a string
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Parse the response body as JSON
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_str(&self.body).expect("Failed to parse JSON response")
    }

    /// Assert that the response has a specific status code
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}. Body: {}",
            expected, self.status, self.body
        );
        self
    }

    /// Assert that the response is successful (2xx)
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.status.is_success(),
            "Expected success status, got {}. Body: {}",
            self.status,
            self.body
        );
        self
    }

    /// Assert that the response is a client error (4xx)
    pub fn assert_client_error(&self) -> &Self {
        assert!(
            self.status.is_client_error(),
            "Expected client error status, got {}. Body: {}",
            self.status,
            self.body
        );
        self
    }

    /// Assert that the response is a server error (5xx)
    pub fn assert_server_error(&self) -> &Self {
        assert!(
            self.status.is_server_error(),
            "Expected server error status, got {}. Body: {}",
            self.status,
            self.body
        );
        self
    }

    /// Assert that the response body contains a specific string
    pub fn assert_body_contains(&self, expected: &str) -> &Self {
        assert!(
            self.body.contains(expected),
            "Expected body to contain '{}', but it didn't. Body: {}",
            expected,
            self.body
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
        let app = Router::new().route(
            "/test",
            get(|| async { Json(json!({"message": "hello"})) }),
        );

        let client = TestClient::new(app);
        let response = client.get("/test").await;

        response.assert_status(StatusCode::OK);
        let data: serde_json::Value = response.json();
        assert_eq!(data["message"], "hello");
    }
}
