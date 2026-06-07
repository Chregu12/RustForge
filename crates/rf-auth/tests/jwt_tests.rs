//! Integration tests for rf-auth JWT module

use chrono::{Duration, Utc};
use rf_auth::{Claims, JwtManager};

const SECRET_32: &str = "super-secret-key-exactly-32chars";
const SECRET_LONG: &str = "another-super-secret-key-that-is-way-longer-than-32-characters";

// ── JwtManager construction ──────────────────────────────────────────────────

#[test]
fn jwt_manager_created_with_valid_secret() {
    assert!(JwtManager::new(SECRET_32).is_ok());
}

#[test]
fn jwt_manager_rejected_for_short_secret() {
    assert!(JwtManager::new("short").is_err());
}

#[test]
fn jwt_manager_rejected_for_31_char_secret() {
    // 31 characters — one too short
    assert!(JwtManager::new("exactly31characterslongkeyXXXXX").is_err());
}

#[test]
fn jwt_manager_accepted_for_exactly_32_char_secret() {
    assert!(JwtManager::new(SECRET_32).is_ok());
}

// ── Token generation ─────────────────────────────────────────────────────────

#[test]
fn token_is_non_empty_string_with_two_dots() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let claims = Claims::new(1, "alice@example.com".into(), vec![], 1);
    let token = jwt.generate_token(&claims).unwrap();
    assert!(!token.is_empty());
    assert_eq!(token.matches('.').count(), 2, "JWT must have three parts");
}

#[test]
fn two_tokens_for_same_claims_are_decodable() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let claims = Claims::new(1, "bob@example.com".into(), vec![], 1);
    let t1 = jwt.generate_token(&claims).unwrap();
    let t2 = jwt.generate_token(&claims).unwrap();
    let d1 = jwt.validate_token(&t1).unwrap();
    let d2 = jwt.validate_token(&t2).unwrap();
    assert_eq!(d1.user_id, d2.user_id);
    assert_eq!(d1.sub, d2.sub);
}

// ── Token validation ─────────────────────────────────────────────────────────

#[test]
fn valid_token_round_trips_all_claims() {
    let jwt = JwtManager::new(SECRET_LONG).unwrap();
    let claims = Claims::new(
        42,
        "carol@example.com".into(),
        vec!["admin".into(), "editor".into()],
        24,
    );
    let token = jwt.generate_token(&claims).unwrap();
    let decoded = jwt.validate_token(&token).unwrap();

    assert_eq!(decoded.user_id, 42);
    assert_eq!(decoded.sub, "carol@example.com");
    assert_eq!(decoded.roles, vec!["admin", "editor"]);
    assert!(decoded.jti.is_some());
}

#[test]
fn validate_rejects_completely_invalid_token() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    assert!(jwt.validate_token("this.is.garbage").is_err());
}

#[test]
fn validate_rejects_empty_string() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    assert!(jwt.validate_token("").is_err());
}

#[test]
fn validate_rejects_token_signed_with_different_secret() {
    let jwt_a = JwtManager::new(SECRET_32).unwrap();
    let jwt_b = JwtManager::new("completely-different-32char-secret").unwrap();
    let claims = Claims::new(1, "dave@example.com".into(), vec![], 1);
    let token = jwt_a.generate_token(&claims).unwrap();
    assert!(jwt_b.validate_token(&token).is_err());
}

#[test]
fn validate_rejects_expired_token() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let mut claims = Claims::new(1, "eve@example.com".into(), vec![], 1);
    // Force expiry well into the past (2 hours ago) to exceed any leeway
    claims.exp = (Utc::now() - Duration::hours(2)).timestamp();
    let token = jwt.generate_token(&claims).unwrap();
    assert!(jwt.validate_token(&token).is_err());
}

#[test]
fn validate_rejects_tampered_signature() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let claims = Claims::new(1, "frank@example.com".into(), vec![], 1);
    let token = jwt.generate_token(&claims).unwrap();

    // Replace the signature (last segment) with garbage
    let mut parts: Vec<&str> = token.split('.').collect();
    parts[2] = "invalidsignatureXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let tampered = parts.join(".");
    assert!(jwt.validate_token(&tampered).is_err());
}

// ── Refresh token ────────────────────────────────────────────────────────────

#[test]
fn refresh_token_belongs_to_same_user() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let claims = Claims::new(7, "grace@example.com".into(), vec!["user".into()], 1);
    let refresh = jwt.generate_refresh_token(&claims).unwrap();
    let decoded = jwt.validate_refresh_token(&refresh).unwrap();
    assert_eq!(decoded.user_id, 7);
    assert_eq!(decoded.sub, "grace@example.com");
}

#[test]
fn refresh_token_has_different_jti_than_access_token() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let claims = Claims::new(8, "henry@example.com".into(), vec![], 1);
    let access = jwt.generate_token(&claims).unwrap();
    let refresh = jwt.generate_refresh_token(&claims).unwrap();

    let access_claims = jwt.validate_token(&access).unwrap();
    let refresh_claims = jwt.validate_refresh_token(&refresh).unwrap();
    assert_ne!(access_claims.jti, refresh_claims.jti);
}

#[test]
fn refresh_token_has_longer_expiry_than_access_token() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    // Access token valid for 1 hour
    let claims = Claims::new(9, "iris@example.com".into(), vec![], 1);
    let access = jwt.generate_token(&claims).unwrap();
    let refresh = jwt.generate_refresh_token(&claims).unwrap();

    let access_decoded = jwt.validate_token(&access).unwrap();
    let refresh_decoded = jwt.validate_refresh_token(&refresh).unwrap();
    assert!(refresh_decoded.exp > access_decoded.exp);
}

// ── Claims helpers ─────────────────────────────────────────────────────────────

#[test]
fn claims_not_expired_when_freshly_created() {
    let claims = Claims::new(1, "jack@example.com".into(), vec![], 24);
    assert!(!claims.is_expired());
}

#[test]
fn claims_is_expired_when_exp_in_past() {
    let mut claims = Claims::new(1, "kate@example.com".into(), vec![], 1);
    claims.exp = (Utc::now() - Duration::seconds(1)).timestamp();
    assert!(claims.is_expired());
}

#[test]
fn has_role_returns_true_for_matching_role() {
    let claims = Claims::new(
        1,
        "leo@example.com".into(),
        vec!["admin".into(), "moderator".into()],
        1,
    );
    assert!(claims.has_role("admin"));
    assert!(claims.has_role("moderator"));
    assert!(!claims.has_role("user"));
}

#[test]
fn has_any_role_true_when_at_least_one_matches() {
    let claims = Claims::new(1, "mia@example.com".into(), vec!["editor".into()], 1);
    assert!(claims.has_any_role(&["admin", "editor"]));
    assert!(!claims.has_any_role(&["admin", "superuser"]));
}

#[test]
fn has_all_roles_true_only_when_every_role_present() {
    let claims = Claims::new(
        1,
        "nina@example.com".into(),
        vec!["editor".into(), "reviewer".into()],
        1,
    );
    assert!(claims.has_all_roles(&["editor", "reviewer"]));
    assert!(!claims.has_all_roles(&["editor", "admin"]));
}

#[test]
fn is_expired_helper_on_manager_delegates_to_claims() {
    let jwt = JwtManager::new(SECRET_32).unwrap();
    let claims = Claims::new(1, "oscar@example.com".into(), vec![], 1);
    assert!(!jwt.is_expired(&claims));
}

#[test]
fn claims_jti_is_unique_uuid_format() {
    let c1 = Claims::new(1, "a@example.com".into(), vec![], 1);
    let c2 = Claims::new(1, "a@example.com".into(), vec![], 1);
    // Each fresh Claims gets its own JTI
    assert!(c1.jti.is_some());
    assert!(c2.jti.is_some());
    assert_ne!(c1.jti, c2.jti);
}
