//! Route templates for authentication

/// Auth routes template (basic)
pub const AUTH_ROUTES: &str = r#"use axum::{
    routing::{get, post},
    Router,
};

mod auth {
    pub mod login;
    pub mod register;
    pub mod password_reset;
    pub mod email_verification;
}

mod dashboard;

/// Register authentication routes
pub fn auth_routes() -> Router {
    Router::new()
        // Login routes
        .route("/login", get(auth::login::show_login_form))
        .route("/login", post(auth::login::login))
        .route("/logout", post(auth::login::logout))

        // Registration routes
        .route("/register", get(auth::register::show_register_form))
        .route("/register", post(auth::register::register))

        // Dashboard (protected)
        .route("/dashboard", get(dashboard::show_dashboard))
}
"#;

/// Auth routes template (with password reset)
pub const AUTH_ROUTES_WITH_PASSWORD_RESET: &str = r#"use axum::{
    routing::{get, post},
    Router,
};

mod auth {
    pub mod login;
    pub mod register;
    pub mod password_reset;
    pub mod email_verification;
}

mod dashboard;

/// Register authentication routes
pub fn auth_routes() -> Router {
    Router::new()
        // Login routes
        .route("/login", get(auth::login::show_login_form))
        .route("/login", post(auth::login::login))
        .route("/logout", post(auth::login::logout))

        // Registration routes
        .route("/register", get(auth::register::show_register_form))
        .route("/register", post(auth::register::register))

        // Password reset routes
        .route("/forgot-password", get(auth::password_reset::show_forgot_password_form))
        .route("/forgot-password", post(auth::password_reset::forgot_password))
        .route("/reset-password", get(auth::password_reset::show_reset_password_form))
        .route("/reset-password", post(auth::password_reset::reset_password))

        // Dashboard (protected)
        .route("/dashboard", get(dashboard::show_dashboard))
}
"#;

/// Auth routes template (full with email verification)
pub const AUTH_ROUTES_FULL: &str = r#"use axum::{
    routing::{get, post},
    Router,
};
use rf_auth::middleware::auth_layer;

mod auth {
    pub mod login;
    pub mod register;
    pub mod password_reset;
    pub mod email_verification;
}

mod dashboard;

/// Register authentication routes
pub fn auth_routes() -> Router {
    Router::new()
        // Guest routes (not authenticated)
        .merge(guest_routes())

        // Protected routes (require authentication)
        .merge(protected_routes())
}

/// Guest routes (accessible without authentication)
fn guest_routes() -> Router {
    Router::new()
        // Login routes
        .route("/login", get(auth::login::show_login_form))
        .route("/login", post(auth::login::login))

        // Registration routes
        .route("/register", get(auth::register::show_register_form))
        .route("/register", post(auth::register::register))

        // Password reset routes
        .route("/forgot-password", get(auth::password_reset::show_forgot_password_form))
        .route("/forgot-password", post(auth::password_reset::forgot_password))
        .route("/reset-password", get(auth::password_reset::show_reset_password_form))
        .route("/reset-password", post(auth::password_reset::reset_password))
}

/// Protected routes (require authentication)
fn protected_routes() -> Router {
    Router::new()
        // Logout
        .route("/logout", post(auth::login::logout))

        // Email verification
        .route("/email/verify", get(auth::email_verification::show_verify_email_notice))
        .route("/email/verify/:id/:hash", get(auth::email_verification::verify_email))
        .route("/email/verification-notification", post(auth::email_verification::resend_verification_email))

        // Dashboard
        .route("/dashboard", get(dashboard::show_dashboard))

        // Apply auth middleware to all protected routes
        .layer(axum::middleware::from_fn(auth_layer))
}
"#;

/// API auth routes template
pub const API_AUTH_ROUTES: &str = r#"use axum::{
    routing::{get, post},
    Router,
};
use rf_auth::middleware::auth_layer;

mod api {
    pub mod auth;
}

/// Register API authentication routes
pub fn api_auth_routes() -> Router {
    Router::new()
        // Public API routes
        .route("/api/login", post(api::auth::login))
        .route("/api/register", post(api::auth::register))
        .route("/api/forgot-password", post(api::auth::forgot_password))
        .route("/api/reset-password", post(api::auth::reset_password))

        // Protected API routes
        .route("/api/user", get(api::auth::user))
        .route("/api/logout", post(api::auth::logout))
        .layer(axum::middleware::from_fn(auth_layer))
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_templates_exist() {
        assert!(!AUTH_ROUTES.is_empty());
        assert!(!AUTH_ROUTES_WITH_PASSWORD_RESET.is_empty());
        assert!(!AUTH_ROUTES_FULL.is_empty());
        assert!(!API_AUTH_ROUTES.is_empty());
    }

    #[test]
    fn test_auth_routes_has_login() {
        assert!(AUTH_ROUTES.contains("/login"));
        assert!(AUTH_ROUTES.contains("show_login_form"));
    }

    #[test]
    fn test_auth_routes_has_register() {
        assert!(AUTH_ROUTES.contains("/register"));
        assert!(AUTH_ROUTES.contains("show_register_form"));
    }

    #[test]
    fn test_password_reset_routes() {
        assert!(AUTH_ROUTES_WITH_PASSWORD_RESET.contains("/forgot-password"));
        assert!(AUTH_ROUTES_WITH_PASSWORD_RESET.contains("/reset-password"));
    }

    #[test]
    fn test_full_routes_has_email_verification() {
        assert!(AUTH_ROUTES_FULL.contains("/email/verify"));
        assert!(AUTH_ROUTES_FULL.contains("verify_email"));
    }

    #[test]
    fn test_api_routes() {
        assert!(API_AUTH_ROUTES.contains("/api/login"));
        assert!(API_AUTH_ROUTES.contains("/api/register"));
        assert!(API_AUTH_ROUTES.contains("/api/user"));
    }
}
