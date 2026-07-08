//! Axum extractor that parses the request body/query into a [`Request`].
//!
//! Populates `Request.fields` from the query string plus the body
//! (`application/json`, `application/x-www-form-urlencoded`, or
//! `multipart/form-data` text parts), and `Request` files from multipart file
//! parts — so handler-side `request.get(..)` / `request.file(..)` are real.

use crate::{error::RequestError, upload::UploadedFile, Request};
use axum::{
    body::Body,
    extract::{FromRequest, Multipart, Request as AxumRequest},
    http::Request as HttpRequest,
};
use serde_json::Value;
use std::collections::HashMap;

/// Extractor marker for the [`Request`] type (the impl below is what does the work).
pub struct RequestExtractor;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

/// Parse a urlencoded string (query or form body) into string fields.
fn merge_urlencoded(fields: &mut HashMap<String, Value>, input: &str) {
    if let Ok(pairs) = serde_urlencoded::from_str::<Vec<(String, String)>>(input) {
        for (k, v) in pairs {
            fields.insert(k, Value::String(v));
        }
    }
}

/// Parse an incoming request into `(fields, files, drained_inner_request)`.
///
/// Shared by the [`Request`] extractor and the `capture_request` middleware, so
/// both populate identical fields/files from JSON, query, form and multipart.
pub async fn parse_request<S>(
    req: AxumRequest,
    state: &S,
) -> Result<(HashMap<String, Value>, HashMap<String, UploadedFile>, HttpRequest<Body>), RequestError>
where
    S: Send + Sync,
{
    {
        let (parts, body) = req.into_parts();

        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Captured so we can rebuild `inner` after the body/parts are consumed.
        let method = parts.method.clone();
        let uri = parts.uri.clone();
        let version = parts.version;
        let headers = parts.headers.clone();
        // Preserve request extensions (captured before the multipart branch may
        // move `parts`). These carry axum's routing state — matched path params
        // (`RawPathParams`) and `MatchedPath` — which are inserted during route
        // matching. `capture_request` runs *after* matching, so dropping them here
        // would hide `/posts/:id` params from any per-route layer or extractor.
        let extensions = parts.extensions.clone();

        // 1. Query string is always merged in as base fields.
        let mut fields: HashMap<String, Value> = HashMap::new();
        if let Some(query) = uri.query() {
            merge_urlencoded(&mut fields, query);
        }

        let mut files: HashMap<String, UploadedFile> = HashMap::new();

        // 2. Body, by content type (body values override query on key collision).
        if content_type.contains("application/json") {
            let bytes = axum::body::to_bytes(body, MAX_BODY_SIZE)
                .await
                .map_err(|e| RequestError::InvalidBody(e.to_string()))?;
            if !bytes.is_empty() {
                let json: Value = serde_json::from_slice(&bytes)
                    .map_err(|e| RequestError::InvalidBody(e.to_string()))?;
                match json {
                    Value::Object(map) => {
                        for (k, v) in map {
                            fields.insert(k, v);
                        }
                    }
                    _ => {
                        return Err(RequestError::InvalidBody(
                            "Expected JSON object".to_string(),
                        ))
                    }
                }
            }
        } else if content_type.contains("application/x-www-form-urlencoded") {
            let bytes = axum::body::to_bytes(body, MAX_BODY_SIZE)
                .await
                .map_err(|e| RequestError::InvalidBody(e.to_string()))?;
            let text = String::from_utf8_lossy(&bytes);
            merge_urlencoded(&mut fields, &text);
        } else if content_type.contains("multipart/form-data") {
            // Hand the (parts, body) to axum's Multipart extractor.
            let req = AxumRequest::from_parts(parts, body);
            let mut multipart = Multipart::from_request(req, state)
                .await
                .map_err(|e| RequestError::InvalidBody(e.to_string()))?;

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RequestError::InvalidBody(e.to_string()))?
            {
                let Some(name) = field.name().map(str::to_string) else {
                    continue;
                };
                let filename = field.file_name().map(str::to_string);
                let field_content_type = field.content_type().map(str::to_string);

                if filename.is_some() {
                    // A file part: keep the raw bytes for later `.store()`.
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| RequestError::InvalidBody(e.to_string()))?;
                    files.insert(
                        name.clone(),
                        UploadedFile::new(name, filename, field_content_type, data),
                    );
                } else {
                    // A plain text field.
                    let text = field
                        .text()
                        .await
                        .map_err(|e| RequestError::InvalidBody(e.to_string()))?;
                    fields.insert(name, Value::String(text));
                }
            }
        }
        // Any other content type: no body fields (query params still apply).

        // Rebuild a lightweight inner request (headers/method/uri preserved, body drained).
        let mut builder = HttpRequest::builder().method(method).uri(uri).version(version);
        if let Some(dst) = builder.headers_mut() {
            *dst = headers;
        }
        let mut http_req = builder
            .body(Body::empty())
            .map_err(|e| RequestError::InvalidBody(e.to_string()))?;
        *http_req.extensions_mut() = extensions;

        Ok((fields, files, http_req))
    }
}

impl<S> FromRequest<S> for Request
where
    S: Send + Sync,
{
    type Rejection = RequestError;

    async fn from_request(req: AxumRequest, state: &S) -> Result<Self, Self::Rejection> {
        let (fields, files, inner) = parse_request(req, state).await?;
        Ok(Request::new(inner).with_fields(fields).with_files(files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::Request as HttpRequest;

    #[tokio::test]
    async fn test_extract_json_request() {
        let http_req = HttpRequest::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name": "John", "age": 30}"#))
            .unwrap();

        let request = Request::from_request(http_req, &()).await.unwrap();
        assert_eq!(request.get::<String>("name").unwrap(), "John");
        assert_eq!(request.get::<u32>("age").unwrap(), 30);
    }

    #[tokio::test]
    async fn test_extract_query_params() {
        let http_req = HttpRequest::builder()
            .method("GET")
            .uri("/search?q=rust&page=2")
            .body(Body::empty())
            .unwrap();

        let request = Request::from_request(http_req, &()).await.unwrap();
        assert_eq!(request.get::<String>("q").unwrap(), "rust");
        assert_eq!(request.get::<String>("page").unwrap(), "2");
    }

    #[tokio::test]
    async fn test_extract_form_urlencoded() {
        let http_req = HttpRequest::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("title=Hello&body=World"))
            .unwrap();

        let request = Request::from_request(http_req, &()).await.unwrap();
        assert_eq!(request.get::<String>("title").unwrap(), "Hello");
        assert_eq!(request.get::<String>("body").unwrap(), "World");
    }

    #[tokio::test]
    async fn test_extract_multipart_text_and_file() {
        let boundary = "X-BOUNDARY";
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHi\r\n\
             --{b}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"a.txt\"\r\n\
             Content-Type: text/plain\r\n\r\nFILEDATA\r\n--{b}--\r\n",
            b = boundary
        );
        let http_req = HttpRequest::builder()
            .method("POST")
            .uri("/upload")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();

        let request = Request::from_request(http_req, &()).await.unwrap();
        // text field parsed into fields
        assert_eq!(request.get::<String>("title").unwrap(), "Hi");
        // file part parsed into files
        let file = request.file("image").expect("image file present");
        assert_eq!(file.filename(), Some("a.txt"));
        assert_eq!(file.bytes(), b"FILEDATA");
        assert_eq!(file.size(), 8);
    }
}
