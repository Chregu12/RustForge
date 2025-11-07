//! Authentication middleware for admin panel

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Authentication middleware
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    // TODO: Implement actual session checking
    // For now, check if path is /login
    if request.uri().path().ends_with("/login") {
        return next.run(request).await;
    }

    // In production, check session/JWT here
    // let session = request.extensions().get::<Session>();
    // if session.is_none() {
    //     return Redirect::to("/admin/login").into_response();
    // }

    next.run(request).await
}

/// CSRF protection middleware
pub async fn csrf_middleware(request: Request, next: Next) -> Response {
    // TODO: Implement CSRF token validation for POST/PUT/DELETE
    next.run(request).await
}
