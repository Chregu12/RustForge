//! OAuth2 Middleware for scope verification

use crate::{scopes::ScopeChecker, server::OAuth2Server, token::AccessToken, OAuth2Error};
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Middleware to require specific OAuth2 scopes
///
/// # Example
///
/// ```rust,ignore
/// use rf_oauth2_server::middleware::require_scopes;
///
/// let app = Router::new()
///     .route("/admin", get(admin_handler))
///     .layer(axum::middleware::from_fn(require_scopes(vec!["admin"])));
/// ```
pub fn require_scopes(
    scopes: Vec<&'static str>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, OAuth2Error>> + Send>> + Clone
{
    move |req: Request, next: Next| {
        let scopes = scopes.clone();
        Box::pin(async move {
            // Get access token from request extensions
            let token = req
                .extensions()
                .get::<AccessToken>()
                .ok_or_else(|| OAuth2Error::Unauthorized("Missing access token".to_string()))?;

            // Check if token has all required scopes
            for scope in &scopes {
                if !token.has_scope(scope) {
                    return Err(OAuth2Error::InsufficientScope(format!(
                        "Missing required scope: {}",
                        scope
                    )));
                }
            }

            Ok(next.run(req).await)
        })
    }
}

/// Middleware to require ANY of the specified OAuth2 scopes
pub fn require_any_scope(
    scopes: Vec<&'static str>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, OAuth2Error>> + Send>> + Clone
{
    move |req: Request, next: Next| {
        let scopes = scopes.clone();
        Box::pin(async move {
            let token = req
                .extensions()
                .get::<AccessToken>()
                .ok_or_else(|| OAuth2Error::Unauthorized("Missing access token".to_string()))?;

            // Check if token has any of the required scopes
            let has_any = scopes.iter().any(|scope| token.has_scope(scope));

            if !has_any {
                return Err(OAuth2Error::InsufficientScope(format!(
                    "Missing any of required scopes: {}",
                    scopes.join(", ")
                )));
            }

            Ok(next.run(req).await)
        })
    }
}

/// Middleware factory that authenticates requests via their OAuth2 bearer token.
///
/// The returned middleware reads the `Authorization: Bearer <token>` header,
/// validates the token against `server` (rejecting unknown or expired tokens with
/// `401`), and on success inserts the resolved [`AccessToken`] into the request
/// extensions so downstream layers such as [`require_scopes`] can read it.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use rf_oauth2_server::{OAuth2Server, middleware::extract_bearer_token};
///
/// let server = Arc::new(OAuth2Server::new(config));
/// let app = Router::new()
///     .route("/me", get(me))
///     .layer(axum::middleware::from_fn(extract_bearer_token(server)));
/// ```
pub fn extract_bearer_token(
    server: Arc<OAuth2Server>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, OAuth2Error>> + Send>> + Clone
{
    move |mut req: Request, next: Next| {
        let server = server.clone();
        Box::pin(async move {
            // Extract the bearer token from the Authorization header.
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| {
                    OAuth2Error::Unauthorized("Missing Authorization header".to_string())
                })?;

            let token = auth_header
                .strip_prefix("Bearer ")
                .ok_or_else(|| {
                    OAuth2Error::Unauthorized("Invalid Authorization format".to_string())
                })?
                .to_string();

            // Validate against the server's token store: unknown or expired tokens
            // are rejected here (mapped to 401) instead of silently passing through.
            let access_token = server.validate_token(&token).await?;

            // Expose the authenticated token to downstream handlers/middleware.
            req.extensions_mut().insert(access_token);

            Ok(next.run(req).await)
        })
    }
}

/// Macro for creating scope middleware more conveniently
///
/// # Example
///
/// ```rust,ignore
/// use rf_oauth2_server::require_scopes;
///
/// let app = Router::new()
///     .route("/posts", get(posts))
///     .layer(require_scopes!["read:posts", "write:posts"]);
/// ```
#[macro_export]
macro_rules! require_scopes {
    [$($scope:expr),* $(,)?] => {
        axum::middleware::from_fn($crate::middleware::require_scopes(vec![$($scope),*]))
    };
}

/// Macro for requiring any scope
#[macro_export]
macro_rules! require_any_scope {
    [$($scope:expr),* $(,)?] => {
        axum::middleware::from_fn($crate::middleware::require_any_scope(vec![$($scope),*]))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_compiles() {
        let _middleware = require_scopes(vec!["read"]);
        let _any_middleware = require_any_scope(vec!["read", "write"]);
        assert!(true);
    }
}
