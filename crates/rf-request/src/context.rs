//! Implicit (task-local) access to the current request.
//!
//! The `capture_request` middleware parses each incoming request once and stashes
//! its fields/files in a per-request task-local scope, so handlers can use the
//! Laravel-style global helpers — `input()`, `has()`, `file()`, `all()` — without
//! threading a `Request` around. Outside a request scope these return empty.

use crate::extractors::parse_request;
use crate::upload::UploadedFile;
use axum::{
    extract::{RawPathParams, Request as AxumRequest},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    RequestPartsExt,
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
/// It buffers the body to parse the globals, then re-inserts the JSON/urlencoded
/// bytes on the downstream request — so a handler behind this middleware can use
/// BOTH the global helpers AND a second body extractor (axum `Json`,
/// `ValidatedJson`) reading the same body. (Multipart bodies are drained
/// field-by-field and not re-inserted; multipart handlers use the globals/context.)
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
        // An oversize body (over MAX_BODY_SIZE, whether JSON/urlencoded/multipart)
        // is rejected as 413 Payload Too Large — the SAME ceiling for every body
        // kind — with the framework's stable JSON envelope.
        Err(crate::error::RequestError::PayloadTooLarge(_)) => (
            StatusCode::PAYLOAD_TOO_LARGE,
            axum::Json(serde_json::json!({
                "error": "Payload too large",
                "message": "The request body exceeds the allowed size limit.",
            })),
        )
            .into_response(),
        // Malformed body (e.g. invalid JSON): reject with a stable JSON error
        // envelope (application/json) instead of a text/plain string that would
        // leak the underlying serde parser internals (byte/line/column). The
        // shape mirrors the rest of the framework's JSON error responses.
        Err(_) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "Invalid request body",
                "message": "The request body could not be parsed.",
            })),
        )
            .into_response(),
    }
}

/// Per-route middleware that captures matched path parameters (e.g. the `id` in
/// `GET /posts/:id`) and merges them into the current request context, so the
/// global helpers can read `input::<i64>("id")` from a handler with no arguments.
///
/// Because axum only populates [`RawPathParams`] *after* route matching, this must
/// run as a `route_layer` (inside the router), not as the outer `capture_request`
/// layer which runs before matching. It is wired automatically for every route by
/// `rf_routing::GlobalRouter::build_router`.
///
/// It extends whatever context [`capture_request`] already set (query/body/files);
/// if that outer layer is absent it still exposes the path params on their own.
/// Path params take precedence over query/body fields on key collision, matching
/// the URL being authoritative.
/// Coerce a raw path segment into the most natural JSON scalar so the typed
/// helpers work: `/posts/5` yields a JSON number (`input::<i64>("id")` == 5),
/// while a non-numeric slug stays a string (`input::<String>("slug")`).
fn path_param_value(raw: &str) -> Value {
    if let Ok(i) = raw.parse::<i64>() {
        return Value::from(i);
    }
    Value::String(raw.to_string())
}

pub async fn capture_path_params(req: AxumRequest, next: Next) -> Response {
    // axum 0.8 dropped the `Option<RawPathParams>` extractor form (an optional
    // extractor now needs `OptionalFromRequestParts`, which `RawPathParams` does
    // not implement), while a bare `RawPathParams` *rejects* on a route with no
    // params (e.g. `/posts`). So split the request, try to extract the params, and
    // treat a rejection as "no params" — keeping this layer safe on every route,
    // parameterised or not.
    let (mut parts, body) = req.into_parts();
    let params = parts.extract::<RawPathParams>().await.ok();
    let req = AxumRequest::from_parts(parts, body);

    // Start from the context set by `capture_request` (or an empty one).
    let (mut fields, files) =
        with_ctx(|c| (c.fields.clone(), c.files.clone())).unwrap_or_default();

    let mut merged = false;
    if let Some(params) = &params {
        for (key, value) in params.iter() {
            fields.insert(key.to_string(), path_param_value(value));
            merged = true;
        }
    }

    // Nothing to add and no outer context to preserve: avoid a needless re-scope.
    if !merged && CURRENT_REQUEST.try_with(|_| ()).is_err() {
        return next.run(req).await;
    }

    let ctx = Arc::new(RequestContext { fields, files });
    with_request_context(ctx, next.run(req)).await
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

    #[test]
    fn test_path_param_value_coercion() {
        assert_eq!(path_param_value("5"), Value::from(5_i64));
        assert_eq!(path_param_value("-12"), Value::from(-12_i64));
        assert_eq!(path_param_value("hello-world"), Value::String("hello-world".into()));
        // A value that overflows i64 stays a string rather than being mangled.
        assert_eq!(
            path_param_value("99999999999999999999"),
            Value::String("99999999999999999999".into())
        );
    }

    #[tokio::test]
    async fn test_capture_path_params_merges_into_globals() {
        use axum::{body::Body, http::Request, routing::get, Router};
        use tower::ServiceExt;

        // A handler with NO arguments reads the matched `{id}` via the globals.
        async fn show() -> String {
            let id: i64 = input("id").expect("id present as i64");
            let extra: String = input("q").unwrap_or_default();
            format!("id={id};q={extra}")
        }

        let app = Router::new().route(
            "/posts/{id}",
            get(show).route_layer(axum::middleware::from_fn(capture_path_params)),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/posts/42?q=hi")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        // Path param merged (typed as i64); query is NOT parsed here (that is
        // `capture_request`'s job) so `q` is absent — proving this layer is
        // strictly additive over whatever context already exists.
        assert_eq!(String::from_utf8_lossy(&bytes), "id=42;q=");
    }
}
