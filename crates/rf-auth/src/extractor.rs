//! Axum extractors for authentication
//!
//! Provides helper functions to extract claims from requests.

use crate::Claims;
use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rf_core::error::AppError;
use serde_json::json;

/// Authentication error response
#[derive(Debug)]
pub struct AuthRejection(pub AppError);

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = json!({
            "error": self.0.to_string(),
            "status": self.0.status_code(),
        });
        (status, Json(body)).into_response()
    }
}

/// Extract claims from request extensions
///
/// Use this after applying auth_layer middleware.
///
/// # Example
///
/// ```no_run
/// use rf_auth::extractor::get_claims;
/// use axum::{extract::Request, http::StatusCode};
///
/// async fn protected_handler(req: Request) -> Result<String, StatusCode> {
///     let claims = get_claims(&req).ok_or(StatusCode::UNAUTHORIZED)?;
///     Ok(format!("Hello, user {}!", claims.user_id))
/// }
/// ```
pub fn get_claims(req: &Request) -> Option<&Claims> {
    req.extensions().get::<Claims>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_rejection() {
        let rejection = AuthRejection(AppError::Unauthorized);
        let response = rejection.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
