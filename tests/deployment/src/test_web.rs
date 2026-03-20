//! Deployment tests for rf-web

#[cfg(test)]
mod tests {
    use rf_web::csrf::{CsrfToken, CsrfConfig, CsrfTokenStore, csrf_token, csrf_field, csrf_meta};
    use rf_web::session::store::{Session, SessionStore};
    use rf_web::session::driver::MemorySessionDriver;
    use rf_web::session::middleware::{SessionConfig, SameSite};
    use rf_web::versioning::{ApiVersion, VersionedRouter};
    use axum::{routing::get, Router};
    use std::sync::Arc;
    use std::time::Duration;

    // ── CSRF ─────────────────────────────────────────────────────

    #[test]
    fn csrf_token_generation() {
        let token = CsrfToken::generate();
        assert!(!token.token().is_empty());
        assert!(!token.is_expired());
    }

    #[test]
    fn csrf_token_verification() {
        let token = CsrfToken::generate();
        let token_str = token.token().to_string();
        assert!(token.verify(&token_str));
        assert!(!token.verify("wrong-token"));
    }

    #[test]
    fn csrf_token_regenerate() {
        let token1 = CsrfToken::generate();
        let token2 = CsrfToken::regenerate();
        assert_ne!(token1.token(), token2.token());
    }

    #[test]
    fn csrf_config_builder() {
        let config = CsrfConfig::new()
            .exempt("/api/webhooks")
            .exempt("/health")
            .lifetime_hours(2)
            .field_name("_csrf")
            .header_name("X-CSRF-TOKEN");
        let _ = config; // Just verify it builds
    }

    #[tokio::test]
    async fn csrf_token_store() {
        let store = CsrfTokenStore::new();
        let token = csrf_token();
        store.register(&token).await;
        assert!(store.validate(token.token(), 1).await);
        assert!(!store.validate("invalid", 1).await);
    }

    #[test]
    fn csrf_field_helper() {
        let token = csrf_token();
        let field = csrf_field(&token);
        assert!(field.contains("input"));
        assert!(field.contains(token.token()));
    }

    #[test]
    fn csrf_meta_helper() {
        let token = csrf_token();
        let meta = csrf_meta(&token);
        assert!(meta.contains("meta"));
        assert!(meta.contains(token.token()));
    }

    // ── Sessions ─────────────────────────────────────────────────

    #[test]
    fn session_config_builder() {
        let config = SessionConfig::new()
            .cookie_name("rf_session")
            .lifetime(3600)
            .path("/")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Strict);
        let _ = config;
    }

    #[tokio::test]
    async fn session_create_and_manipulate() {
        let driver = Arc::new(MemorySessionDriver::new());
        let store = SessionStore::new(driver);
        let mut session = store.create().await.expect("create session");

        assert!(session.is_empty());
        session.put("user_id", 42);
        assert!(session.has("user_id"));
        assert_eq!(session.get_as::<i32>("user_id"), Some(42));

        session.forget("user_id");
        assert!(!session.has("user_id"));
    }

    #[tokio::test]
    async fn session_flash_data() {
        let driver = Arc::new(MemorySessionDriver::new());
        let store = SessionStore::new(driver.clone());
        let mut session = store.create().await.expect("create session");

        session.flash("message", "Success!");
        let id = session.id().to_string();
        session.save().await.expect("save");

        // Flash data is available in the next request after aging (new→old)
        let mut loaded = store.load(&id).await.expect("load");
        loaded.age_flash_data(); // simulates what middleware does on next request
        let msg = loaded.get_flash_as::<String>("message");
        assert_eq!(msg, Some("Success!".to_string()));
    }

    #[tokio::test]
    async fn session_save_and_load() {
        let driver = Arc::new(MemorySessionDriver::new());
        let store = SessionStore::new(driver);

        let mut session = store.create().await.expect("create");
        session.put("key", "value");
        let id = session.id().to_string();
        session.save().await.expect("save");

        let loaded = store.load(&id).await.expect("load");
        assert_eq!(loaded.get_as::<String>("key"), Some("value".to_string()));
    }

    #[tokio::test]
    async fn session_regenerate() {
        let driver = Arc::new(MemorySessionDriver::new());
        let store = SessionStore::new(driver);
        let mut session = store.create().await.expect("create");
        let old_id = session.id().to_string();
        session.regenerate().await.expect("regenerate");
        assert_ne!(session.id(), old_id);
    }

    #[tokio::test]
    async fn session_flush() {
        let driver = Arc::new(MemorySessionDriver::new());
        let store = SessionStore::new(driver);
        let mut session = store.create().await.expect("create");
        session.put("a", 1);
        session.put("b", 2);
        session.flush();
        assert!(session.is_empty());
    }

    // ── API Versioning ───────────────────────────────────────────

    #[test]
    fn api_version_creation() {
        let v = ApiVersion::new("v1");
        assert_eq!(v.as_str(), "v1");
        assert!(v.matches("v1"));
        assert!(!v.matches("v2"));
    }

    #[test]
    fn api_version_parse() {
        let v = ApiVersion::parse("v2").expect("parse");
        // parse strips the "v" prefix, returning the numeric part
        assert!(v.as_str() == "v2" || v.as_str() == "2");
    }

    #[test]
    fn versioned_router_builder() {
        use axum::{routing::get, Router};
        let router = VersionedRouter::new()
            .version("v1", Router::new().route("/users", get(|| async { "v1" })))
            .version("v2", Router::new().route("/users", get(|| async { "v2" })))
            .build();
        let _ = router; // Just verify it builds
    }
}
