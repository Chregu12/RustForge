//! Rate limit middleware for Axum

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use crate::{RateLimiter, RateLimitStorage};

pub struct RateLimitMiddleware<S: RateLimitStorage> {
    limiter: RateLimiter<S>,
}

impl<S: RateLimitStorage> RateLimitMiddleware<S> {
    pub fn new(limiter: RateLimiter<S>) -> Self {
        Self { limiter }
    }

    pub async fn handle(&self, request: Request, next: Next) -> Response {
        let key = self.extract_key(&request);

        match self.limiter.check(&key).await {
            Ok(true) => {
                let remaining = self.limiter.remaining(&key).await.unwrap_or(0);
                let mut response = next.run(request).await;

                response.headers_mut().insert(
                    "X-RateLimit-Remaining",
                    remaining.to_string().parse().unwrap(),
                );

                response
            }
            Ok(false) => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response()
            }
            Err(_) => next.run(request).await,
        }
    }

    fn extract_key(&self, request: &Request) -> String {
        // Extract IP or user ID from request
        request
            .headers()
            .get("X-Forwarded-For")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string()
    }
}
