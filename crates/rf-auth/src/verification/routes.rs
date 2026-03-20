//! Axum router factory for the email verification endpoints.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_auth::verification::{EmailVerification, routes::verification_routes};
//! use rf_mail::MemoryMailer;
//! use axum::Router;
//! use std::{sync::Arc, time::Duration};
//!
//! let verification = Arc::new(EmailVerification::with_default_ttl(
//!     "secret-key-minimum-32-characters!!".to_string(),
//! ));
//! let mailer: Arc<dyn rf_mail::Mailer> = Arc::new(MemoryMailer::new());
//!
//! let router: Router = verification_routes(
//!     verification,
//!     mailer,
//!     "https://example.com".to_string(),
//! );
//!
//! // Merge into your main application router:
//! // let app = Router::new().merge(router);
//! ```

use crate::verification::{
    handlers::{send_verification, verify_email, VerificationState},
    EmailVerification,
};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use std::sync::Arc;

/// Build an [`axum::Router`] with the email verification endpoints pre-configured.
///
/// Mounts:
/// - `POST /email/verify/send` → [`send_verification`]
/// - `GET  /email/verify`      → [`verify_email`]
pub fn verification_routes(
    verification: Arc<EmailVerification>,
    mailer: Arc<dyn rf_mail::Mailer>,
    base_url: String,
) -> Router {
    let state = Arc::new(VerificationState {
        verification,
        mailer,
        base_url,
    });

    Router::new()
        .route("/email/verify/send", post(send_verification))
        .route("/email/verify", get(verify_email))
        .layer(Extension(state))
}
