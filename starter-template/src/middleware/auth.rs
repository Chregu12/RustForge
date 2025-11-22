//! Authentication Middleware
//!
//! Provides JWT-based authentication middleware for protected routes

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // Subject (user email)
    pub user_id: i32,
    pub exp: usize,   // Expiration time
}

pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Extract JWT token from Authorization header
    pub fn extract_token(headers: &HeaderMap) -> Result<String, StatusCode> {
        let auth_header = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(token.to_string())
    }

    /// Validate JWT token
    pub fn validate_token(token: &str, secret: &str) -> Result<Claims, StatusCode> {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::default();

        decode::<Claims>(token, &decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }
}

/// Middleware function that requires authentication
pub async fn require_auth(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get JWT secret from environment
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "change-this-secret-in-production-min-32-chars".to_string());

    // Extract and validate token
    let token = AuthMiddleware::extract_token(&headers)?;
    let claims = AuthMiddleware::validate_token(&token, &secret)?;

    // Add claims to request extensions for use in handlers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
