//! Integration tests for auth features
//!
//! Tests email verification, password reset, and remember me functionality.

use chrono::{DateTime, Utc};
use rf_auth::{
    password_reset::{PasswordReset, Resettable},
    remember_me::RememberMe,
    verification::{EmailVerification, Verifiable},
    AuthResult, PasswordHasher,
};
use std::time::Duration;

const TEST_SECRET: &str = "test-secret-key-must-be-32-chars-long";

// Mock User for testing
#[derive(Clone, Debug)]
struct TestUser {
    id: i64,
    email: String,
    password: String,
    email_verified_at: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
impl Verifiable for TestUser {
    fn verification_email(&self) -> &str {
        &self.email
    }

    fn verification_user_id(&self) -> i64 {
        self.id
    }

    fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    async fn mark_verified(&mut self) -> AuthResult<()> {
        self.email_verified_at = Some(Utc::now());
        Ok(())
    }
}

#[async_trait::async_trait]
impl Resettable for TestUser {
    fn reset_email(&self) -> &str {
        &self.email
    }

    fn reset_user_id(&self) -> i64 {
        self.id
    }

    async fn update_password(&mut self, new_password_hash: String) -> AuthResult<()> {
        self.password = new_password_hash;
        Ok(())
    }
}

// EMAIL VERIFICATION TESTS

#[tokio::test]
async fn test_email_verification_full_flow() {
    let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());

    let mut user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: "hashed_password".to_string(),
        email_verified_at: None,
    };

    // User starts unverified
    assert!(!user.is_verified());

    // Generate verification token
    let token = verification
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // Verify email with token
    user.verify_email(&token, &verification)
        .await
        .expect("Email verification failed");

    // User should now be verified
    assert!(user.is_verified());
    assert!(user.email_verified_at.is_some());
}

#[tokio::test]
async fn test_email_verification_url_generation() {
    let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());

    let url = verification
        .generate_url("https://example.com", 123, "test@example.com")
        .expect("URL generation failed");

    assert!(url.starts_with("https://example.com/verify-email?token="));
    assert!(url.len() > 50);

    // Extract token from URL
    let token = url.split("token=").nth(1).expect("Token not found in URL");

    // Verify the token works
    let claims = verification
        .verify_token(token)
        .expect("Token verification failed");
    assert_eq!(claims.sub, 123);
    assert_eq!(claims.email, "test@example.com");
}

#[tokio::test]
async fn test_email_verification_expired_token() {
    let verification = EmailVerification::new(TEST_SECRET.to_string(), Duration::from_secs(1));

    let mut user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: "hashed_password".to_string(),
        email_verified_at: None,
    };

    let token = verification
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // Wait for token to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verification should fail
    let result = user.verify_email(&token, &verification).await;
    assert!(result.is_err());
    assert!(!user.is_verified());
}

#[tokio::test]
async fn test_email_verification_wrong_user() {
    let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());

    let mut user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: "hashed_password".to_string(),
        email_verified_at: None,
    };

    // Generate token for different user
    let token = verification
        .generate_token(999, "other@example.com")
        .expect("Token generation failed");

    // Verification should fail
    let result = user.verify_email(&token, &verification).await;
    assert!(result.is_err());
    assert!(!user.is_verified());
}

// PASSWORD RESET TESTS

#[tokio::test]
async fn test_password_reset_full_flow() {
    let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
    let hasher = PasswordHasher::bcrypt(4).expect("Hasher creation failed");

    let mut user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: hasher.hash("old_password").expect("Hash failed"),
        email_verified_at: Some(Utc::now()),
    };

    let old_password_hash = user.password.clone();

    // Generate reset token
    let token = reset
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // Reset password
    user.reset_password(&token, "new_password", &hasher, &reset)
        .await
        .expect("Password reset failed");

    // Password should be changed
    assert_ne!(user.password, old_password_hash);
    assert!(hasher.verify("new_password", &user.password).unwrap());
    assert!(!hasher.verify("old_password", &user.password).unwrap());
}

#[tokio::test]
async fn test_password_reset_url_generation() {
    let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());

    let url = reset
        .generate_url("https://example.com", 123, "test@example.com")
        .expect("URL generation failed");

    assert!(url.starts_with("https://example.com/reset-password?token="));
    assert!(url.len() > 50);

    // Extract token from URL
    let token = url.split("token=").nth(1).expect("Token not found in URL");

    // Verify the token works
    let claims = reset
        .verify_token(token)
        .expect("Token verification failed");
    assert_eq!(claims.sub, 123);
    assert_eq!(claims.email, "test@example.com");
}

#[tokio::test]
async fn test_password_reset_expired_token() {
    let reset = PasswordReset::new(TEST_SECRET.to_string(), Duration::from_secs(1));
    let hasher = PasswordHasher::bcrypt(4).expect("Hasher creation failed");

    let mut user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: hasher.hash("old_password").expect("Hash failed"),
        email_verified_at: Some(Utc::now()),
    };

    let token = reset
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // Wait for token to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Reset should fail
    let result = user
        .reset_password(&token, "new_password", &hasher, &reset)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_password_reset_wrong_user() {
    let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
    let hasher = PasswordHasher::bcrypt(4).expect("Hasher creation failed");

    let mut user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: hasher.hash("old_password").expect("Hash failed"),
        email_verified_at: Some(Utc::now()),
    };

    // Generate token for different user
    let token = reset
        .generate_token(999, "other@example.com")
        .expect("Token generation failed");

    // Reset should fail
    let result = user
        .reset_password(&token, "new_password", &hasher, &reset)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_password_reset_verify_token() {
    let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());

    let user = TestUser {
        id: 1,
        email: "test@example.com".to_string(),
        password: "hash".to_string(),
        email_verified_at: Some(Utc::now()),
    };

    let token = reset
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // Verify token without resetting
    let claims = user
        .verify_reset_token(&token, &reset)
        .expect("Token verification failed");

    assert_eq!(claims.sub, user.id);
    assert_eq!(claims.email, user.email);
}

// REMEMBER ME TESTS

#[test]
fn test_remember_me_token_generation() {
    let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

    let token = remember
        .generate_token(123)
        .expect("Token generation failed");

    assert!(!token.is_empty());
    assert!(token.contains('.'));

    // Verify token
    let user_id = remember
        .verify_token(&token)
        .expect("Token verification failed");
    assert_eq!(user_id, 123);
}

#[test]
fn test_remember_me_cookie_creation() {
    let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

    let cookie = remember.create_cookie(123).expect("Cookie creation failed");

    let cookie_str = cookie.to_str().expect("Invalid cookie string");
    assert!(cookie_str.contains("remember_token="));
    assert!(cookie_str.contains("HttpOnly"));
    assert!(cookie_str.contains("Secure"));
    assert!(cookie_str.contains("SameSite=Strict"));
    assert!(cookie_str.contains("Max-Age="));
}

#[test]
fn test_remember_me_delete_cookie() {
    let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

    let cookie = remember.delete_cookie();

    let cookie_str = cookie.to_str().expect("Invalid cookie string");
    assert!(cookie_str.contains("remember_token="));
    assert!(cookie_str.contains("Max-Age=0"));
}

#[test]
fn test_remember_me_token_rotation() {
    let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

    let old_token = remember
        .generate_token(123)
        .expect("Token generation failed");

    // Rotate token
    let new_token = remember
        .rotate_token(&old_token)
        .expect("Token rotation failed");

    // Both tokens should be for same user
    assert_eq!(remember.verify_token(&old_token).unwrap(), 123);
    assert_eq!(remember.verify_token(&new_token).unwrap(), 123);

    // But tokens should be different
    assert_ne!(old_token, new_token);

    // And have different JTIs
    let old_claims = remember.verify_token_full(&old_token).unwrap();
    let new_claims = remember.verify_token_full(&new_token).unwrap();
    assert_ne!(old_claims.jti, new_claims.jti);
}

#[test]
fn test_remember_me_expired_token() {
    let remember = RememberMe::new(TEST_SECRET.to_string(), Duration::from_secs(1));

    let token = remember
        .generate_token(123)
        .expect("Token generation failed");

    // Wait for token to expire
    std::thread::sleep(Duration::from_secs(2));

    // Verification should fail
    let result = remember.verify_token(&token);
    assert!(result.is_err());
}

// SECURITY TESTS

#[test]
fn test_tokens_with_different_secrets() {
    let secret1 = TEST_SECRET.to_string();
    let secret2 = "different-secret-key-32-chars!!".to_string();

    // Email Verification
    let verification1 = EmailVerification::with_default_ttl(secret1.clone());
    let verification2 = EmailVerification::with_default_ttl(secret2.clone());

    let token1 = verification1
        .generate_token(123, "test@example.com")
        .unwrap();
    assert!(verification2.verify_token(&token1).is_err());

    // Password Reset
    let reset1 = PasswordReset::with_default_ttl(secret1.clone());
    let reset2 = PasswordReset::with_default_ttl(secret2.clone());

    let token2 = reset1.generate_token(123, "test@example.com").unwrap();
    assert!(reset2.verify_token(&token2).is_err());

    // Remember Me
    let remember1 = RememberMe::with_default_ttl(secret1);
    let remember2 = RememberMe::with_default_ttl(secret2);

    let token3 = remember1.generate_token(123).unwrap();
    assert!(remember2.verify_token(&token3).is_err());
}

#[test]
fn test_invalid_tokens_rejected() {
    let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
    let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
    let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

    // Invalid token formats
    assert!(verification.verify_token("invalid").is_err());
    assert!(verification.verify_token("invalid.token.here").is_err());

    assert!(reset.verify_token("invalid").is_err());
    assert!(reset.verify_token("invalid.token.here").is_err());

    assert!(remember.verify_token("invalid").is_err());
    assert!(remember.verify_token("invalid.token.here").is_err());
}

// INTEGRATION TESTS

#[tokio::test]
async fn test_complete_user_registration_flow() {
    let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
    let hasher = PasswordHasher::bcrypt(4).expect("Hasher creation failed");

    // 1. User registers
    let mut user = TestUser {
        id: 1,
        email: "newuser@example.com".to_string(),
        password: hasher.hash("initial_password").expect("Hash failed"),
        email_verified_at: None,
    };

    assert!(!user.is_verified());

    // 2. Send verification email (we just generate token here)
    let verify_token = verification
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // 3. User clicks verification link
    user.verify_email(&verify_token, &verification)
        .await
        .expect("Email verification failed");

    assert!(user.is_verified());
}

#[tokio::test]
async fn test_complete_password_reset_flow() {
    let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
    let hasher = PasswordHasher::bcrypt(4).expect("Hasher creation failed");

    // 1. User with existing account
    let mut user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        password: hasher.hash("forgotten_password").expect("Hash failed"),
        email_verified_at: Some(Utc::now()),
    };

    // 2. User requests password reset
    let reset_token = reset
        .generate_token(user.id, &user.email)
        .expect("Token generation failed");

    // 3. User clicks reset link and enters new password
    user.reset_password(&reset_token, "new_secure_password", &hasher, &reset)
        .await
        .expect("Password reset failed");

    // 4. Verify new password works
    assert!(hasher
        .verify("new_secure_password", &user.password)
        .unwrap());
    assert!(!hasher.verify("forgotten_password", &user.password).unwrap());
}

#[test]
fn test_complete_remember_me_flow() {
    let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

    // 1. User logs in with "remember me" checked
    let user_id = 123;
    let cookie = remember
        .create_cookie(user_id)
        .expect("Cookie creation failed");

    // 2. Cookie is set in response
    let cookie_str = cookie.to_str().expect("Invalid cookie string");
    assert!(cookie_str.contains("remember_token="));

    // 3. User returns later, cookie is sent
    // Extract token from cookie
    let token = cookie_str
        .split("remember_token=")
        .nth(1)
        .and_then(|s| s.split(';').next())
        .expect("Token extraction failed");

    // 4. Verify token and authenticate user
    let authenticated_user_id = remember
        .verify_token(token)
        .expect("Token verification failed");
    assert_eq!(authenticated_user_id, user_id);

    // 5. Rotate token for next visit
    let new_token = remember.rotate_token(token).expect("Token rotation failed");
    assert_ne!(token, new_token);
}
