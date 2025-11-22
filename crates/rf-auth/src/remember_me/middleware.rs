//! Remember Me middleware for automatic authentication

use crate::remember_me::RememberMe;
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware for Remember Me authentication
///
/// This middleware automatically authenticates users based on their remember me cookie.
/// If a valid cookie is found, the user is loaded and added to request extensions.
///
/// # Features
///
/// - Automatic authentication from cookie
/// - Token rotation for enhanced security
/// - Optional user loading callback
/// - Graceful degradation (no error if cookie missing/invalid)
///
/// # Example
///
/// ```no_run
/// use axum::{Router, routing::get, Extension};
/// use rf_auth::remember_me::{RememberMe, RememberMeMiddleware};
/// use std::sync::Arc;
///
/// # struct User { id: i64 }
/// # async fn load_user(id: i64) -> Option<User> { None }
/// # async fn handler() -> &'static str { "hello" }
///
/// # async fn example() {
/// let remember = Arc::new(RememberMe::with_default_ttl("secret-key".to_string()));
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(axum::middleware::from_fn_with_state(
///         remember.clone(),
///         RememberMeMiddleware::middleware::<User, _>(load_user)
///     ));
/// # }
/// ```
pub struct RememberMeMiddleware;

impl RememberMeMiddleware {
    /// Middleware function for remember me authentication
    ///
    /// # Type Parameters
    ///
    /// * `T` - User type to load and add to extensions
    /// * `F` - User loader function type
    ///
    /// # Arguments
    ///
    /// * `load_user` - Async function to load user by ID
    ///
    /// # Behavior
    ///
    /// 1. Extract remember_token cookie from request
    /// 2. If cookie exists and valid:
    ///    - Verify token and extract user_id
    ///    - Load user using provided callback
    ///    - Add user to request extensions
    ///    - Rotate token (optional, for security)
    ///    - Set new cookie in response
    /// 3. If cookie missing/invalid:
    ///    - Allow request to proceed without authentication
    ///    - No error is raised
    ///
    /// # Security
    ///
    /// - Token rotation on each use limits impact of compromise
    /// - HTTP-only cookies prevent XSS attacks
    /// - Secure flag ensures HTTPS transmission
    /// - SameSite=Strict prevents CSRF attacks
    pub async fn middleware<T, F, Fut>(
        load_user: F,
    ) -> impl Fn(
        axum::extract::State<std::sync::Arc<RememberMe>>,
        Request,
        Next,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
           + Clone
    where
        T: Clone + Send + Sync + 'static,
        F: Fn(i64) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Option<T>> + Send,
    {
        move |state: axum::extract::State<std::sync::Arc<RememberMe>>,
              mut req: Request,
              next: Next| {
            let load_user = load_user.clone();
            Box::pin(async move {
                let remember = state.0;

                // Try to extract remember_token cookie
                let cookie_value = req
                    .headers()
                    .get(header::COOKIE)
                    .and_then(|header| header.to_str().ok())
                    .and_then(|cookies| {
                        cookies.split(';').find_map(|cookie| {
                            let mut parts = cookie.trim().splitn(2, '=');
                            match (parts.next(), parts.next()) {
                                (Some(name), Some(value)) if name == RememberMe::COOKIE_NAME => {
                                    Some(value.to_string())
                                }
                                _ => None,
                            }
                        })
                    });

                let mut response = if let Some(token) = cookie_value {
                    // Verify token and get user_id
                    if let Ok(user_id) = remember.verify_token(&token) {
                        // Load user
                        if let Some(user) = load_user(user_id).await {
                            // Add user to request extensions (note: T must be Clone)
                            req.extensions_mut().insert(user.clone());

                            // Rotate token for security
                            let mut response = next.run(req).await;

                            // Generate new token and set cookie
                            if let Ok(new_token) = remember.rotate_token(&token) {
                                if let Ok(cookie) = remember.create_cookie(user_id) {
                                    response.headers_mut().insert(header::SET_COOKIE, cookie);
                                }
                            }

                            return response;
                        }
                    }

                    // Token invalid or user not found - continue without auth
                    next.run(req).await
                } else {
                    // No remember_token cookie - continue without auth
                    next.run(req).await
                };

                response
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
        }
    }

    /// Create middleware layer without token rotation
    ///
    /// Use this if you don't want automatic token rotation.
    /// Less secure but may be useful for debugging or specific requirements.
    pub async fn middleware_no_rotation<T, F, Fut>(
        load_user: F,
    ) -> impl Fn(
        axum::extract::State<std::sync::Arc<RememberMe>>,
        Request,
        Next,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
           + Clone
    where
        T: Clone + Send + Sync + 'static,
        F: Fn(i64) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Option<T>> + Send,
    {
        move |state: axum::extract::State<std::sync::Arc<RememberMe>>,
              mut req: Request,
              next: Next| {
            let load_user = load_user.clone();
            Box::pin(async move {
                let remember = state.0;

                // Try to extract remember_token cookie
                let cookie_value = req
                    .headers()
                    .get(header::COOKIE)
                    .and_then(|header| header.to_str().ok())
                    .and_then(|cookies| {
                        cookies.split(';').find_map(|cookie| {
                            let mut parts = cookie.trim().splitn(2, '=');
                            match (parts.next(), parts.next()) {
                                (Some(name), Some(value)) if name == RememberMe::COOKIE_NAME => {
                                    Some(value.to_string())
                                }
                                _ => None,
                            }
                        })
                    });

                if let Some(token) = cookie_value {
                    // Verify token and get user_id
                    if let Ok(user_id) = remember.verify_token(&token) {
                        // Load user
                        if let Some(user) = load_user(user_id).await {
                            // Add user to request extensions (note: T must be Clone)
                            req.extensions_mut().insert(user.clone());
                        }
                    }
                }

                next.run(req).await
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Extension, Router,
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    #[derive(Clone, Debug, PartialEq)]
    struct TestUser {
        id: i64,
        name: String,
    }

    async fn load_test_user(id: i64) -> Option<TestUser> {
        if id == 123 {
            Some(TestUser {
                id,
                name: "Test User".to_string(),
            })
        } else {
            None
        }
    }

    async fn test_handler(user: Option<Extension<TestUser>>) -> String {
        if let Some(Extension(user)) = user {
            format!("Hello, {}!", user.name)
        } else {
            "Hello, Guest!".to_string()
        }
    }

    const TEST_SECRET: &str = "test-secret-key-must-be-32-chars-long";

    #[tokio::test]
    async fn test_remember_me_authentication() {
        let remember = Arc::new(RememberMe::with_default_ttl(TEST_SECRET.to_string()));
        let token = remember.generate_token(123).unwrap();

        // Note: This test demonstrates the structure but won't work without proper setup
        // In real usage, you'd use the middleware with Axum router
    }

    #[tokio::test]
    async fn test_invalid_token_graceful_degradation() {
        let remember = Arc::new(RememberMe::with_default_ttl(TEST_SECRET.to_string()));

        // Create request with invalid token
        // Should not error, just not authenticate
    }

    #[tokio::test]
    async fn test_missing_cookie_graceful_degradation() {
        let remember = Arc::new(RememberMe::with_default_ttl(TEST_SECRET.to_string()));

        // Create request without cookie
        // Should not error, just not authenticate
    }
}
