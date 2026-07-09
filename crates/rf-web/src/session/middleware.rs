//! Session configuration for HTTP cookie handling.
//!
//! `SessionConfig` and `SameSite` describe how the session cookie should be
//! set on responses.  The actual session middleware is `session_scope` in
//! `crates/rf-web/src/session_facade.rs`, which is the single working HTTP
//! session system in rf-web.

/// Session cookie configuration.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Cookie name for session ID
    pub cookie_name: String,
    /// Cookie lifetime in seconds (None = session cookie)
    pub lifetime: Option<i64>,
    /// Cookie path
    pub path: String,
    /// Cookie domain
    pub domain: Option<String>,
    /// Secure flag (HTTPS only)
    pub secure: bool,
    /// HTTP only flag
    pub http_only: bool,
    /// SameSite policy
    pub same_site: SameSite,
}

/// SameSite cookie policy.
#[derive(Clone, Debug)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "session_id".to_string(),
            lifetime: Some(7200), // 2 hours
            path: "/".to_string(),
            domain: None,
            secure: false,
            http_only: true,
            same_site: SameSite::Lax,
        }
    }
}

impl SessionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    pub fn lifetime(mut self, seconds: i64) -> Self {
        self.lifetime = Some(seconds);
        self
    }

    pub fn session_cookie(mut self) -> Self {
        self.lifetime = None;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_builder() {
        let config = SessionConfig::new()
            .cookie_name("my_session")
            .lifetime(3600)
            .path("/app")
            .domain("example.com")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Strict);

        assert_eq!(config.cookie_name, "my_session");
        assert_eq!(config.lifetime, Some(3600));
        assert_eq!(config.path, "/app");
        assert_eq!(config.domain, Some("example.com".to_string()));
        assert!(config.secure);
        assert!(config.http_only);
    }

    #[test]
    fn test_session_config_session_cookie() {
        let config = SessionConfig::new().session_cookie();
        assert!(config.lifetime.is_none());
    }
}
