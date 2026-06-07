//! Integration tests for rf-auth password hashing module

use rf_auth::PasswordHasher;

// ── Bcrypt ───────────────────────────────────────────────────────────────────

#[test]
fn bcrypt_hash_starts_with_bcrypt_prefix() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("password123").unwrap();
    assert!(hash.starts_with("$2"), "bcrypt hash should start with $2");
}

#[test]
fn bcrypt_correct_password_verifies() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("correct-horse").unwrap();
    assert!(hasher.verify("correct-horse", &hash).unwrap());
}

#[test]
fn bcrypt_wrong_password_rejected() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("correct-horse").unwrap();
    assert!(!hasher.verify("wrong-password", &hash).unwrap());
}

#[test]
fn bcrypt_empty_password_rejected_by_wrong_guess() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("not-empty").unwrap();
    assert!(!hasher.verify("", &hash).unwrap());
}

#[test]
fn bcrypt_cost_4_is_valid() {
    assert!(PasswordHasher::bcrypt(4).is_ok());
}

#[test]
fn bcrypt_cost_31_is_valid() {
    assert!(PasswordHasher::bcrypt(31).is_ok());
}

#[test]
fn bcrypt_cost_3_is_invalid() {
    let err = PasswordHasher::bcrypt(3);
    assert!(err.is_err());
}

#[test]
fn bcrypt_cost_32_is_invalid() {
    let err = PasswordHasher::bcrypt(32);
    assert!(err.is_err());
}

#[test]
fn bcrypt_cost_0_is_invalid() {
    assert!(PasswordHasher::bcrypt(0).is_err());
}

#[test]
fn bcrypt_produces_different_hashes_for_same_password() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let h1 = hasher.hash("same-password").unwrap();
    let h2 = hasher.hash("same-password").unwrap();
    // bcrypt embeds a random salt, so hashes must differ
    assert_ne!(h1, h2);
}

#[test]
fn bcrypt_hash_with_different_cost_produces_different_hash() {
    let hasher4 = PasswordHasher::bcrypt(4).unwrap();
    let hasher6 = PasswordHasher::bcrypt(6).unwrap();
    let h4 = hasher4.hash("pw").unwrap();
    let h6 = hasher6.hash("pw").unwrap();
    // Both still verify
    assert!(hasher4.verify("pw", &h4).unwrap());
    assert!(hasher6.verify("pw", &h6).unwrap());
    // But the stored hashes contain the cost factor and differ
    assert_ne!(h4, h6);
}

// ── Argon2 ───────────────────────────────────────────────────────────────────

#[test]
fn argon2_hash_starts_with_argon2_prefix() {
    let hasher = PasswordHasher::argon2().unwrap();
    let hash = hasher.hash("secure-pass").unwrap();
    assert!(
        hash.starts_with("$argon2"),
        "argon2 hash should start with $argon2"
    );
}

#[test]
fn argon2_correct_password_verifies() {
    let hasher = PasswordHasher::argon2().unwrap();
    let hash = hasher.hash("my-password").unwrap();
    assert!(hasher.verify("my-password", &hash).unwrap());
}

#[test]
fn argon2_wrong_password_rejected() {
    let hasher = PasswordHasher::argon2().unwrap();
    let hash = hasher.hash("my-password").unwrap();
    assert!(!hasher.verify("wrong-password", &hash).unwrap());
}

#[test]
fn argon2_produces_different_hashes_due_to_random_salt() {
    let hasher = PasswordHasher::argon2().unwrap();
    let h1 = hasher.hash("same").unwrap();
    let h2 = hasher.hash("same").unwrap();
    assert_ne!(h1, h2);
}

// ── Cross-algorithm verification ─────────────────────────────────────────────

#[test]
fn argon2_hasher_can_verify_bcrypt_hash_via_auto_detection() {
    let bcrypt_hasher = PasswordHasher::bcrypt(4).unwrap();
    let argon2_hasher = PasswordHasher::argon2().unwrap();
    let bcrypt_hash = bcrypt_hasher.hash("cross-verify").unwrap();
    // auto-detection should pick bcrypt path
    assert!(argon2_hasher.verify("cross-verify", &bcrypt_hash).unwrap());
}

#[test]
fn bcrypt_hasher_can_verify_argon2_hash_via_auto_detection() {
    let bcrypt_hasher = PasswordHasher::bcrypt(4).unwrap();
    let argon2_hasher = PasswordHasher::argon2().unwrap();
    let argon2_hash = argon2_hasher.hash("cross-verify").unwrap();
    assert!(bcrypt_hasher.verify("cross-verify", &argon2_hash).unwrap());
}

#[test]
fn unknown_hash_format_returns_error() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let result = hasher.verify("password", "not-a-known-hash-format");
    assert!(result.is_err());
}

// ── Timing-safe verify ────────────────────────────────────────────────────────

#[test]
fn timing_safe_verify_returns_true_for_correct_password() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("timing-safe-test").unwrap();
    assert!(hasher.verify_timing_safe("timing-safe-test", &hash).unwrap());
}

#[test]
fn timing_safe_verify_returns_false_for_wrong_password() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let hash = hasher.hash("correct").unwrap();
    assert!(!hasher.verify_timing_safe("wrong", &hash).unwrap());
}

// ── Default hasher ────────────────────────────────────────────────────────────

#[test]
fn default_hasher_uses_bcrypt() {
    let hasher = PasswordHasher::default();
    let hash = hasher.hash("default-test").unwrap();
    assert!(hash.starts_with("$2"));
    assert!(hasher.verify("default-test", &hash).unwrap());
}

// ── Unicode / special chars ───────────────────────────────────────────────────

#[test]
fn bcrypt_handles_unicode_password() {
    let hasher = PasswordHasher::bcrypt(4).unwrap();
    let pw = "pässwörd-日本語-🔐";
    let hash = hasher.hash(pw).unwrap();
    assert!(hasher.verify(pw, &hash).unwrap());
    assert!(!hasher.verify("wrong", &hash).unwrap());
}
