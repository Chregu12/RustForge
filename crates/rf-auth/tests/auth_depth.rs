//! Deep integration tests for rf-auth — JwtManager, PasswordHasher, AuthManager.
//!
//! Covers the critical paths the external audit flagged as untested:
//!   - JWT issue → validate round-trip with full claims preservation
//!   - Expired / tampered / wrong-secret token rejection
//!   - PasswordHasher bcrypt + argon2 hash→verify round-trip and wrong password
//!   - Timing-safe verify (both algorithms)
//!   - AuthManager::attempt fail-closed edge cases (no provider, missing fields)
//!   - Password stripped from current_user after login
//!   - Login → check → logout → check cycle
//!   - has_role / id() guards

use rf_auth::{
    auth_manager::{with_auth_scope, with_auth_scope_sync, AuthManager, UserProvider},
    error::AuthError,
    Claims, JwtManager, PasswordHasher,
};
use serde_json::{json, Value};
use std::sync::Arc;

const SECRET: &str = "test-secret-key-exactly-32-chars!"; // exactly 32 chars

// ============================================================================
// JwtManager — basic creation
// ============================================================================

#[test]
fn jwt_manager_rejects_secret_shorter_than_32_chars() {
    assert!(JwtManager::new("short").is_err());
    // 31 chars: still too short.
    assert!(JwtManager::new("1234567890123456789012345678901").is_err());
}

#[test]
fn jwt_manager_accepts_32_char_secret() {
    assert!(JwtManager::new(SECRET).is_ok());
}

// ============================================================================
// JWT issue → validate round-trip: ALL claims fields preserved
// ============================================================================

#[test]
fn jwt_round_trip_preserves_all_claims_fields() {
    let jwt = JwtManager::new(SECRET).unwrap();
    let original = Claims::new(
        42,
        "alice@example.com".to_string(),
        vec!["user".to_string(), "moderator".to_string()],
        2, // 2 hours
    );

    let token = jwt.generate_token(&original).unwrap();
    let decoded = jwt.validate_token(&token).unwrap();

    assert_eq!(decoded.user_id, 42);
    assert_eq!(decoded.sub, "alice@example.com");
    assert_eq!(decoded.roles, vec!["user", "moderator"]);
    assert_eq!(decoded.jti, original.jti, "jti must survive the round-trip");
    assert_eq!(decoded.iat, original.iat, "iat must survive");
    assert_eq!(decoded.exp, original.exp, "exp must survive");
    assert!(!decoded.is_expired());
}

// ============================================================================
// JWT: expired token rejected
// ============================================================================

#[test]
fn jwt_expired_token_is_rejected() {
    let jwt = JwtManager::new(SECRET).unwrap();
    let mut claims = Claims::new(1, "test@example.com".to_string(), vec![], 1);

    // Back-date the expiry to the past.
    claims.exp = chrono::Utc::now().timestamp() - 3600; // expired 1 hour ago

    let token = jwt.generate_token(&claims).unwrap();
    let result = jwt.validate_token(&token);

    assert!(result.is_err(), "expired token must be rejected");
    // The error must be a JWT error (not some other auth error).
    match result.unwrap_err() {
        AuthError::JwtError(_) => {}
        other => panic!("expected AuthError::JwtError, got {:?}", other),
    }
}

// ============================================================================
// JWT: tampered payload rejected
// ============================================================================

#[test]
fn jwt_tampered_payload_is_rejected() {
    let jwt = JwtManager::new(SECRET).unwrap();
    let claims = Claims::new(1, "victim@example.com".to_string(), vec![], 24);
    let token = jwt.generate_token(&claims).unwrap();

    // A JWT has three base64url-encoded parts: header.payload.signature.
    // Changing ANY byte in the payload invalidates the HMAC signature.
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "valid JWT must have 3 parts");

    // Decode payload, flip a bit, re-encode.
    let mut payload_bytes = base64_url_decode(parts[1]);
    if let Some(b) = payload_bytes.first_mut() {
        *b ^= 0x01; // flip one bit
    }
    let tampered_payload = base64_url_encode(&payload_bytes);

    let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);
    let result = jwt.validate_token(&tampered_token);
    assert!(
        result.is_err(),
        "token with tampered payload must be rejected"
    );
}

/// Minimal base64url decoder (no padding) for test purposes.
fn base64_url_decode(input: &str) -> Vec<u8> {
    // base64url: + → -, / → _; no padding.
    let standard = input.replace('-', "+").replace('_', "/");
    let padded = match standard.len() % 4 {
        0 => standard,
        2 => standard + "==",
        3 => standard + "=",
        _ => standard + "=",
    };
    // Use the base64 crate via the fact it's already a transitive dependency
    // (jsonwebtoken pulls it in). We call the stdlib's decode manually.
    // Simple implementation using only std:
    let alphabet =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let chars: Vec<u8> = padded
        .bytes()
        .filter(|b| *b != b'=')
        .map(|b| {
            alphabet
                .iter()
                .position(|&a| a == b)
                .unwrap_or(0) as u8
        })
        .collect();
    for chunk in chars.chunks(4) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let b3 = *chunk.get(3).unwrap_or(&0);
        out.push((b0 << 2) | (b1 >> 4));
        if chunk.len() > 2 {
            out.push((b1 << 4) | (b2 >> 2));
        }
        if chunk.len() > 3 {
            out.push((b2 << 6) | b3);
        }
    }
    out
}

fn base64_url_encode(input: &[u8]) -> String {
    let alphabet =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(alphabet[(b0 >> 2) as usize] as char);
        out.push(alphabet[((b0 & 3) << 4 | b1 >> 4) as usize] as char);
        if chunk.len() > 1 {
            out.push(alphabet[((b1 & 0xf) << 2 | b2 >> 6) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(alphabet[(b2 & 0x3f) as usize] as char);
        }
    }
    // Replace standard base64 chars with base64url and strip padding.
    out.replace('+', "-").replace('/', "_")
}

// ============================================================================
// JWT: wrong secret rejected
// ============================================================================

#[test]
fn jwt_wrong_secret_rejects_token() {
    let jwt_a = JwtManager::new("secret-aaa-exactly-32-characters!").unwrap(); // 34 chars
    let jwt_b = JwtManager::new("secret-bbb-exactly-32-characters!").unwrap(); // 34 chars

    let claims = Claims::new(1, "test@example.com".to_string(), vec![], 1);
    let token = jwt_a.generate_token(&claims).unwrap();

    // Must be rejected by jwt_b (different secret).
    assert!(
        jwt_b.validate_token(&token).is_err(),
        "token signed with secret-a must fail validation under secret-b"
    );
    // Must be accepted by jwt_a.
    assert!(
        jwt_a.validate_token(&token).is_ok(),
        "token must validate under its own secret"
    );
}

// ============================================================================
// JWT: garbage / empty token rejected
// ============================================================================

#[test]
fn jwt_garbage_token_is_rejected() {
    let jwt = JwtManager::new(SECRET).unwrap();
    assert!(jwt.validate_token("").is_err());
    assert!(jwt.validate_token("not.a.jwt").is_err());
    assert!(jwt.validate_token("eyJhbGciOiJub25lIn0.e30.").is_err()); // alg=none attack
}

// ============================================================================
// JWT: refresh token gets new JTI
// ============================================================================

#[test]
fn jwt_refresh_token_has_different_jti() {
    let jwt = JwtManager::new(SECRET).unwrap();
    let claims = Claims::new(1, "test@example.com".to_string(), vec![], 1);

    let refresh = jwt.generate_refresh_token(&claims).unwrap();
    let decoded = jwt.validate_refresh_token(&refresh).unwrap();

    // Refresh must have a DIFFERENT jti from the original claims.
    assert_ne!(
        decoded.jti, claims.jti,
        "refresh token must have a distinct jti"
    );
    // user_id and sub must be the same.
    assert_eq!(decoded.user_id, 1);
    assert_eq!(decoded.sub, "test@example.com");
}

// ============================================================================
// Claims helpers
// ============================================================================

#[test]
fn claims_has_role_helpers() {
    let claims = Claims::new(
        1,
        "u@example.com".to_string(),
        vec!["admin".to_string(), "editor".to_string()],
        1,
    );

    assert!(claims.has_role("admin"));
    assert!(claims.has_role("editor"));
    assert!(!claims.has_role("viewer"));

    assert!(claims.has_any_role(&["admin", "superuser"]));
    assert!(!claims.has_any_role(&["superuser", "viewer"]));

    assert!(claims.has_all_roles(&["admin", "editor"]));
    assert!(!claims.has_all_roles(&["admin", "viewer"]));
}

// ============================================================================
// PasswordHasher — bcrypt round-trip
// ============================================================================

#[test]
fn bcrypt_hash_and_verify_round_trip() {
    let hasher = PasswordHasher::bcrypt(4).unwrap(); // cost 4 for test speed
    let password = "correct-horse-battery-staple";

    let hash = hasher.hash(password).unwrap();
    assert!(hash.starts_with("$2"), "bcrypt hash must start with $2");

    // Correct password → true.
    assert!(hasher.verify(password, &hash).unwrap());
    // Wrong password → false (not an error, just false).
    assert!(!hasher.verify("wrong-password", &hash).unwrap());
}

// ============================================================================
// PasswordHasher — argon2 round-trip
// ============================================================================

#[test]
fn argon2_hash_and_verify_round_trip() {
    let hasher = PasswordHasher::argon2().unwrap();
    let password = "correct-horse-battery-staple";

    let hash = hasher.hash(password).unwrap();
    assert!(
        hash.starts_with("$argon2"),
        "argon2 hash must start with $argon2"
    );

    assert!(hasher.verify(password, &hash).unwrap());
    assert!(!hasher.verify("wrong", &hash).unwrap());
}

// ============================================================================
// PasswordHasher — timing-safe verify delegates correctly
// ============================================================================

#[test]
fn timing_safe_verify_matches_verify() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("password").unwrap();

    assert!(hasher.verify_timing_safe("password", &hash).unwrap());
    assert!(!hasher.verify_timing_safe("wrong", &hash).unwrap());
}

// ============================================================================
// PasswordHasher — auto-detect algorithm from hash prefix
// ============================================================================

#[test]
fn bcrypt_hash_can_be_verified_by_argon2_hasher_via_auto_detect() {
    let bcrypt_hasher = PasswordHasher::bcrypt(4).unwrap();
    let argon2_hasher = PasswordHasher::argon2().unwrap();
    let password = "shared-password";

    let bcrypt_hash = bcrypt_hasher.hash(password).unwrap();
    // Both hasher instances use auto-detection in verify(), so argon2_hasher can
    // also verify a bcrypt hash.
    assert!(
        argon2_hasher.verify(password, &bcrypt_hash).unwrap(),
        "argon2 hasher must accept a bcrypt hash via auto-detect"
    );
}

#[test]
fn argon2_hash_can_be_verified_by_bcrypt_hasher_via_auto_detect() {
    let bcrypt_hasher = PasswordHasher::bcrypt(4).unwrap();
    let argon2_hasher = PasswordHasher::argon2().unwrap();
    let password = "shared-password";

    let argon2_hash = argon2_hasher.hash(password).unwrap();
    assert!(
        bcrypt_hasher.verify(password, &argon2_hash).unwrap(),
        "bcrypt hasher must accept an argon2 hash via auto-detect"
    );
}

// ============================================================================
// PasswordHasher — empty password (edge case)
// ============================================================================

#[test]
fn empty_password_hashes_and_verifies() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("").unwrap();
    assert!(hasher.verify("", &hash).unwrap());
    assert!(!hasher.verify("x", &hash).unwrap());
}

// ============================================================================
// PasswordHasher — invalid hash format returns Err (not false)
// ============================================================================

#[test]
fn verify_with_unknown_hash_format_returns_err() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    // A hash that doesn't start with $2 (bcrypt) or $argon2 (argon2).
    let result = hasher.verify("password", "not-a-hash");
    assert!(
        result.is_err(),
        "unknown hash format must return Err, not false"
    );
}

// ============================================================================
// PasswordHasher — bcrypt cost boundaries
// ============================================================================

#[test]
fn bcrypt_cost_boundary_validation() {
    assert!(PasswordHasher::bcrypt(3).is_err(), "cost 3 is below minimum");
    assert!(PasswordHasher::bcrypt(4).is_ok(), "cost 4 is minimum valid");
    assert!(PasswordHasher::bcrypt(31).is_ok(), "cost 31 is maximum valid");
    assert!(PasswordHasher::bcrypt(32).is_err(), "cost 32 is above maximum");
}

// ============================================================================
// AuthManager::attempt — fail-closed cases
// ============================================================================

struct SimpleProvider {
    users: Vec<(String, String)>, // (email, password_hash)
}

impl UserProvider for SimpleProvider {
    fn retrieve_by_credentials(&self, creds: &Value) -> Option<Value> {
        let email = creds.get("email").and_then(Value::as_str)?;
        let (_, hash) = self.users.iter().find(|(e, _)| e == email)?;
        Some(json!({
            "id": 1,
            "email": email,
            "password": hash,
            "roles": ["user"]
        }))
    }
}

fn make_provider(email: &str, plain_password: &str) -> Arc<dyn UserProvider> {
    let hash = PasswordHasher::bcrypt(4).unwrap().hash(plain_password).unwrap();
    Arc::new(SimpleProvider {
        users: vec![(email.to_string(), hash)],
    })
}

#[test]
fn attempt_fails_closed_when_no_provider_registered() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        // No provider → must silently return false (never panic or propagate an error).
        let result = m.attempt(json!({"email": "a@b.com", "password": "secret"}));
        assert!(result.is_ok());
        assert!(!result.unwrap(), "no provider must deny");
        assert!(!m.check());
    });
}

#[test]
fn attempt_fails_closed_when_password_key_missing_from_credentials() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.set_provider(make_provider("a@b.com", "secret"));
        // Credentials with no "password" key.
        let result = m.attempt(json!({"email": "a@b.com"}));
        assert!(result.is_ok());
        assert!(!result.unwrap(), "missing password key must deny");
        assert!(!m.check());
    });
}

#[test]
fn attempt_fails_when_user_not_found() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.set_provider(make_provider("real@example.com", "secret"));
        let result = m.attempt(json!({"email": "ghost@example.com", "password": "secret"}));
        assert!(result.is_ok());
        assert!(!result.unwrap(), "unknown user must be denied");
        assert!(!m.check());
    });
}

#[test]
fn attempt_fails_when_password_is_wrong() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.set_provider(make_provider("u@example.com", "correct"));
        let result = m.attempt(json!({"email": "u@example.com", "password": "wrong"}));
        assert!(result.is_ok());
        assert!(!result.unwrap(), "wrong password must be denied");
        assert!(!m.check());
    });
}

#[test]
fn attempt_succeeds_and_strips_password_from_current_user() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.set_provider(make_provider("u@example.com", "correct"));
        let result = m.attempt(json!({"email": "u@example.com", "password": "correct"}));
        assert!(result.is_ok());
        assert!(result.unwrap(), "correct creds must succeed");
        assert!(m.check(), "user must be authenticated after success");

        // Password hash must NEVER appear in current_user.
        let user: Value = m.user().expect("user must be set");
        assert!(
            user.get("password").is_none(),
            "password hash must be stripped from session state"
        );
        assert_eq!(
            user.get("email").and_then(Value::as_str),
            Some("u@example.com")
        );
    });
}

// ============================================================================
// AuthManager — provider with no password field in returned record
// ============================================================================

struct PasswordlessProvider;

impl UserProvider for PasswordlessProvider {
    fn retrieve_by_credentials(&self, creds: &Value) -> Option<Value> {
        let email = creds.get("email").and_then(Value::as_str)?;
        if email == "u@example.com" {
            // Note: no "password" field — this simulates a misconfigured store.
            Some(json!({"id": 1, "email": email}))
        } else {
            None
        }
    }
}

#[test]
fn attempt_fails_closed_when_record_has_no_password_field() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.set_provider(Arc::new(PasswordlessProvider));
        let result = m.attempt(json!({"email": "u@example.com", "password": "anything"}));
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "record without password field must deny"
        );
        assert!(!m.check());
    });
}

// ============================================================================
// AuthManager — login → check → logout → check cycle
// ============================================================================

#[test]
fn login_check_logout_cycle() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();

        // Initially a guest.
        assert!(!m.check());
        assert!(m.guest());
        assert!(m.id().is_none());
        assert!(m.user::<Value>().is_none());

        // Login.
        m.login(json!({"id": 7, "email": "bob@example.com"}))
            .unwrap();
        assert!(m.check());
        assert!(!m.guest());
        assert_eq!(m.id(), Some(7));
        assert_eq!(
            m.user::<Value>()
                .unwrap()
                .get("email")
                .and_then(Value::as_str),
            Some("bob@example.com")
        );

        // Logout.
        m.logout();
        assert!(!m.check());
        assert!(m.guest());
        assert!(m.id().is_none());
    });
}

// ============================================================================
// AuthManager — has_role
// ============================================================================

#[test]
fn has_role_works_on_logged_in_user() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.login(json!({"id": 1, "roles": ["editor", "moderator"]}))
            .unwrap();

        assert!(m.has_role("editor"));
        assert!(m.has_role("moderator"));
        assert!(!m.has_role("admin"));

        assert!(m.has_any_role(&["editor", "admin"]));
        assert!(!m.has_any_role(&["admin", "superuser"]));

        assert!(m.has_all_roles(&["editor", "moderator"]));
        assert!(!m.has_all_roles(&["editor", "admin"]));
    });
}

#[test]
fn has_role_when_guest_returns_false() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        assert!(!m.has_role("admin"));
    });
}

// ============================================================================
// AuthManager — concurrent scope isolation (async)
// ============================================================================

#[tokio::test]
async fn concurrent_scopes_do_not_share_login_state() {
    let scope_a = with_auth_scope(async {
        let m = AuthManager::new();
        m.login(json!({"id": 10, "name": "Alice"})).unwrap();
        tokio::task::yield_now().await;
        (m.check(), m.id())
    });

    let scope_b = with_auth_scope(async {
        let m = AuthManager::new();
        // B never logs in; it must remain a guest despite A's concurrent login.
        tokio::task::yield_now().await;
        (m.check(), m.id())
    });

    let ((a_check, a_id), (b_check, b_id)) = tokio::join!(scope_a, scope_b);
    assert!(a_check && a_id == Some(10), "scope A must be authenticated");
    assert!(
        !b_check && b_id.is_none(),
        "scope B must not inherit A's authenticated state"
    );
}

// ============================================================================
// AuthManager — login_with_remember persists via_remember flag
// ============================================================================

#[test]
fn login_with_remember_sets_flag() {
    with_auth_scope_sync(|| {
        let m = AuthManager::new();
        m.login_with_remember(json!({"id": 3}), true).unwrap();
        assert!(m.check());
        assert!(m.via_remember(), "via_remember must be true after login_with_remember(_, true)");

        // Logout clears the flag.
        m.logout();
        assert!(!m.via_remember());
    });
}
