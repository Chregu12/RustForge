//! Axum router factory for the password reset endpoints.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_auth::password_reset::{PasswordReset, routes::password_reset_routes};
//! use rf_mail::MemoryMailer;
//! use axum::Router;
//! use std::sync::Arc;
//!
//! let reset = Arc::new(PasswordReset::with_default_ttl(
//!     "secret-key-minimum-32-characters!!".to_string(),
//! ));
//! let mailer: Arc<dyn rf_mail::Mailer> = Arc::new(MemoryMailer::new());
//!
//! let router: Router = password_reset_routes(
//!     reset,
//!     mailer,
//!     "https://example.com".to_string(),
//! );
//!
//! // Merge into your main application router:
//! // let app = Router::new().merge(router);
//! ```

use crate::password_reset::{
    handlers::{request_reset, reset_password, PasswordResetState},
    PasswordReset,
};
use axum::{routing::post, Extension, Router};
use std::sync::Arc;

/// Build an [`axum::Router`] with the password reset endpoints pre-configured.
///
/// Mounts:
/// - `POST /password/reset/request` → [`request_reset`]
/// - `POST /password/reset`         → [`reset_password`]
pub fn password_reset_routes(
    password_reset: Arc<PasswordReset>,
    mailer: Arc<dyn rf_mail::Mailer>,
    base_url: String,
) -> Router {
    let state = Arc::new(PasswordResetState {
        password_reset,
        mailer,
        base_url,
    });

    Router::new()
        .route("/password/reset/request", post(request_reset))
        .route("/password/reset", post(reset_password))
        .layer(Extension(state))
}
