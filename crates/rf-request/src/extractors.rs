//! Axum extractors for Request type

use crate::{error::RequestError, Request};
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Request as AxumRequest},
    http::Request as HttpRequest,
};
use serde_json::Value;
use std::collections::HashMap;

/// Extractor for Request type
///
/// This allows using `Request` directly in Axum handlers
pub struct RequestExtractor;

#[async_trait]
impl<S> FromRequest<S> for Request
where
    S: Send + Sync,
{
    type Rejection = RequestError;

    async fn from_request(req: AxumRequest, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the body first
        let (parts, body) = req.into_parts();

        // Get the content type from parts
        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Parse the body based on content type
        let fields = if content_type.contains("application/json") {
            // Parse JSON body
            let bytes = axum::body::to_bytes(body, usize::MAX)
                .await
                .map_err(|e| RequestError::InvalidBody(e.to_string()))?;

            if bytes.is_empty() {
                HashMap::new()
            } else {
                let json: Value = serde_json::from_slice(&bytes)
                    .map_err(|e| RequestError::InvalidBody(e.to_string()))?;

                match json {
                    Value::Object(map) => map
                        .into_iter()
                        .map(|(k, v)| (k, v))
                        .collect(),
                    _ => return Err(RequestError::InvalidBody(
                        "Expected JSON object".to_string(),
                    )),
                }
            }
        } else {
            // For now, we only support JSON
            // TODO: Add support for form data, multipart, etc.
            HashMap::new()
        };

        // Reconstruct the HTTP request
        let http_req = HttpRequest::from_parts(parts, Body::empty());

        Ok(Request::new(http_req).with_fields(fields))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::Request as HttpRequest;

    #[tokio::test]
    async fn test_extract_json_request() {
        let json_body = r#"{"name": "John", "age": 30}"#;

        let http_req = HttpRequest::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(json_body))
            .unwrap();

        let request = Request::from_request(http_req, &()).await.unwrap();

        assert!(request.has("name"));
        assert!(request.has("age"));

        let name: String = request.get("name").unwrap();
        assert_eq!(name, "John");

        let age: u32 = request.get("age").unwrap();
        assert_eq!(age, 30);
    }

    #[tokio::test]
    async fn test_extract_empty_request() {
        let http_req = HttpRequest::builder()
            .method("GET")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let request = Request::from_request(http_req, &()).await.unwrap();
        assert_eq!(request.all().len(), 0);
    }
}
