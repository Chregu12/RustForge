//! Middleware for requiring email verification

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::verification::Verifiable;

/// Middleware to require email verification
///
/// This middleware checks if the authenticated user has verified their email.
/// The user must implement the `Verifiable` trait and be added to request extensions.
///
/// # Example
///
/// ```no_run
/// use axum::{Router, routing::get, middleware};
/// use rf_auth::verification::{RequireVerified, Verifiable};
///
/// async fn protected_handler() -> &'static str {
///     "This route requires verified email"
/// }
///
/// # async fn example() {
/// let app = Router::new()
///     .route("/dashboard", get(protected_handler))
///     .layer(middleware::from_fn(RequireVerified::middleware::<User>));
/// # }
/// # struct User;
/// # impl Verifiable for User {
/// #     fn verification_email(&self) -> &str { "" }
/// #     fn verification_user_id(&self) -> i64 { 0 }
/// #     fn is_verified(&self) -> bool { true }
/// #     async fn mark_verified(&mut self) -> rf_auth::AuthResult<()> { Ok(()) }
/// # }
/// ```
pub struct RequireVerified;

impl RequireVerified {
    /// Middleware function to require verified email
    ///
    /// # Type Parameters
    ///
    /// * `T` - User type that implements Verifiable
    ///
    /// # Behavior
    ///
    /// - Extracts user from request extensions
    /// - Checks if user is verified
    /// - Returns 403 Forbidden if not verified
    /// - Allows request to proceed if verified
    ///
    /// # Errors
    ///
    /// Returns 401 if user is not authenticated (not in extensions)
    /// Returns 403 if user's email is not verified
    pub async fn middleware<T: Verifiable + Clone + Send + Sync + 'static>(
        req: Request,
        next: Next,
    ) -> Response {
        // Extract user from request extensions
        let user = match req.extensions().get::<T>() {
            Some(user) => user.clone(),
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Unauthenticated",
                        "message": "You must be logged in to access this resource"
                    })),
                )
                    .into_response();
            }
        };

        // Check if email is verified
        if !user.is_verified() {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "Email not verified",
                    "message": "Please verify your email address to access this resource"
                })),
            )
                .into_response();
        }

        // User is verified, allow request to proceed
        next.run(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{verification::Verifiable, AuthResult};
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn,
        routing::get,
        Extension, Router,
    };
    use chrono::{DateTime, Utc};
    use tower::ServiceExt;

    #[derive(Clone)]
    struct TestUser {
        id: i64,
        email: String,
        verified: bool,
    }

    #[async_trait]
    impl Verifiable for TestUser {
        fn verification_email(&self) -> &str {
            &self.email
        }

        fn verification_user_id(&self) -> i64 {
            self.id
        }

        fn is_verified(&self) -> bool {
            self.verified
        }

        async fn mark_verified(&mut self) -> AuthResult<()> {
            self.verified = true;
            Ok(())
        }
    }

    async fn test_handler() -> &'static str {
        "success"
    }

    #[tokio::test]
    async fn test_verified_user_allowed() {
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            verified: true,
        };

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(Extension(user))
            .layer(from_fn(RequireVerified::middleware::<TestUser>));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_unverified_user_rejected() {
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            verified: false,
        };

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(Extension(user))
            .layer(from_fn(RequireVerified::middleware::<TestUser>));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_no_user_rejected() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(from_fn(RequireVerified::middleware::<TestUser>));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
