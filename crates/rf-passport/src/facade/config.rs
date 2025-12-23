//! Runtime configuration helpers for Passport facade

use crate::facade::manager::GLOBAL_PASSPORT;
use chrono::Duration;

/// Helper struct for configuring token lifetimes
pub struct TokenLifetimes;

impl TokenLifetimes {
    /// Set access token lifetime
    pub fn access_tokens_expire_in(duration: Duration) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().access_token_lifetime = duration.num_seconds();
    }

    /// Set refresh token lifetime
    pub fn refresh_tokens_expire_in(duration: Duration) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().refresh_token_lifetime = duration.num_seconds();
    }

    /// Set personal access token lifetime
    pub fn personal_access_tokens_expire_in(duration: Duration) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().personal_access_token_lifetime = Some(duration.num_seconds());
    }

    /// Set authorization code lifetime
    pub fn auth_codes_expire_in(duration: Duration) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().auth_code_lifetime = duration.num_seconds();
    }
}

/// Helper struct for enabling/disabling grants
pub struct GrantControl;

impl GrantControl {
    /// Enable password grant
    pub fn enable_password_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_password_grant = true;
    }

    /// Disable password grant
    pub fn disable_password_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_password_grant = false;
    }

    /// Enable implicit grant
    pub fn enable_implicit_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_implicit_grant = true;
    }

    /// Disable implicit grant
    pub fn disable_implicit_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_implicit_grant = false;
    }

    /// Enable client credentials grant
    pub fn enable_client_credentials_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_client_credentials_grant = true;
    }

    /// Disable client credentials grant
    pub fn disable_client_credentials_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_client_credentials_grant = false;
    }

    /// Enable authorization code grant
    pub fn enable_authorization_code_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_authorization_code_grant = true;
    }

    /// Disable authorization code grant
    pub fn disable_authorization_code_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_authorization_code_grant = false;
    }

    /// Enable refresh token grant
    pub fn enable_refresh_token_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_refresh_token_grant = true;
    }

    /// Disable refresh token grant
    pub fn disable_refresh_token_grant() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enable_refresh_token_grant = false;
    }
}

/// Helper struct for PKCE configuration
pub struct PkceControl;

impl PkceControl {
    /// Require PKCE for authorization code flow
    pub fn require_pkce(enforce: bool) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().enforce_pkce = enforce;
    }

    /// Allow plain text PKCE (not recommended)
    pub fn allow_plain_pkce(allow: bool) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.config_mut().allow_plain_pkce = allow;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_lifetimes() {
        TokenLifetimes::access_tokens_expire_in(Duration::seconds(7200));
        let manager = GLOBAL_PASSPORT.read().unwrap();
        assert_eq!(manager.config().access_token_lifetime, 7200);
    }

    #[test]
    fn test_grant_control() {
        GrantControl::enable_password_grant();
        let manager = GLOBAL_PASSPORT.read().unwrap();
        assert_eq!(manager.config().enable_password_grant, true);
    }

    #[test]
    fn test_pkce_control() {
        PkceControl::require_pkce(false);
        let manager = GLOBAL_PASSPORT.read().unwrap();
        assert_eq!(manager.config().enforce_pkce, false);
    }
}
