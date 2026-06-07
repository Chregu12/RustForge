//! Integration tests for rf-sanctum token module

use chrono::{Duration, Utc};
use rf_sanctum::{
    PersonalAccessToken, TransientTokenBuilder, TransientTokenStore,
};

// ── Token generation ──────────────────────────────────────────────────────────

#[test]
fn generate_token_produces_80_char_alphanum_string() {
    let token = PersonalAccessToken::generate_token();
    assert_eq!(token.len(), 80);
    assert!(token.chars().all(|c| c.is_alphanumeric()));
}

#[test]
fn two_generated_tokens_are_unique() {
    let t1 = PersonalAccessToken::generate_token();
    let t2 = PersonalAccessToken::generate_token();
    assert_ne!(t1, t2);
}

#[test]
fn hash_token_produces_64_char_hex_sha256() {
    let token = "my-plaintext-token";
    let hash = PersonalAccessToken::hash_token(token);
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn hash_token_is_deterministic() {
    let h1 = PersonalAccessToken::hash_token("same-token");
    let h2 = PersonalAccessToken::hash_token("same-token");
    assert_eq!(h1, h2);
}

#[test]
fn hash_token_differs_for_different_inputs() {
    let h1 = PersonalAccessToken::hash_token("token-a");
    let h2 = PersonalAccessToken::hash_token("token-b");
    assert_ne!(h1, h2);
}

#[test]
fn stored_token_field_is_hash_not_plaintext() {
    // Builder stores the SHA-256 hash, never the plain value
    let (plain, token) = TransientTokenBuilder::new("User", 1, "test").build();
    let expected_hash = PersonalAccessToken::hash_token(&plain);
    assert_eq!(token.token, expected_hash);
    assert_ne!(token.token, plain);
}

// ── Abilities / can() ────────────────────────────────────────────────────────

fn make_token(abilities: Vec<&str>) -> PersonalAccessToken {
    PersonalAccessToken {
        id: 1,
        tokenable_type: "User".into(),
        tokenable_id: 1,
        name: "test".into(),
        token: "hash".into(),
        abilities: abilities.into_iter().map(String::from).collect(),
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_agent: None,
        last_used_ip: None,
    }
}

#[test]
fn can_returns_true_for_present_ability() {
    let token = make_token(vec!["read:posts", "write:posts"]);
    assert!(token.can("read:posts"));
    assert!(token.can("write:posts"));
}

#[test]
fn can_returns_false_for_absent_ability() {
    let token = make_token(vec!["read:posts"]);
    assert!(!token.can("delete:posts"));
}

#[test]
fn wildcard_ability_grants_everything() {
    let token = make_token(vec!["*"]);
    assert!(token.can("read:posts"));
    assert!(token.can("delete:everything"));
    assert!(token.can("admin:superpower"));
}

#[test]
fn can_any_returns_true_when_one_ability_matches() {
    let token = make_token(vec!["read:posts"]);
    assert!(token.can_any(&["read:posts", "write:posts"]));
}

#[test]
fn can_any_returns_false_when_no_ability_matches() {
    let token = make_token(vec!["read:posts"]);
    assert!(!token.can_any(&["delete:posts", "admin:access"]));
}

#[test]
fn can_all_returns_true_when_all_abilities_present() {
    let token = make_token(vec!["read:posts", "write:posts", "delete:posts"]);
    assert!(token.can_all(&["read:posts", "write:posts"]));
}

#[test]
fn can_all_returns_false_when_any_ability_missing() {
    let token = make_token(vec!["read:posts"]);
    assert!(!token.can_all(&["read:posts", "write:posts"]));
}

#[test]
fn empty_abilities_list_denies_everything() {
    let token = make_token(vec![]);
    assert!(!token.can("read:posts"));
    assert!(!token.can_any(&["read:posts"]));
    assert!(!token.can_all(&["read:posts"]));
}

// ── Expiration ────────────────────────────────────────────────────────────────

#[test]
fn token_without_expiry_is_never_expired() {
    let token = make_token(vec![]);
    assert!(!token.is_expired());
}

#[test]
fn token_with_future_expiry_is_not_expired() {
    let mut token = make_token(vec![]);
    token.expires_at = Some(Utc::now() + Duration::hours(24));
    assert!(!token.is_expired());
}

#[test]
fn token_with_past_expiry_is_expired() {
    let mut token = make_token(vec![]);
    token.expires_at = Some(Utc::now() - Duration::seconds(1));
    assert!(token.is_expired());
}

// ── TransientTokenStore ───────────────────────────────────────────────────────

#[test]
fn store_and_find_token_by_hash() {
    let store = TransientTokenStore::new();
    let (plain, token) = TransientTokenBuilder::new("User", 1, "api-key")
        .with_abilities(vec!["read".into()])
        .build();
    let hash = token.token.clone();

    store.store(token).unwrap();
    let found = store.find(&hash).unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "api-key");

    // Verify plaintext is not recoverable from the store
    assert!(store.find(&plain).unwrap().is_none());
}

#[test]
fn remove_token_makes_it_unfindable() {
    let store = TransientTokenStore::new();
    let (_, token) = TransientTokenBuilder::new("User", 2, "to-remove").build();
    let hash = token.token.clone();

    store.store(token).unwrap();
    store.remove(&hash).unwrap();
    assert!(store.find(&hash).unwrap().is_none());
}

#[test]
fn cleanup_expired_removes_only_expired_tokens() {
    let store = TransientTokenStore::new();

    let (_, expired) = TransientTokenBuilder::new("User", 1, "expired")
        .with_expiration(Utc::now() - Duration::hours(1))
        .build();
    let (_, valid) = TransientTokenBuilder::new("User", 1, "valid")
        .with_expiration(Utc::now() + Duration::hours(1))
        .build();
    let valid_hash = valid.token.clone();

    store.store(expired).unwrap();
    store.store(valid).unwrap();

    let removed = store.cleanup_expired().unwrap();
    assert_eq!(removed, 1);
    assert!(store.find(&valid_hash).unwrap().is_some());
}

#[test]
fn remove_all_for_tokenable_removes_only_that_users_tokens() {
    let store = TransientTokenStore::new();
    let (_, t1) = TransientTokenBuilder::new("User", 10, "t1").build();
    let (_, t2) = TransientTokenBuilder::new("User", 10, "t2").build();
    let (_, t3) = TransientTokenBuilder::new("User", 99, "t3").build();
    let t3_hash = t3.token.clone();

    store.store(t1).unwrap();
    store.store(t2).unwrap();
    store.store(t3).unwrap();

    store.remove_all_for_tokenable("User", 10).unwrap();

    assert_eq!(store.count().unwrap(), 1);
    assert!(store.find(&t3_hash).unwrap().is_some());
}

#[test]
fn touch_sets_last_used_at() {
    let store = TransientTokenStore::new();
    let (_, token) = TransientTokenBuilder::new("User", 1, "touchable").build();
    let hash = token.token.clone();

    store.store(token).unwrap();
    assert!(store.find(&hash).unwrap().unwrap().last_used_at.is_none());

    store.touch(&hash).unwrap();
    assert!(store.find(&hash).unwrap().unwrap().last_used_at.is_some());
}

#[test]
fn clear_empties_the_store() {
    let store = TransientTokenStore::new();
    let (_, t) = TransientTokenBuilder::new("User", 1, "token").build();
    store.store(t).unwrap();
    store.clear().unwrap();
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn find_by_tokenable_returns_only_matching_tokens() {
    let store = TransientTokenStore::new();
    let (_, t1) = TransientTokenBuilder::new("User", 5, "t1").build();
    let (_, t2) = TransientTokenBuilder::new("User", 5, "t2").build();
    let (_, t3) = TransientTokenBuilder::new("App", 5, "t3").build(); // different type

    store.store(t1).unwrap();
    store.store(t2).unwrap();
    store.store(t3).unwrap();

    let user5 = store.find_by_tokenable("User", 5).unwrap();
    assert_eq!(user5.len(), 2);

    let app5 = store.find_by_tokenable("App", 5).unwrap();
    assert_eq!(app5.len(), 1);
}
