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
    /// Create new rate limit layer with IP-based limiting
    pub fn new(limiter: Arc<dyn RateLimiter>) -> Self {
        Self {
            limiter,
            key_extractor: Arc::new(|_req| {
                // For now, use a default key
                // In production, extract from ConnectInfo<SocketAddr>
                "default".to_string()
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

/// Create rate limit exceeded response
fn rate_limit_exceeded_response(result: &LimitResult) -> Response {
    let body = serde_json::json!({
        "error": "Rate limit exceeded",
        "message": "Too many requests. Please try again later.",
        "retry_after": result.retry_after,
        "limit": result.limit,
        "remaining": result.remaining,
    });

    let mut response = (StatusCode::TOO_MANY_REQUESTS, body.to_string()).into_response();

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

    #[tokio::test]
    async fn test_rate_limit_exceeded_response() {
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
    }
}
