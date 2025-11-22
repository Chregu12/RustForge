//! Logging Middleware
//!
//! Provides request/response logging functionality

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

pub struct LoggingMiddleware;

/// Middleware function that logs requests
pub async fn log_requests(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    tracing::info!("--> {} {}", method, uri);

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    tracing::info!(
        "<-- {} {} {} ({:?})",
        method,
        uri,
        status.as_u16(),
        duration
    );

    response
}
