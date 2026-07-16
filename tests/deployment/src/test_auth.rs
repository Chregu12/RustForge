//! Deployment tests for rf-auth

#[cfg(test)]
mod tests {
    use rf_auth::prelude::*;
    use rf_auth::{Auth, AuthManager, Guard};
    use rf_auth::auth_manager::with_auth_scope_sync;
    use serde::{Deserialize, Serialize};

    // ── Password Hashing ─────────────────────────────────────────

    #[test]
    fn bcrypt_hash_and_verify() {
        let hasher = PasswordHasher::bcrypt(4).expect("valid cost");
        let hash = hasher.hash("password123").expect("hash");
        assert!(hasher.verify("password123", &hash).expect("verify"));
        assert!(!hasher.verify("wrong_password", &hash).expect("verify"));
    }

    #[test]
    fn argon2_hash_and_verify() {
        let hasher = PasswordHasher::argon2().expect("argon2");
        let hash = hasher.hash("secure_pass").expect("hash");
        assert!(hasher.verify("secure_pass", &hash).expect("verify"));
        assert!(!hasher.verify("wrong", &hash).expect("verify"));
    }

    #[test]
    fn default_hasher() {
        let hasher = PasswordHasher::default();
        let hash = hasher.hash("test").expect("hash");
        assert!(hasher.verify("test", &hash).expect("verify"));
    }

    #[test]
    fn bcrypt_auto_detects_algorithm() {
        let bcrypt = PasswordHasher::bcrypt(4).expect("bcrypt");
        let hash = bcrypt.hash("test").expect("hash");
        // Argon2 hasher should still verify bcrypt hashes (auto-detection)
        let argon2 = PasswordHasher::argon2().expect("argon2");
        assert!(argon2.verify("test", &hash).expect("verify"));
    }

    #[test]
    fn timing_safe_verify() {
        let hasher = PasswordHasher::bcrypt(4).expect("bcrypt");
        let hash = hasher.hash("secret").expect("hash");
        assert!(hasher.verify_timing_safe("secret", &hash).expect("verify"));
        assert!(!hasher.verify_timing_safe("wrong", &hash).expect("verify"));
    }

    #[test]
    fn invalid_bcrypt_cost_rejected() {
        assert!(PasswordHasher::bcrypt(1).is_err());
        assert!(PasswordHasher::bcrypt(50).is_err());
    }

    // ── JWT ──────────────────────────────────────────────────────

    #[test]
    fn jwt_claims_creation() {
        let claims = Claims::new(1, "user@example.com".into(), vec!["admin".into()], 24);
        assert_eq!(claims.user_id, 1);
        assert_eq!(claims.sub, "user@example.com");
        assert!(claims.has_role("admin"));
        assert!(!claims.has_role("moderator"));
        assert!(!claims.is_expired());
    }

    #[test]
    fn jwt_role_checks() {
        let claims = Claims::new(1, "u@e.com".into(), vec!["admin".into(), "editor".into()], 1);
        assert!(claims.has_any_role(&["admin", "superadmin"]));
        assert!(!claims.has_any_role(&["superadmin", "guest"]));
        assert!(claims.has_all_roles(&["admin", "editor"]));
        assert!(!claims.has_all_roles(&["admin", "superadmin"]));
    }

    #[test]
    fn jwt_generate_and_validate() {
        let manager = JwtManager::new("my-super-secret-key-that-is-long-enough").expect("manager");
        let claims = Claims::new(42, "user@test.com".into(), vec!["user".into()], 1);

        let token = manager.generate_token(&claims).expect("generate");
        assert!(!token.is_empty());

        let validated = manager.validate_token(&token).expect("validate");
        assert_eq!(validated.user_id, 42);
        assert_eq!(validated.sub, "user@test.com");
    }

    #[test]
    fn jwt_refresh_token() {
        let manager = JwtManager::new("another-secret-key-for-testing-jwt").expect("manager");
        let claims = Claims::new(1, "test@test.com".into(), vec![], 1);

        let refresh = manager.generate_refresh_token(&claims).expect("refresh");
        let validated = manager.validate_refresh_token(&refresh).expect("validate");
        assert_eq!(validated.user_id, 1);
    }

    #[test]
    fn jwt_invalid_token_rejected() {
        let manager = JwtManager::new("secret-key-for-jwt-testing-purposes").expect("manager");
        assert!(manager.validate_token("invalid.token.here").is_err());
    }

    #[test]
    fn jwt_different_secrets_incompatible() {
        let m1 = JwtManager::new("secret-one-is-this-long-string-here").expect("m1");
        let m2 = JwtManager::new("secret-two-is-different-long-string").expect("m2");
        let claims = Claims::new(1, "t@t.com".into(), vec![], 1);
        let token = m1.generate_token(&claims).expect("generate");
        assert!(m2.validate_token(&token).is_err());
    }

    #[test]
    fn jwt_invalid_secret_rejected() {
        assert!(JwtManager::new("").is_err());
    }

    // ── AuthManager ──────────────────────────────────────────────

    #[derive(Serialize, Deserialize, Clone)]
    struct TestUser {
        id: u64,
        email: String,
    }

    #[test]
    fn auth_manager_login_logout() {
        // Each test gets its own auth scope to avoid FALLBACK_STATE races.
        with_auth_scope_sync(|| {
            let auth = AuthManager::new();
            assert!(!auth.check());
            assert!(auth.guest());

            let user = TestUser { id: 1, email: "test@test.com".into() };
            auth.login(user).expect("login");

            assert!(auth.check());
            assert!(!auth.guest());
            assert_eq!(auth.id(), Some(1));

            let retrieved: Option<TestUser> = auth.user();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().email, "test@test.com");

            auth.logout();
            assert!(!auth.check());
            assert!(auth.guest());
            assert_eq!(auth.id(), None);
        });
    }

    #[test]
    fn auth_manager_guards() {
        with_auth_scope_sync(|| {
            let auth = AuthManager::new();
            assert_eq!(auth.guard_name(), "web");
            auth.set_guard("api".into());
            assert_eq!(auth.guard_name(), "api");
        });
    }

    #[test]
    fn auth_manager_remember() {
        with_auth_scope_sync(|| {
            let auth = AuthManager::new();
            let user = TestUser { id: 1, email: "t@t.com".into() };
            auth.login_with_remember(user, true).expect("login");
            assert!(auth.via_remember());
        });
    }

    // ── Guard ────────────────────────────────────────────────────

    #[test]
    fn guard_creation() {
        // Guard methods require a per-request auth scope — wrap in
        // with_auth_scope_sync so the test has its own isolated scope.
        with_auth_scope_sync(|| {
            let guard = Guard::new("api");
            assert_eq!(guard.name(), "api");
            assert!(!guard.check());
            assert!(guard.guest());
        });
    }

    // ── Auth Facade ──────────────────────────────────────────────

    #[test]
    fn auth_facade_basic_operations() {
        // Auth facade methods require a per-request scope and panic without one.
        // Use with_auth_scope_sync to give this test its own isolated scope.
        with_auth_scope_sync(|| {
            assert!(!Auth::check());
            assert!(Auth::guest());

            let user = TestUser { id: 5, email: "facade@test.com".into() };
            Auth::login(user).expect("login");
            assert!(Auth::check());
            assert_eq!(Auth::id(), Some(5));

            Auth::logout();
            assert!(!Auth::check());
        });
    }

    // ── Authorization Gates ──────────────────────────────────────

    #[derive(Clone)]
    struct GateUser {
        id: i64,
        roles: Vec<String>,
    }

    #[tokio::test]
    async fn gate_define_and_check() {
        let gate: Gate<GateUser> = Gate::new();
        gate.define("admin", |user: &GateUser| {
            let has = user.roles.contains(&"admin".to_string());
            async move { has }
        });

        let admin = GateUser { id: 1, roles: vec!["admin".into()] };
        let user = GateUser { id: 2, roles: vec!["user".into()] };

        assert!(gate.allows(&admin, "admin").await);
        assert!(gate.denies(&user, "admin").await);
    }

    #[tokio::test]
    async fn gate_any_all() {
        let gate: Gate<GateUser> = Gate::new();
        gate.define("edit", |_: &GateUser| async { true });
        gate.define("delete", |u: &GateUser| {
            let is_admin = u.roles.contains(&"admin".to_string());
            async move { is_admin }
        });

        let admin = GateUser { id: 1, roles: vec!["admin".into()] };
        assert!(gate.any(&admin, &["edit", "delete"]).await);
        assert!(gate.all(&admin, &["edit", "delete"]).await);

        let user = GateUser { id: 2, roles: vec!["user".into()] };
        assert!(gate.any(&user, &["edit", "delete"]).await);
        assert!(!gate.all(&user, &["edit", "delete"]).await);
    }

    #[tokio::test]
    async fn gate_has_and_forget() {
        let gate: Gate<GateUser> = Gate::new();
        gate.define("test", |_: &GateUser| async { true });
        assert!(gate.has("test"));
        assert!(gate.forget("test"));
        assert!(!gate.has("test"));
    }

    #[tokio::test]
    async fn gate_list() {
        let gate: Gate<GateUser> = Gate::new();
        gate.define("a", |_: &GateUser| async { true });
        gate.define("b", |_: &GateUser| async { true });
        let names = gate.gates();
        assert_eq!(names.len(), 2);
    }

    // ── AuthError ────────────────────────────────────────────────

    #[test]
    fn auth_error_variants() {
        let _e1 = AuthError::InvalidCredentials;
        let _e2 = AuthError::TokenExpired;
        let _e3 = AuthError::WeakPassword { reason: "too short".into() };
        let _e4 = AuthError::UserNotFound;
        let _e5 = AuthError::EmailExists;
        let _e6 = AuthError::InvalidSecret;
        let _e7 = AuthError::AlreadyVerified;
    }

    // ── RememberMe ───────────────────────────────────────────────

    #[test]
    fn remember_me_token_lifecycle() {
        let rm = RememberMe::with_default_ttl("super-secret-remember-me-key-here".into());
        let token = rm.generate_token(42).expect("generate");
        assert!(!token.is_empty());

        let user_id = rm.verify_token(&token).expect("verify");
        assert_eq!(user_id, 42);
    }

    #[test]
    fn remember_me_token_rotation() {
        let rm = RememberMe::with_default_ttl("another-secret-remember-me-key-test".into());
        let token1 = rm.generate_token(1).expect("generate");
        let token2 = rm.rotate_token(&token1).expect("rotate");
        assert_ne!(token1, token2);
        assert_eq!(rm.verify_token(&token2).expect("verify"), 1);
    }

    #[test]
    fn remember_me_invalid_token() {
        let rm = RememberMe::with_default_ttl("secret-key-for-remember-me-tests!".into());
        assert!(rm.verify_token("invalid-token").is_err());
    }

    // ── EmailVerification ────────────────────────────────────────

    #[test]
    fn email_verification_token() {
        let ev = EmailVerification::with_default_ttl("email-verify-secret-key-testing".into());
        let token = ev.generate_token(1, "test@test.com").expect("generate");
        let claims = ev.verify_token(&token).expect("verify");
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.email, "test@test.com");
    }

    #[test]
    fn email_verification_url() {
        let ev = EmailVerification::with_default_ttl("email-verify-secret-url-test!".into());
        let url = ev.generate_url("https://example.com", 1, "u@t.com").expect("url");
        assert!(url.starts_with("https://example.com"));
        assert!(url.contains("token="));
    }

    // ── PasswordReset ────────────────────────────────────────────

    #[test]
    fn password_reset_token() {
        let pr = PasswordReset::with_default_ttl("password-reset-secret-key-test!".into());
        let token = pr.generate_token(1, "test@test.com").expect("generate");
        let claims = pr.verify_token(&token).expect("verify");
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.email, "test@test.com");
    }

    #[test]
    fn password_reset_url() {
        let pr = PasswordReset::with_default_ttl("password-reset-secret-url-test!".into());
        let url = pr.generate_url("https://app.com/reset", 1, "u@t.com").expect("url");
        assert!(url.starts_with("https://app.com/reset"));
    }

    // ── Middleware ────────────────────────────────────────────────

    #[test]
    fn require_role_success() {
        let claims = Claims::new(1, "a@b.com".into(), vec!["admin".into()], 1);
        assert!(rf_auth::middleware::require_role(&claims, "admin").is_ok());
    }

    #[test]
    fn require_role_failure() {
        let claims = Claims::new(1, "a@b.com".into(), vec!["user".into()], 1);
        assert!(rf_auth::middleware::require_role(&claims, "admin").is_err());
    }
}
