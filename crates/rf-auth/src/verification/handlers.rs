//! Axum HTTP handlers for the email verification flow.
//!
//! Mount these handlers with [`verification_routes`] (see `routes.rs`).
//!
//! ## Endpoints
//!
//! | Method | Path                  | Description                          |
//! |--------|-----------------------|--------------------------------------|
//! | POST   | `/email/verify/send`  | (Re-)send a verification email       |
//! | GET    | `/email/verify`       | Verify the token from the email link |
//!
//! ## Setup
//!
//! ```rust,no_run
//! use rf_auth::verification::handlers::{VerificationState, send_verification, verify_email};
//! use rf_auth::verification::EmailVerification;
//! use rf_mail::MemoryMailer;
//! use axum::{Router, routing::{get, post}, Extension};
//! use std::{sync::Arc, time::Duration};
//!
//! let verification = Arc::new(EmailVerification::with_default_ttl("secret-32-chars-minimum!!!!".to_string()));
//! let mailer: Arc<dyn rf_mail::Mailer> = Arc::new(MemoryMailer::new());
//!
//! let state = VerificationState {
//!     verification: verification.clone(),
//!     mailer: mailer.clone(),
//!     base_url: "https://example.com".to_string(),
//! };
//!
//! let app: Router = Router::new()
//!     .route("/email/verify/send", post(send_verification))
//!     .route("/email/verify", get(verify_email))
//!     .layer(Extension(Arc::new(state)));
//! ```

use crate::{
    error::{AuthError, AuthResult},
    verification::EmailVerification,
};
use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ─── Shared state ─────────────────────────────────────────────────────────────

/// Shared state injected into the verification handlers via Axum [`Extension`].
pub struct VerificationState {
    /// Email verification token manager.
    pub verification: Arc<EmailVerification>,
    /// Mail backend used to dispatch verification emails.
    pub mailer: Arc<dyn rf_mail::Mailer>,
    /// Application base URL used when constructing verification links.
    pub base_url: String,
}

// ─── Request / Response types ─────────────────────────────────────────────────

/// Request body for `POST /email/verify/send`.
#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    /// User ID to issue the token for.
    pub user_id: i64,
    /// Email address that should receive the verification link.
    pub email: String,
}

/// Response body for successful verification dispatch.
#[derive(Debug, Serialize)]
pub struct SendVerificationResponse {
    pub message: String,
}

/// Query parameters for `GET /email/verify`.
#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    /// The signed JWT token delivered via email.
    pub token: String,
}

/// Response body returned after successful verification.
#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub message: String,
    pub user_id: i64,
    pub email: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// `POST /email/verify/send`
///
/// Generates a signed verification token and sends it to the provided address.
///
/// On success returns HTTP **200** with `{"message": "Verification email sent"}`.
///
/// # Errors
///
/// - **400 Bad Request** — email is empty or token generation fails.
/// - **500 Internal Server Error** — the mail backend reported a delivery error.
pub async fn send_verification(
    Extension(state): Extension<Arc<VerificationState>>,
    Json(body): Json<SendVerificationRequest>,
) -> impl IntoResponse {
    if body.email.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Email address is required"})),
        );
    }

    // Build the verification URL.
    let url = match state
        .verification
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

    // Build and send the email.
    let mail = match build_verification_mail(&body.email, &url) {
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
            Json(serde_json::json!({"message": "Verification email sent"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// `GET /email/verify?token=<JWT>`
///
/// Validates the token from the verification link.
///
/// On success returns HTTP **200** with the decoded `user_id` and `email`.
///
/// # Errors
///
/// - **400 Bad Request** — token is missing, malformed, or expired.
pub async fn verify_email(
    Extension(state): Extension<Arc<VerificationState>>,
    Query(params): Query<VerifyEmailQuery>,
) -> impl IntoResponse {
    match state.verification.verify_token(&params.token) {
        Ok(claims) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Email verified successfully",
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

fn build_verification_mail(
    to_email: &str,
    verification_url: &str,
) -> AuthResult<rf_mail::Mail> {
    let mail = rf_mail::MailBuilder::new()
        .from(rf_mail::Address::new("noreply@example.com"))
        .to(rf_mail::Address::new(to_email))
        .subject("Verify Your Email Address")
        .html(format!(
            r#"<p>Please <a href="{url}">click here</a> to verify your email address.</p>
<p>Or copy this link: {url}</p>
<p>This link expires in 24 hours.</p>"#,
            url = verification_url
        ))
        .build()
        .map_err(|e| AuthError::EmailSendFailed(e.to_string()))?;
    Ok(mail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_verification_mail() {
        let mail = build_verification_mail(
            "user@example.com",
            "https://example.com/verify-email?token=abc",
        )
        .unwrap();
        assert_eq!(mail.to[0].email, "user@example.com");
        assert_eq!(mail.subject, "Verify Your Email Address");
    }
}
