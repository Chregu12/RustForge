//! OAuth2 Middleware for scope verification

use crate::{token::AccessToken, OAuth2Error};
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::future::Future;
use std::pin::Pin;

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

/// Extract and verify OAuth2 bearer token from Authorization header
pub async fn extract_bearer_token(req: Request, next: Next) -> Result<Response, OAuth2Error> {
    // Extract bearer token from Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| OAuth2Error::Unauthorized("Missing Authorization header".to_string()))?;

    let _token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| OAuth2Error::Unauthorized("Invalid Authorization format".to_string()))?;

    // TODO: Validate token against database/cache
    // For now, just pass it through
    // In production, you would:
    // 1. Look up token in database
    // 2. Check expiration
    // 3. Load scopes
    // 4. Insert AccessToken into extensions

    Ok(next.run(req).await)
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
