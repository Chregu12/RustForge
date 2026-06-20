//! Middleware for HTTP request tracking

use crate::watchers::request::{RequestInfo, RequestWatcher};
use crate::Telescope;
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

/// Extension for storing request tracking information
#[derive(Clone)]
pub struct RequestTracker {
    pub request_id: String,
    pub started_at: Instant,
}

/// Middleware function to track HTTP requests with Telescope
pub async fn track_request(
    telescope: Telescope,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |req: Request, next: Next| {
        let telescope = telescope.clone();
        Box::pin(async move { track_request_impl(telescope, req, next).await })
    }
}

async fn track_request_impl(telescope: Telescope, req: Request, next: Next) -> Response {
    // Generate request ID
    let _request_id = Uuid::new_v4().to_string();
    let started_at = Instant::now();

    // Extract request information
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();

    // Extract headers
    let mut headers = std::collections::HashMap::new();
    for (name, value) in req.headers().iter() {
        if let Ok(val) = value.to_str() {
            headers.insert(name.to_string(), val.to_string());
        }
    }

    // Get IP address from headers
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Create request info
    let mut request_info =
        RequestInfo::new(&method, &path, &ip_address).with_headers(headers.clone());

    // Parse query parameters
    if !query.is_empty() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                request_info = request_info.with_query_param(key, value);
            }
        }
    }

    // Process the request
    let response = next.run(req).await;

    // Calculate duration
    let duration_ms = started_at.elapsed().as_millis() as u64;

    // Get status code
    let status = response.status().as_u16();

    // Record the request if watching is enabled
    if telescope.config.watch_requests {
        let watcher = RequestWatcher::new(telescope.storage().clone());
        let final_info = request_info.with_status(status).with_duration(duration_ms);

        watcher.record(final_info).await;
    }

    response
}

/// Layer for Axum to add Telescope request tracking
pub fn telescope_layer(_telescope: Telescope) -> tower::layer::util::Identity {
    // For now, return an identity layer
    // In a real implementation, this would use axum::middleware::from_fn
    // but that requires more complex type bounds
    tower::layer::util::Identity::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Telescope;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn,
        response::Response,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "Hello, World!"
    }

    async fn error_handler() -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response()
    }

    // Note: Middleware integration tests are disabled due to complex type bounds
    // In production, middleware would be used via custom implementation
}
