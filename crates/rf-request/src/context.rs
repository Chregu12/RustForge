//! Implicit (task-local) access to the current request.
//!
//! The `capture_request` middleware parses each incoming request once and stashes
//! its fields/files in a per-request task-local scope, so handlers can use the
//! Laravel-style global helpers — `input()`, `has()`, `file()`, `all()` — without
//! threading a `Request` around. Outside a request scope these return empty.

use crate::extractors::parse_request;
use crate::upload::UploadedFile;
use axum::{
    extract::Request as AxumRequest,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

/// The parsed request data exposed to the global helpers.
#[derive(Debug, Default)]
pub struct RequestContext {
    /// Merged query + body fields.
    pub fields: HashMap<String, Value>,
    /// Uploaded files, keyed by form field name.
    pub files: HashMap<String, UploadedFile>,
}

tokio::task_local! {
    static CURRENT_REQUEST: Arc<RequestContext>;
}

fn with_ctx<R>(f: impl FnOnce(&RequestContext) -> R) -> Option<R> {
    CURRENT_REQUEST.try_with(|ctx| f(ctx)).ok()
}

/// Get a field from the current request (query or body). `None` outside a request
/// scope, or when the key is absent / cannot deserialize to `T`.
pub fn input<T: DeserializeOwned>(key: &str) -> Option<T> {
    with_ctx(|c| {
        c.fields
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    })
    .flatten()
}

/// True if the current request has a field named `key`.
pub fn has(key: &str) -> bool {
    with_ctx(|c| c.fields.contains_key(key)).unwrap_or(false)
}

/// Get an uploaded file from the current request by field name, e.g. `file("image")`.
pub fn file(name: &str) -> Option<UploadedFile> {
    with_ctx(|c| c.files.get(name).cloned()).flatten()
}

/// All fields of the current request.
pub fn all() -> HashMap<String, Value> {
    with_ctx(|c| c.fields.clone()).unwrap_or_default()
}

/// Run a future within a request context scope (used by the middleware and tests).
pub async fn with_request_context<F, R>(ctx: Arc<RequestContext>, fut: F) -> R
where
    F: Future<Output = R>,
{
    CURRENT_REQUEST.scope(ctx, fut).await
}

/// Middleware that parses the incoming request once and exposes it to the global
/// helpers ([`input`]/[`has`]/[`file`]/[`all`]) for the duration of the handler.
///
/// Because it drains the body while parsing, handlers behind this middleware use
/// the global helpers (or the parsed context) rather than a second body extractor.
///
/// ```ignore
/// use axum::{Router, routing::post, middleware};
/// use rf_request::capture_request;
/// let app = Router::new().route("/", post(handler))
///     .layer(middleware::from_fn(capture_request));
/// ```
pub async fn capture_request(req: AxumRequest, next: Next) -> Response {
    match parse_request(req, &()).await {
        Ok((fields, files, inner)) => {
            let ctx = Arc::new(RequestContext { fields, files });
            with_request_context(ctx, next.run(inner)).await
        }
        // Malformed body (e.g. invalid JSON): reject with 400 instead of proceeding.
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_globals_read_current_scope() {
        let mut fields = HashMap::new();
        fields.insert("q".to_string(), Value::String("rust".into()));
        let ctx = Arc::new(RequestContext { fields, files: HashMap::new() });

        with_request_context(ctx, async {
            assert_eq!(input::<String>("q"), Some("rust".to_string()));
            assert!(has("q"));
            assert!(!has("missing"));
            assert!(file("nope").is_none());
        })
        .await;

        // Outside any scope, globals are empty (no panic).
        assert_eq!(input::<String>("q"), None);
        assert!(!has("q"));
    }
}
