//! Axum middleware for rate limiting

use crate::{LimitResult, RateLimiter};
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Rate limit middleware layer
///
/// # Default key extractor
///
/// By default the layer extracts a per-client key from, in order:
/// 1. The `X-Forwarded-For` header (first / leftmost IP — set by load balancers)
/// 2. The `X-Real-IP` header (set by some reverse proxies, e.g. nginx)
/// 3. `axum::extract::ConnectInfo<std::net::SocketAddr>` from request extensions
///    (requires `.into_make_service_with_connect_info::<SocketAddr>()` on the server)
/// 4. Falls back to `"unknown"` and emits a `tracing::warn` — all requests then
///    share **one** bucket, which is almost certainly wrong for production.
///
/// Override this with [`RateLimitLayer::with_key_extractor`] if you need custom
/// logic (e.g. user-ID-based limiting, bearer-token scoping, etc.).
///
/// # Example
///
/// ```ignore
/// use rf_ratelimit::*;
/// use axum::{Router, routing::get};
///
/// let config = RateLimitConfig::per_minute(60);
/// let limiter = Arc::new(MemoryRateLimiter::new(config));
/// let layer = RateLimitLayer::new(limiter);
///
/// let app = Router::new()
///     .route("/api/users", get(get_users))
///     .layer(axum::middleware::from_fn(move |req, next| {
///         layer.clone().handle(req, next)
///     }));
/// ```
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<dyn RateLimiter>,
    key_extractor: Arc<dyn Fn(&Request) -> String + Send + Sync>,
}

impl RateLimitLayer {
    /// Create new rate limit layer with per-client IP-based limiting.
    ///
    /// The default key extractor resolves the client IP from `X-Forwarded-For`,
    /// then `X-Real-IP`, then `ConnectInfo<SocketAddr>`, falling back to
    /// `"unknown"` with a warning.  Use [`with_key_extractor`] to override.
    pub fn new(limiter: Arc<dyn RateLimiter>) -> Self {
        Self {
            limiter,
            key_extractor: Arc::new(|req| {
                // 1. X-Forwarded-For: take the leftmost (original client) IP.
                if let Some(xff) = req.headers().get("x-forwarded-for") {
                    if let Ok(val) = xff.to_str() {
                        let first = val.split(',').next().unwrap_or("").trim();
                        if !first.is_empty() {
                            return first.to_string();
                        }
                    }
                }
                // 2. X-Real-IP: used by nginx and other proxies.
                if let Some(real_ip) = req.headers().get("x-real-ip") {
                    if let Ok(val) = real_ip.to_str() {
                        let val = val.trim();
                        if !val.is_empty() {
                            return val.to_string();
                        }
                    }
                }
                // 3. ConnectInfo<SocketAddr> inserted by axum when the server is
                //    started with `.into_make_service_with_connect_info::<SocketAddr>()`.
                if let Some(addr) = req
                    .extensions()
                    .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                {
                    return addr.ip().to_string();
                }
                // 4. No IP available — log a warning and share one bucket.
                //    This is almost certainly wrong in production; override with
                //    `RateLimitLayer::with_key_extractor`.
                tracing::warn!(
                    "rf-ratelimit: cannot determine client IP \
                     (no X-Forwarded-For, X-Real-IP, or ConnectInfo extension). \
                     All requests share one rate-limit bucket. \
                     Use RateLimitLayer::with_key_extractor to supply a per-client key."
                );
                "unknown".to_string()
            }),
        }
    }

    /// Set custom key extraction function
    pub fn with_key_extractor<F>(mut self, extractor: F) -> Self
    where
        F: Fn(&Request) -> String + Send + Sync + 'static,
    {
        self.key_extractor = Arc::new(extractor);
        self
    }

    /// Handle middleware request
    pub async fn handle(self, req: Request, next: Next) -> Response {
        let key = (self.key_extractor)(&req);

        match self.limiter.check(&key).await {
            Ok(result) => {
                if result.allowed {
                    // Request allowed - add headers and continue
                    let mut response = next.run(req).await;
                    add_rate_limit_headers(response.headers_mut(), &result);
                    response
                } else {
                    // Rate limit exceeded
                    rate_limit_exceeded_response(&result)
                }
            }
            Err(e) => {
                tracing::error!("Rate limit check failed: {}", e);
                // On error, allow request but log
                next.run(req).await
            }
        }
    }
}

/// Add rate limit headers to response
fn add_rate_limit_headers(headers: &mut HeaderMap, result: &LimitResult) {
    if let Ok(value) = HeaderValue::from_str(&result.limit.to_string()) {
        headers.insert("X-RateLimit-Limit", value);
    }
    if let Ok(value) = HeaderValue::from_str(&result.remaining.to_string()) {
        headers.insert("X-RateLimit-Remaining", value);
    }
    if let Ok(value) = HeaderValue::from_str(&result.reset_at.timestamp().to_string()) {
        headers.insert("X-RateLimit-Reset", value);
    }
}

/// Create rate limit exceeded response with `application/json` content-type.
fn rate_limit_exceeded_response(result: &LimitResult) -> Response {
    let body = serde_json::json!({
        "error": "Rate limit exceeded",
        "message": "Too many requests. Please try again later.",
        "retry_after": result.retry_after,
        "limit": result.limit,
        "remaining": result.remaining,
    });

    // axum::Json sets Content-Type: application/json automatically.
    let mut response = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();

    add_rate_limit_headers(response.headers_mut(), result);

    // Add Retry-After header
    if let Some(retry_after) = result.retry_after {
        if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
            response.headers_mut().insert("Retry-After", value);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    #[tokio::test]
    async fn test_rate_limit_headers() {
        let result = LimitResult {
            allowed: true,
            limit: 5,
            remaining: 3,
            reset_after: 60,
            reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            retry_after: None,
        };

        let mut headers = HeaderMap::new();
        add_rate_limit_headers(&mut headers, &result);

        assert_eq!(headers.get("X-RateLimit-Limit").unwrap(), "5");
        assert_eq!(headers.get("X-RateLimit-Remaining").unwrap(), "3");
        assert!(headers.contains_key("X-RateLimit-Reset"));
    }

    /// Fix (1): 429 response must carry Content-Type: application/json.
    #[tokio::test]
    async fn test_rate_limit_exceeded_response_is_json() {
        let result = LimitResult {
            allowed: false,
            limit: 10,
            remaining: 0,
            reset_after: 60,
            reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            retry_after: Some(60),
        };

        let response = rate_limit_exceeded_response(&result);

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("Retry-After"));
        assert_eq!(response.headers().get("Retry-After").unwrap(), "60");

        // Content-Type must be application/json, NOT text/plain.
        let ct = response
            .headers()
            .get("content-type")
            .expect("429 response must have Content-Type header")
            .to_str()
            .unwrap();
        assert!(
            ct.contains("application/json"),
            "Content-Type must be application/json, got: {ct}"
        );
        // Rate-limit info headers must also be present on the 429.
        assert!(response.headers().contains_key("X-RateLimit-Limit"));
        assert!(response.headers().contains_key("X-RateLimit-Remaining"));
        assert!(response.headers().contains_key("X-RateLimit-Reset"));
    }

    /// Fix (3): default extractor falls back to "unknown" when no IP header is set,
    /// and with_key_extractor provides per-client isolation.
    #[test]
    fn test_default_key_extractor_reads_x_forwarded_for() {
        use crate::{MemoryRateLimiter, RateLimitConfig};
        let config = RateLimitConfig::per_minute(10);
        let limiter = std::sync::Arc::new(MemoryRateLimiter::new(config));
        let layer = RateLimitLayer::new(limiter);

        let req_with_xff = HttpRequest::builder()
            .header("x-forwarded-for", "1.2.3.4, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        let key = (layer.key_extractor)(&req_with_xff);
        assert_eq!(key, "1.2.3.4", "should take leftmost IP from X-Forwarded-For");

        let req_with_real_ip = HttpRequest::builder()
            .header("x-real-ip", "5.6.7.8")
            .body(axum::body::Body::empty())
            .unwrap();
        let key2 = (layer.key_extractor)(&req_with_real_ip);
        assert_eq!(key2, "5.6.7.8", "should use X-Real-IP when no X-Forwarded-For");
    }

    #[test]
    fn test_with_key_extractor_isolates_clients() {
        use crate::{MemoryRateLimiter, RateLimitConfig};
        let config = RateLimitConfig::per_minute(10);
        let limiter = std::sync::Arc::new(MemoryRateLimiter::new(config));
        let layer = RateLimitLayer::new(limiter)
            .with_key_extractor(|req| {
                req.headers()
                    .get("x-client-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("anon")
                    .to_string()
            });

        let req_a = HttpRequest::builder()
            .header("x-client-id", "client-A")
            .body(axum::body::Body::empty())
            .unwrap();
        let req_b = HttpRequest::builder()
            .header("x-client-id", "client-B")
            .body(axum::body::Body::empty())
            .unwrap();

        assert_eq!((layer.key_extractor)(&req_a), "client-A");
        assert_eq!((layer.key_extractor)(&req_b), "client-B");
    }
}
