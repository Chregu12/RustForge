//! OAuth2 route handlers
//!
//! This module provides ready-to-use route handlers for OAuth2 flows.
//! These can be integrated into any web framework.

use crate::manager::SocialiteManager;
use crate::{SocialiteError, User};
use serde::Deserialize;

/// Request to initiate OAuth flow
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    /// OAuth provider name
    pub provider: String,
}

/// OAuth callback parameters
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    /// Authorization code from provider
    pub code: String,

    /// State parameter for CSRF protection
    pub state: Option<String>,

    /// Error from provider (if any)
    pub error: Option<String>,

    /// Error description
    pub error_description: Option<String>,
}

/// OAuth redirect handler
///
/// Generates the authorization URL and redirects the user to the OAuth provider
///
/// # Example
///
/// ```no_run
/// use rf_socialite::routes::redirect_to_provider;
/// use rf_socialite::manager::SocialiteManager;
///
/// async fn handle_redirect(provider: String) -> Result<String, String> {
///     let manager = SocialiteManager::from_env();
///     redirect_to_provider(&manager, &provider, true)
///         .map_err(|e| e.to_string())
/// }
/// ```
pub fn redirect_to_provider(
    manager: &SocialiteManager,
    provider: &str,
    use_pkce: bool,
) -> Result<String, SocialiteError> {
    let state = manager.generate_state();

    let mut builder = manager.driver(provider)?.state(state);

    if use_pkce {
        builder = builder.with_pkce();
    }

    let mut driver = builder.build()?;
    driver.redirect()
}

/// OAuth callback handler
///
/// Handles the OAuth callback, verifies state, and exchanges code for user info
///
/// # Example
///
/// ```no_run
/// use rf_socialite::routes::{handle_callback, CallbackParams};
/// use rf_socialite::manager::SocialiteManager;
///
/// async fn handle_oauth_callback(
///     provider: String,
///     params: CallbackParams,
/// ) -> Result<(), String> {
///     let manager = SocialiteManager::from_env();
///     let user = handle_callback(&manager, &provider, params).await
///         .map_err(|e| e.to_string())?;
///
///     println!("User logged in: {}", user.name);
///     Ok(())
/// }
/// ```
pub async fn handle_callback(
    manager: &SocialiteManager,
    provider: &str,
    params: CallbackParams,
) -> Result<User, SocialiteError> {
    // Check for OAuth errors
    if let Some(error) = params.error {
        let description = params.error_description.unwrap_or_default();
        return Err(SocialiteError::OAuthError(format!(
            "{}: {}",
            error, description
        )));
    }

    // Verify state for CSRF protection
    if let Some(state) = params.state {
        if !manager.verify_state(&state) {
            return Err(SocialiteError::OAuthError(
                "Invalid or expired state parameter".to_string(),
            ));
        }
    }

    // Exchange code for user info
    let driver = manager.driver(provider)?.build()?;
    driver.user_from_code(&params.code).await
}

/// Helper to generate OAuth routes for common web frameworks
pub struct RouteHelper;

impl RouteHelper {
    /// Get the authorization route path
    pub fn auth_path() -> &'static str {
        "/auth/{provider}"
    }

    /// Get the callback route path
    pub fn callback_path() -> &'static str {
        "/auth/{provider}/callback"
    }

    /// Generate callback URL for a provider
    pub fn callback_url(base_url: &str, provider: &str) -> String {
        format!(
            "{}/auth/{}/callback",
            base_url.trim_end_matches('/'),
            provider
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_paths() {
        assert_eq!(RouteHelper::auth_path(), "/auth/{provider}");
        assert_eq!(RouteHelper::callback_path(), "/auth/{provider}/callback");
    }

    #[test]
    fn test_callback_url() {
        let url = RouteHelper::callback_url("http://localhost:8000", "github");
        assert_eq!(url, "http://localhost:8000/auth/github/callback");
    }

    #[test]
    fn test_callback_url_with_trailing_slash() {
        let url = RouteHelper::callback_url("http://localhost:8000/", "google");
        assert_eq!(url, "http://localhost:8000/auth/google/callback");
    }
}
