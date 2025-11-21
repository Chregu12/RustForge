//! Session middleware for HTTP request/response handling

use super::driver::SessionDriver;
use super::store::{Session, SessionStore};
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Session configuration
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

/// SameSite cookie policy
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

/// Session middleware
#[derive(Clone)]
pub struct SessionMiddleware {
    config: Arc<SessionConfig>,
    store: Arc<SessionStore>,
}

impl SessionMiddleware {
    pub fn new(driver: Arc<dyn SessionDriver>) -> Self {
        Self {
            config: Arc::new(SessionConfig::default()),
            store: Arc::new(SessionStore::new(driver)),
        }
    }

    pub fn with_config(driver: Arc<dyn SessionDriver>, config: SessionConfig) -> Self {
        Self {
            config: Arc::new(config),
            store: Arc::new(SessionStore::new(driver)),
        }
    }

    /// Extract session ID from cookie
    fn extract_session_id(&self, req: &Request) -> Option<String> {
        req.headers()
            .get(header::COOKIE)
            .and_then(|cookies| cookies.to_str().ok())
            .and_then(|cookie_str| {
                cookie_str
                    .split(';')
                    .find_map(|cookie| {
                        let mut parts = cookie.trim().splitn(2, '=');
                        let name = parts.next()?.trim();
                        let value = parts.next()?.trim();

                        if name == self.config.cookie_name {
                            Some(value.to_string())
                        } else {
                            None
                        }
                    })
            })
    }

    /// Build Set-Cookie header value
    fn build_cookie(&self, session_id: &str) -> String {
        let mut cookie = format!("{}={}", self.config.cookie_name, session_id);

        if let Some(lifetime) = self.config.lifetime {
            cookie.push_str(&format!("; Max-Age={}", lifetime));
        }

        cookie.push_str(&format!("; Path={}", self.config.path));

        if let Some(domain) = &self.config.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        if self.config.secure {
            cookie.push_str("; Secure");
        }

        if self.config.http_only {
            cookie.push_str("; HttpOnly");
        }

        match self.config.same_site {
            SameSite::Strict => cookie.push_str("; SameSite=Strict"),
            SameSite::Lax => cookie.push_str("; SameSite=Lax"),
            SameSite::None => cookie.push_str("; SameSite=None"),
        }

        cookie
    }

    /// Handle the session middleware
    pub async fn handle(&self, mut req: Request, next: Next) -> Response {
        // Extract or create session
        let session_id = self.extract_session_id(&req);
        let mut session = if let Some(id) = session_id {
            self.store.load(id).await.unwrap_or_else(|_| {
                // Create new session if load fails
                futures::executor::block_on(async { self.store.create().await.unwrap() })
            })
        } else {
            self.store.create().await.unwrap()
        };

        // Age flash data before request
        session.age_flash_data();

        // Store session in request extensions
        let session_id = session.id().to_string();
        req.extensions_mut().insert(session.clone());

        // Process request
        let mut response = next.run(req).await;

        // Save session if modified
        if session.is_dirty() {
            let _ = session.save().await;
        }

        // Set cookie in response
        let cookie = self.build_cookie(&session_id);
        response.headers_mut().insert(
            header::SET_COOKIE,
            cookie.parse().unwrap(),
        );

        response
    }
}

/// Axum extractor for session
impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Session>()
            .cloned()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Session not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::driver::CookieSessionDriver;
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

    #[test]
    fn test_build_cookie() {
        let driver = Arc::new(CookieSessionDriver::new());
        let config = SessionConfig::new()
            .cookie_name("sid")
            .lifetime(3600)
            .secure(true)
            .http_only(true);

        let middleware = SessionMiddleware::with_config(driver, config);
        let cookie = middleware.build_cookie("test123");

        assert!(cookie.contains("sid=test123"));
        assert!(cookie.contains("Max-Age=3600"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
    }

    #[test]
    fn test_build_cookie_with_domain() {
        let driver = Arc::new(CookieSessionDriver::new());
        let config = SessionConfig::new().domain("example.com");

        let middleware = SessionMiddleware::with_config(driver, config);
        let cookie = middleware.build_cookie("test123");

        assert!(cookie.contains("Domain=example.com"));
    }

    #[test]
    fn test_build_cookie_session() {
        let driver = Arc::new(CookieSessionDriver::new());
        let config = SessionConfig::new().session_cookie();

        let middleware = SessionMiddleware::with_config(driver, config);
        let cookie = middleware.build_cookie("test123");

        assert!(!cookie.contains("Max-Age"));
    }
}
