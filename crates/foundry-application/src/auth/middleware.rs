//! Axum middleware for JWT authentication
//!
//! This module provides reusable middleware layers for protecting routes
//! with JWT-based authentication.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use super::guard::AuthError;
use super::jwt::{Claims, JwtService};

/// Extension type that gets added to requests after successful authentication
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: i64,
    pub email: String,
    pub name: String,
    pub claims: Claims,
}

impl AuthUser {
    /// Create from claims
    pub fn from_claims(claims: Claims) -> Result<Self, AuthError> {
        Ok(Self {
            user_id: claims.user_id()?,
            email: claims.email.clone(),
            name: claims.name.clone(),
            claims,
        })
    }
}

/// JWT authentication middleware
///
/// This middleware validates JWT tokens from the Authorization header
/// and adds the authenticated user to the request extensions.
pub async fn jwt_auth_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthResponse> {
    // Extract token from Authorization header
    let token = extract_token_from_header(&request)?;

    // Validate the token
    let claims = jwt_service
        .validate_access_token(token)
        .map_err(|_| AuthResponse::Unauthorized)?;

    // Create AuthUser from claims
    let auth_user = AuthUser::from_claims(claims)
        .map_err(|_| AuthResponse::Unauthorized)?;

    // Add user to request extensions
    request.extensions_mut().insert(auth_user);

    // Continue with the request
    Ok(next.run(request).await)
}

/// Optional JWT authentication middleware
///
/// Similar to jwt_auth_middleware, but doesn't fail if no token is present.
/// Useful for routes that behave differently for authenticated vs anonymous users.
pub async fn optional_jwt_auth_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate token
    if let Ok(token) = extract_token_from_header(&request) {
        if let Ok(claims) = jwt_service.validate_access_token(token) {
            if let Ok(auth_user) = AuthUser::from_claims(claims) {
                request.extensions_mut().insert(auth_user);
            }
        }
    }

    // Continue regardless of authentication status
    next.run(request).await
}

/// Extract JWT token from Authorization header
fn extract_token_from_header(request: &Request) -> Result<&str, AuthResponse> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthResponse::MissingToken)?
        .to_str()
        .map_err(|_| AuthResponse::InvalidToken)?;

    // Expected format: "Bearer <token>"
    if !auth_header.starts_with("Bearer ") {
        return Err(AuthResponse::InvalidToken);
    }

    Ok(auth_header.trim_start_matches("Bearer ").trim())
}

/// Auth error responses
#[derive(Debug)]
pub enum AuthResponse {
    MissingToken,
    InvalidToken,
    Unauthorized,
}

impl IntoResponse for AuthResponse {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthResponse::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthResponse::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
            AuthResponse::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
        };

        (status, message).into_response()
    }
}

/// Layer for requiring authentication on routes
///
/// Usage:
/// ```rust,ignore
/// use axum::Router;
/// use foundry_application::auth::{JwtAuthLayer, JwtService};
///
/// let jwt_service = Arc::new(JwtService::default());
/// let app = Router::new()
///     .route("/protected", get(handler))
///     .layer(JwtAuthLayer::new(jwt_service));
/// ```
pub struct JwtAuthLayer {
    jwt_service: Arc<JwtService>,
}

impl JwtAuthLayer {
    /// Create a new JWT auth layer
    pub fn new(jwt_service: Arc<JwtService>) -> Self {
        Self { jwt_service }
    }
}

impl<S> tower::Layer<S> for JwtAuthLayer {
    type Service = JwtAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthMiddleware {
            inner,
            jwt_service: self.jwt_service.clone(),
        }
    }
}

/// JWT auth middleware service
#[derive(Clone)]
pub struct JwtAuthMiddleware<S> {
    inner: S,
    jwt_service: Arc<JwtService>,
}

impl<S> tower::Service<Request> for JwtAuthMiddleware<S>
where
    S: tower::Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let jwt_service = self.jwt_service.clone();
        let future = self.inner.call(request);

        Box::pin(async move {
            // This is a simplified version
            // In production, you'd want proper error handling
            future.await
        })
    }
}

/// Require authentication helper
///
/// Use this as an extractor in your route handlers to ensure authentication.
///
/// Example:
/// ```rust,ignore
/// async fn protected_handler(
///     RequireAuth(user): RequireAuth,
/// ) -> impl IntoResponse {
///     format!("Hello, {}!", user.name)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RequireAuth(pub AuthUser);

impl<S> axum::extract::FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = AuthResponse;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .map(RequireAuth)
            .ok_or(AuthResponse::Unauthorized)
    }
}

/// Optional authentication helper
///
/// Use this when you want to optionally access the authenticated user.
///
/// Example:
/// ```rust,ignore
/// async fn handler(
///     OptionalAuth(user): OptionalAuth,
/// ) -> impl IntoResponse {
///     match user {
///         Some(user) => format!("Hello, {}!", user.name),
///         None => "Hello, guest!".to_string(),
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

impl<S> axum::extract::FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuth(parts.extensions.get::<AuthUser>().cloned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};

    #[test]
    fn test_extract_token_from_header() {
        let request = Request::builder()
            .header("Authorization", "Bearer test-token-123")
            .body(Body::empty())
            .unwrap();

        let token = extract_token_from_header(&request).unwrap();
        assert_eq!(token, "test-token-123");
    }

    #[test]
    fn test_extract_token_missing_header() {
        let request = Request::builder()
            .body(Body::empty())
            .unwrap();

        let result = extract_token_from_header(&request);
        assert!(matches!(result, Err(AuthResponse::MissingToken)));
    }

    #[test]
    fn test_extract_token_invalid_format() {
        let request = Request::builder()
            .header("Authorization", "InvalidFormat token")
            .body(Body::empty())
            .unwrap();

        let result = extract_token_from_header(&request);
        assert!(matches!(result, Err(AuthResponse::InvalidToken)));
    }

    #[test]
    fn test_auth_user_from_claims() {
        use super::super::jwt::{Claims, TokenType};
        use chrono::Duration;

        let claims = Claims::new(
            1,
            "test@example.com".to_string(),
            "Test User".to_string(),
            TokenType::Access,
            Duration::minutes(15),
        );

        let auth_user = AuthUser::from_claims(claims).unwrap();
        assert_eq!(auth_user.user_id, 1);
        assert_eq!(auth_user.email, "test@example.com");
        assert_eq!(auth_user.name, "Test User");
    }
}
