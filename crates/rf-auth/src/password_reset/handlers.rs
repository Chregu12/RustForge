//! Axum HTTP handlers for the password reset flow.
//!
//! ## Endpoints
//!
//! | Method | Path                      | Description                                |
//! |--------|---------------------------|--------------------------------------------|
//! | POST   | `/password/reset/request` | Send a password reset email                |
//! | POST   | `/password/reset`         | Consume a token and set the new password   |
//!
//! ## Setup
//!
//! ```rust,no_run
//! use rf_auth::password_reset::handlers::{PasswordResetState, request_reset, reset_password};
//! use rf_auth::password_reset::PasswordReset;
//! use rf_mail::MemoryMailer;
//! use axum::{Router, routing::post, Extension};
//! use std::{sync::Arc, time::Duration};
//!
//! let reset = Arc::new(PasswordReset::with_default_ttl("secret-32-chars-minimum!!!!".to_string()));
//! let mailer: Arc<dyn rf_mail::Mailer> = Arc::new(MemoryMailer::new());
//!
//! let state = PasswordResetState {
//!     password_reset: reset.clone(),
//!     mailer: mailer.clone(),
//!     base_url: "https://example.com".to_string(),
//! };
//!
//! let app: Router = Router::new()
//!     .route("/password/reset/request", post(request_reset))
//!     .route("/password/reset", post(reset_password))
//!     .layer(Extension(Arc::new(state)));
//! ```

use crate::{
    error::AuthError,
    password_reset::PasswordReset,
};
use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ─── Shared state ─────────────────────────────────────────────────────────────

/// Shared state injected into the password reset handlers via [`Extension`].
pub struct PasswordResetState {
    /// Password reset token manager.
    pub password_reset: Arc<PasswordReset>,
    /// Mail backend used to dispatch reset emails.
    pub mailer: Arc<dyn rf_mail::Mailer>,
    /// Application base URL used when constructing reset links.
    pub base_url: String,
}

// ─── Request / Response types ─────────────────────────────────────────────────

/// Request body for `POST /password/reset/request`.
#[derive(Debug, Deserialize)]
pub struct ResetRequestBody {
    /// User ID to issue the token for.
    pub user_id: i64,
    /// Email address that should receive the reset link.
    pub email: String,
}

/// Request body for `POST /password/reset`.
#[derive(Debug, Deserialize)]
pub struct ResetPasswordBody {
    /// The signed JWT token from the reset email.
    pub token: String,
    /// The new password the user wants to set.
    pub new_password: String,
}

/// Response body returned after a successful operation.
#[derive(Debug, Serialize)]
pub struct ResetResponse {
    pub message: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// `POST /password/reset/request`
///
/// Generates a signed password reset token and sends it to the provided address.
///
/// Returns HTTP **200** on success.
///
/// # Errors
///
/// - **400 Bad Request** — email is empty or token generation fails.
/// - **500 Internal Server Error** — the mail backend reported a delivery error.
pub async fn request_reset(
    Extension(state): Extension<Arc<PasswordResetState>>,
    Json(body): Json<ResetRequestBody>,
) -> impl IntoResponse {
    if body.email.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Email address is required"})),
        );
    }

    let url = match state
        .password_reset
        .generate_url(&state.base_url, body.user_id, &body.email)
    {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            );
        }
    };

    let mail = match build_reset_mail(&body.email, &url) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            );
        }
    };

    match state.mailer.send(mail).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"message": "Password reset email sent"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// `POST /password/reset`
///
/// Validates the reset token and returns the decoded claims so the caller can
/// update the user's password in their own storage layer.
///
/// The handler intentionally does **not** persist a new password — that would
/// require knowledge of the application's user store.  Callers should use the
/// returned `user_id` and `email` to locate the user and hash + store the
/// `new_password` themselves.
///
/// Returns HTTP **200** with `{"message", "user_id", "email"}` on success.
///
/// # Errors
///
/// - **400 Bad Request** — token is missing, malformed, expired, or
///   `new_password` is too short (< 8 characters).
pub async fn reset_password(
    Extension(state): Extension<Arc<PasswordResetState>>,
    Json(body): Json<ResetPasswordBody>,
) -> impl IntoResponse {
    if body.new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Password must be at least 8 characters"})),
        );
    }

    match state.password_reset.verify_token(&body.token) {
        Ok(claims) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Token valid — update the user password in your application",
                "user_id": claims.sub,
                "email": claims.email,
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn build_reset_mail(to_email: &str, reset_url: &str) -> Result<rf_mail::Mail, AuthError> {
    let mail = rf_mail::MailBuilder::new()
        .from(rf_mail::Address::new("noreply@example.com"))
        .to(rf_mail::Address::new(to_email))
        .subject("Reset Your Password")
        .html(format!(
            r#"<p>We received a request to reset your password.</p>
<p>Please <a href="{url}">click here</a> to set a new password.</p>
<p>Or copy this link: {url}</p>
<p>This link expires in 1 hour. If you did not request a password reset, please ignore this email.</p>"#,
            url = reset_url
        ))
        .build()
        .map_err(|e| AuthError::EmailSendFailed(e.to_string()))?;
    Ok(mail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_reset_mail() {
        let mail = build_reset_mail(
            "user@example.com",
            "https://example.com/reset-password?token=abc",
        )
        .unwrap();
        assert_eq!(mail.to[0].email, "user@example.com");
        assert_eq!(mail.subject, "Reset Your Password");
    }
}
