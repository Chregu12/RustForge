//! Controller templates for authentication

/// Login controller template
pub const LOGIN_CONTROLLER: &str = r#"use axum::{
    extract::Extension,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use rf_auth::{JwtManager, PasswordHasher};
use rf_blade::BladeEngine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

/// Show login form
pub async fn show_login_form(
    Extension(blade): Extension<Arc<BladeEngine>>,
) -> Result<Html<String>, impl IntoResponse> {
    let html = blade
        .render("auth.login", json!({
            "app_name": "RustForge",
            "old_email": "",
            "errors": ""
        }))
        .await
        .map_err(|_| Redirect::to("/"))?;

    Ok(Html(html))
}

/// Handle login request
pub async fn login(
    Extension(blade): Extension<Arc<BladeEngine>>,
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Extension(jwt): Extension<Arc<JwtManager>>,
    Form(request): Form<LoginRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // TODO: Fetch user from database
    // For now, this is a placeholder implementation

    // Verify password
    // let valid = hasher.verify(&request.password, &user.password_hash)?;
    // if !valid {
    //     return error response
    // }

    // Generate JWT token
    // let claims = Claims::new(user.id, user.email, user.roles, 24);
    // let token = jwt.generate_token(&claims)?;

    // Return success response or redirect
    Ok(Redirect::to("/dashboard"))
}

/// Handle logout
pub async fn logout() -> impl IntoResponse {
    // Clear session/token
    Redirect::to("/login")
}
"#;

/// Register controller template
pub const REGISTER_CONTROLLER: &str = r#"use axum::{
    extract::Extension,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use rf_auth::PasswordHasher;
use rf_blade::BladeEngine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub message: String,
    pub user: User,
}

/// Show registration form
pub async fn show_register_form(
    Extension(blade): Extension<Arc<BladeEngine>>,
) -> Result<Html<String>, impl IntoResponse> {
    let html = blade
        .render("auth.register", json!({
            "app_name": "RustForge",
            "old_name": "",
            "old_email": "",
            "errors": ""
        }))
        .await
        .map_err(|_| Redirect::to("/"))?;

    Ok(Html(html))
}

/// Handle registration request
pub async fn register(
    Extension(blade): Extension<Arc<BladeEngine>>,
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Form(request): Form<RegisterRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Validate passwords match
    if request.password != request.password_confirmation {
        let html = blade
            .render("auth.register", json!({
                "app_name": "RustForge",
                "old_name": request.name,
                "old_email": request.email,
                "errors": "Passwords do not match"
            }))
            .await
            .map_err(|_| Redirect::to("/register"))?;

        return Ok(Html(html));
    }

    // Hash password
    let password_hash = hasher
        .hash(&request.password)
        .map_err(|_| Redirect::to("/register"))?;

    // TODO: Save user to database
    // let user = User {
    //     name: request.name,
    //     email: request.email,
    //     password_hash,
    //     email_verified_at: None,
    // };
    // user.save()?;

    // Redirect to login
    Ok(Redirect::to("/login"))
}
"#;

/// Password reset controller template
pub const PASSWORD_RESET_CONTROLLER: &str = r#"use axum::{
    extract::{Extension, Query},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use rf_auth::{PasswordHasher, PasswordReset};
use rf_blade::BladeEngine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetQuery {
    pub token: String,
    pub email: String,
}

/// Show forgot password form
pub async fn show_forgot_password_form(
    Extension(blade): Extension<Arc<BladeEngine>>,
) -> Result<Html<String>, impl IntoResponse> {
    let html = blade
        .render("auth.forgot-password", json!({
            "app_name": "RustForge",
            "old_email": "",
            "errors": "",
            "success": ""
        }))
        .await
        .map_err(|_| Redirect::to("/"))?;

    Ok(Html(html))
}

/// Handle forgot password request
pub async fn forgot_password(
    Extension(blade): Extension<Arc<BladeEngine>>,
    Extension(password_reset): Extension<Arc<PasswordReset>>,
    Form(request): Form<ForgotPasswordRequest>,
) -> Result<Html<String>, impl IntoResponse> {
    // TODO: Check if user exists
    // Generate reset token and send email
    // password_reset.send_reset_link(&request.email).await?;

    let html = blade
        .render("auth.forgot-password", json!({
            "app_name": "RustForge",
            "old_email": request.email,
            "errors": "",
            "success": "We have emailed your password reset link!"
        }))
        .await
        .map_err(|_| Redirect::to("/forgot-password"))?;

    Ok(Html(html))
}

/// Show reset password form
pub async fn show_reset_password_form(
    Extension(blade): Extension<Arc<BladeEngine>>,
    Query(query): Query<ResetQuery>,
) -> Result<Html<String>, impl IntoResponse> {
    let html = blade
        .render("auth.reset-password", json!({
            "app_name": "RustForge",
            "token": query.token,
            "email": query.email,
            "errors": ""
        }))
        .await
        .map_err(|_| Redirect::to("/"))?;

    Ok(Html(html))
}

/// Handle reset password request
pub async fn reset_password(
    Extension(blade): Extension<Arc<BladeEngine>>,
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Extension(password_reset): Extension<Arc<PasswordReset>>,
    Form(request): Form<ResetPasswordRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Validate passwords match
    if request.password != request.password_confirmation {
        let html = blade
            .render("auth.reset-password", json!({
                "app_name": "RustForge",
                "token": request.token,
                "email": request.email,
                "errors": "Passwords do not match"
            }))
            .await
            .map_err(|_| Redirect::to("/login"))?;

        return Ok(Html(html));
    }

    // TODO: Verify token
    // password_reset.verify_token(&request.token, &request.email)?;

    // Hash new password
    let password_hash = hasher
        .hash(&request.password)
        .map_err(|_| Redirect::to("/login"))?;

    // TODO: Update user password in database
    // user.update_password(password_hash)?;

    Ok(Redirect::to("/login"))
}
"#;

/// Email verification controller template
pub const EMAIL_VERIFICATION_CONTROLLER: &str = r#"use axum::{
    extract::{Extension, Query},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use rf_auth::{EmailVerification, VerificationClaims};
use rf_blade::BladeEngine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub id: i64,
    pub hash: String,
}

/// Show email verification notice
pub async fn show_verify_email_notice(
    Extension(blade): Extension<Arc<BladeEngine>>,
) -> Result<Html<String>, impl IntoResponse> {
    let html = blade
        .render("auth.verify-email", json!({
            "app_name": "RustForge",
            "success": ""
        }))
        .await
        .map_err(|_| Redirect::to("/"))?;

    Ok(Html(html))
}

/// Handle email verification
pub async fn verify_email(
    Extension(verification): Extension<Arc<EmailVerification>>,
    Query(query): Query<VerifyQuery>,
) -> impl IntoResponse {
    // TODO: Verify email using query parameters
    // verification.verify(query.id, &query.hash).await?;

    // Mark user as verified in database

    Redirect::to("/dashboard")
}

/// Resend verification email
pub async fn resend_verification_email(
    Extension(blade): Extension<Arc<BladeEngine>>,
    Extension(verification): Extension<Arc<EmailVerification>>,
) -> Result<Html<String>, impl IntoResponse> {
    // TODO: Get current user and resend verification email
    // verification.send_verification_email(&user).await?;

    let html = blade
        .render("auth.verify-email", json!({
            "app_name": "RustForge",
            "success": "A fresh verification link has been sent!"
        }))
        .await
        .map_err(|_| Redirect::to("/email/verify"))?;

    Ok(Html(html))
}
"#;

/// Dashboard controller template
pub const DASHBOARD_CONTROLLER: &str = r#"use axum::{
    extract::Extension,
    response::{Html, IntoResponse, Redirect},
};
use rf_blade::BladeEngine;
use serde_json::json;
use std::sync::Arc;

/// Show dashboard
pub async fn show_dashboard(
    Extension(blade): Extension<Arc<BladeEngine>>,
    // Extension(user): Extension<User>, // TODO: Extract authenticated user
) -> Result<Html<String>, impl IntoResponse> {
    let html = blade
        .render("dashboard", json!({
            "app_name": "RustForge",
            "user_name": "User" // TODO: Use actual user name
        }))
        .await
        .map_err(|_| Redirect::to("/login"))?;

    Ok(Html(html))
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_templates_exist() {
        assert!(!LOGIN_CONTROLLER.is_empty());
        assert!(!REGISTER_CONTROLLER.is_empty());
        assert!(!PASSWORD_RESET_CONTROLLER.is_empty());
        assert!(!EMAIL_VERIFICATION_CONTROLLER.is_empty());
        assert!(!DASHBOARD_CONTROLLER.is_empty());
    }

    #[test]
    fn test_login_controller_has_handlers() {
        assert!(LOGIN_CONTROLLER.contains("show_login_form"));
        assert!(LOGIN_CONTROLLER.contains("login"));
        assert!(LOGIN_CONTROLLER.contains("logout"));
    }

    #[test]
    fn test_register_controller_has_handlers() {
        assert!(REGISTER_CONTROLLER.contains("show_register_form"));
        assert!(REGISTER_CONTROLLER.contains("register"));
    }

    #[test]
    fn test_password_reset_controller_has_handlers() {
        assert!(PASSWORD_RESET_CONTROLLER.contains("show_forgot_password_form"));
        assert!(PASSWORD_RESET_CONTROLLER.contains("forgot_password"));
        assert!(PASSWORD_RESET_CONTROLLER.contains("show_reset_password_form"));
        assert!(PASSWORD_RESET_CONTROLLER.contains("reset_password"));
    }
}
