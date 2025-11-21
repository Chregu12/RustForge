//! Middleware for ability/scope checking

use crate::{PersonalAccessToken, SanctumError};
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::future::Future;
use std::pin::Pin;

/// Middleware to require specific abilities
///
/// # Example
///
/// ```rust,ignore
/// use rf_sanctum::middleware::require_abilities;
///
/// let app = Router::new()
///     .route("/admin", get(admin_handler))
///     .layer(axum::middleware::from_fn(require_abilities(vec!["admin"])));
/// ```
pub fn require_abilities(
    abilities: Vec<&'static str>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, SanctumError>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let abilities = abilities.clone();
        Box::pin(async move {
            // Get token from request extensions
            let token = req
                .extensions()
                .get::<PersonalAccessToken>()
                .ok_or(SanctumError::Unauthenticated)?;

            // Check if token has all required abilities
            for ability in &abilities {
                if !token.can(ability) {
                    return Err(SanctumError::InsufficientPermissions(
                        format!("Missing ability: {}", ability)
                    ));
                }
            }

            Ok(next.run(req).await)
        })
    }
}

/// Middleware to require ANY of the specified abilities
pub fn require_any_ability(
    abilities: Vec<&'static str>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, SanctumError>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let abilities = abilities.clone();
        Box::pin(async move {
            let token = req
                .extensions()
                .get::<PersonalAccessToken>()
                .ok_or(SanctumError::Unauthenticated)?;

            if !token.can_any(&abilities) {
                return Err(SanctumError::InsufficientPermissions(
                    format!("Missing any of: {}", abilities.join(", "))
                ));
            }

            Ok(next.run(req).await)
        })
    }
}

/// Macro for creating ability middleware more conveniently
///
/// # Example
///
/// ```rust,ignore
/// use rf_sanctum::require_abilities;
///
/// let app = Router::new()
///     .route("/posts", get(posts))
///     .layer(require_abilities!["read:posts", "write:posts"]);
/// ```
#[macro_export]
macro_rules! require_abilities {
    [$($ability:expr),* $(,)?] => {
        axum::middleware::from_fn($crate::middleware::require_abilities(vec![$($ability),*]))
    };
}

/// Macro for requiring any ability
#[macro_export]
macro_rules! require_any_ability {
    [$($ability:expr),* $(,)?] => {
        axum::middleware::from_fn($crate::middleware::require_any_ability(vec![$($ability),*]))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_compiles() {
        let _middleware = require_abilities(vec!["read:posts"]);
        assert!(true);
    }
}
