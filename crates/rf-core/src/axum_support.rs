//! Axum integration for rf-core types
//!
//! This module is only available when the `axum` feature is enabled.

use crate::error::AppError;
use crate::RequestContext;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Implement IntoResponse for AppError to convert to RFC 7807 responses
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Create a default context if none is available
        // In practice, the RequestIdMiddleware will have set this
        let ctx = RequestContext::new("/unknown", "UNKNOWN");

        let problem = self.to_problem_details(&ctx);
        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // Build response with RFC 7807 content-type
        (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/problem+json"),
            )],
            Json(problem),
        )
            .into_response()
    }
}
