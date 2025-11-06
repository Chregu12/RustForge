//! Authentication Middleware

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Require authentication middleware
pub struct RequireAuth;

impl RequireAuth {
    pub async fn layer(request: Request, next: Next) -> Response {
        // In a real implementation:
        // 1. Extract session cookie
        // 2. Validate session
        // 3. Load user
        // 4. Add user to request extensions
        // 5. Continue if valid, redirect to login if not

        // For now, just continue
        next.run(request).await
    }
}

/// Optional authentication middleware
pub struct OptionalAuth;

impl OptionalAuth {
    pub async fn layer(request: Request, next: Next) -> Response {
        // In a real implementation:
        // 1. Extract session cookie if present
        // 2. Validate session if present
        // 3. Load user if session valid
        // 4. Add user to request extensions if present
        // 5. Continue regardless

        next.run(request).await
    }
}
