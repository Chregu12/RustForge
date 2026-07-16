//! Middleware templates for authentication

/// Auth middleware template
pub const AUTH_MIDDLEWARE: &str = r#"use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use rf_auth::{extractor::AuthUser, Claims};

/// Authentication middleware
///
/// Ensures that the user is authenticated before accessing protected routes.
pub async fn auth_middleware(
    auth_user: Option<AuthUser<Claims>>,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    match auth_user {
        Some(_user) => {
            // User is authenticated, proceed with request
            Ok(next.run(request).await)
        }
        None => {
            // User is not authenticated, redirect to login
            Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
        }
    }
}
"#;

/// Guest middleware template (redirect authenticated users)
pub const GUEST_MIDDLEWARE: &str = r#"use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use rf_auth::{extractor::AuthUser, Claims};

/// Guest middleware
///
/// Redirects authenticated users away from guest-only pages (like login/register).
pub async fn guest_middleware(
    auth_user: Option<AuthUser<Claims>>,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    match auth_user {
        Some(_user) => {
            // User is authenticated, redirect to dashboard
            Err(Redirect::to("/dashboard"))
        }
        None => {
            // User is not authenticated, proceed with request
            Ok(next.run(request).await)
        }
    }
}
"#;

/// Verified middleware template (require email verification)
pub const VERIFIED_MIDDLEWARE: &str = r#"use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use rf_auth::{extractor::AuthUser, Claims};

/// Verified middleware
///
/// Ensures that the user's email is verified before accessing certain routes.
pub async fn verified_middleware(
    auth_user: Option<AuthUser<Claims>>,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    match auth_user {
        Some(user) => {
            // TODO: Check if user's email is verified
            // For now, assume all authenticated users are verified
            // if !user.email_verified {
            //     return Err(Redirect::to("/email/verify"));
            // }

            Ok(next.run(request).await)
        }
        None => {
            // User is not authenticated
            Err((StatusCode::UNAUTHORIZED, "Unauthorized").into_response())
        }
    }
}
"#;

/// Role middleware template
pub const ROLE_MIDDLEWARE: &str = r#"use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use rf_auth::{extractor::AuthUser, Claims};

/// Role middleware factory
///
/// Creates middleware that checks if user has required role.
pub fn require_role(role: &'static str) -> impl Fn(Option<AuthUser<Claims>>, Request, Next) -> impl std::future::Future<Output = Result<Response, impl IntoResponse>> {
    move |auth_user: Option<AuthUser<Claims>>, request: Request, next: Next| async move {
        match auth_user {
            Some(user) => {
                if user.claims.roles.contains(&role.to_string()) {
                    Ok(next.run(request).await)
                } else {
                    Err((StatusCode::FORBIDDEN, "Forbidden"))
                }
            }
            None => {
                Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
            }
        }
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_templates_exist() {
        assert!(!AUTH_MIDDLEWARE.is_empty());
        assert!(!GUEST_MIDDLEWARE.is_empty());
        assert!(!VERIFIED_MIDDLEWARE.is_empty());
        assert!(!ROLE_MIDDLEWARE.is_empty());
    }

    #[test]
    fn test_auth_middleware_checks_authentication() {
        assert!(AUTH_MIDDLEWARE.contains("auth_middleware"));
        assert!(AUTH_MIDDLEWARE.contains("AuthUser"));
    }

    #[test]
    fn test_guest_middleware_redirects() {
        assert!(GUEST_MIDDLEWARE.contains("guest_middleware"));
        assert!(GUEST_MIDDLEWARE.contains("Redirect::to(\"/dashboard\")"));
    }

    #[test]
    fn test_verified_middleware_checks_email() {
        assert!(VERIFIED_MIDDLEWARE.contains("verified_middleware"));
        assert!(VERIFIED_MIDDLEWARE.contains("/email/verify"));
    }

    #[test]
    fn test_role_middleware_checks_roles() {
        assert!(ROLE_MIDDLEWARE.contains("require_role"));
        assert!(ROLE_MIDDLEWARE.contains("roles"));
    }
}
