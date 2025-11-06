//! Authentication HTTP Handlers
//!
//! Axum route handlers for authentication endpoints

use crate::auth::{AuthError, AuthService, RegisterData};
use crate::templates::*;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use std::sync::Arc;

/// Authentication State
pub struct AuthState {
    pub service: AuthService,
}

/// Login Form Data
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember: bool,
}

/// Register Form Data
#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

/// Password Reset Request Form
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordForm {
    pub email: String,
}

/// Password Reset Form
#[derive(Debug, Deserialize)]
pub struct ResetPasswordForm {
    pub token: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

/// Auth Error Response
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, "Email already exists"),
            AuthError::EmailNotVerified => (StatusCode::FORBIDDEN, "Email not verified"),
            AuthError::TwoFactorRequired => (StatusCode::FORBIDDEN, "Two-factor required"),
            AuthError::InvalidTwoFactorCode => (StatusCode::UNAUTHORIZED, "Invalid 2FA code"),
            AuthError::SessionNotFound => (StatusCode::UNAUTHORIZED, "Session not found"),
            AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "Session expired"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };

        (status, message).into_response()
    }
}

/// Create authentication routes
pub fn auth_routes() -> Router<Arc<AuthState>> {
    Router::new()
        .route("/login", get(show_login).post(login))
        .route("/register", get(show_register).post(register))
        .route("/logout", post(logout))
        .route("/password/forgot", get(show_forgot_password).post(forgot_password))
        .route("/password/reset", get(show_reset_password).post(reset_password))
        .route("/email/verify/:token", get(verify_email))
}

/// Show login page
async fn show_login() -> Html<String> {
    Html(LoginTemplate {}.render())
}

/// Handle login submission
async fn login(
    State(_state): State<Arc<AuthState>>,
    Form(_form): Form<LoginForm>,
) -> Result<Redirect, AuthError> {
    // In a real implementation:
    // 1. Look up user by email
    // 2. Verify password
    // 3. Create session
    // 4. Set session cookie
    // 5. Redirect to dashboard

    // For now, return a redirect
    Ok(Redirect::to("/dashboard"))
}

/// Show registration page
async fn show_register() -> Html<String> {
    Html(RegisterTemplate {}.render())
}

/// Handle registration submission
async fn register(
    State(state): State<Arc<AuthState>>,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, AuthError> {
    let data = RegisterData {
        name: form.name,
        email: form.email,
        password: form.password,
        password_confirmation: form.password_confirmation,
    };

    // Register user
    let _user = state.service.register(data)?;

    // In a real implementation:
    // 1. Store user in database
    // 2. Send email verification if required
    // 3. Create session
    // 4. Redirect to dashboard or email verification notice

    Ok(Redirect::to("/dashboard"))
}

/// Handle logout
async fn logout() -> Redirect {
    // In a real implementation:
    // 1. Invalidate session
    // 2. Clear session cookie
    // 3. Redirect to login

    Redirect::to("/login")
}

/// Show forgot password page
async fn show_forgot_password() -> Html<String> {
    Html(ForgotPasswordTemplate {}.render())
}

/// Handle forgot password submission
async fn forgot_password(
    State(_state): State<Arc<AuthState>>,
    Form(_form): Form<ForgotPasswordForm>,
) -> Result<Redirect, AuthError> {
    // In a real implementation:
    // 1. Look up user by email
    // 2. Generate password reset token
    // 3. Store token in database
    // 4. Send password reset email
    // 5. Redirect to confirmation page

    Ok(Redirect::to("/password/reset/sent"))
}

/// Show reset password page
async fn show_reset_password() -> Html<String> {
    Html(ResetPasswordTemplate {
        token: "".to_string(),
        email: "".to_string(),
    }.render())
}

/// Handle password reset submission
async fn reset_password(
    State(_state): State<Arc<AuthState>>,
    Form(_form): Form<ResetPasswordForm>,
) -> Result<Redirect, AuthError> {
    // In a real implementation:
    // 1. Validate token
    // 2. Update user password
    // 3. Invalidate token
    // 4. Redirect to login

    Ok(Redirect::to("/login"))
}

/// Handle email verification
async fn verify_email() -> Result<Redirect, AuthError> {
    // In a real implementation:
    // 1. Validate token
    // 2. Mark email as verified
    // 3. Invalidate token
    // 4. Redirect to dashboard

    Ok(Redirect::to("/dashboard"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthConfig;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn create_app() -> Router {
        let config = AuthConfig::default();
        let service = AuthService::new(config);
        let state = Arc::new(AuthState { service });

        auth_routes().with_state(state)
    }

    #[tokio::test]
    async fn test_show_login() {
        let app = create_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/login")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_show_register() {
        let app = create_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/register")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
