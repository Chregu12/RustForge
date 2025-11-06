//! Integration Tests for Auth Scaffolding
//!
//! Comprehensive end-to-end tests for authentication flows

use foundry_auth_scaffolding::*;
use uuid::Uuid;

/// Helper function to create test auth service
fn create_test_auth_service() -> AuthService {
    let config = AuthConfig::default();
    AuthService::new(config)
}

/// Helper function to create test auth service with email verification
fn create_test_auth_service_with_email_verification() -> AuthService {
    let mut config = AuthConfig::default();
    config.require_email_verification = true;
    AuthService::new(config)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_registration_flow() {
        // Setup
        let service = create_test_auth_service();

        // Register new user
        let register_data = RegisterData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "secure_password_123".to_string(),
            password_confirmation: "secure_password_123".to_string(),
        };

        let user = service.register(register_data.clone()).unwrap();

        // Verify user properties
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        assert!(!user.password_hash.is_empty());
        assert_ne!(user.password_hash, "secure_password_123"); // Password should be hashed

        // Verify password is hashed
        let is_valid = service.verify_password("secure_password_123", &user.password_hash).unwrap();
        assert!(is_valid);

        let is_invalid = service.verify_password("wrong_password", &user.password_hash).unwrap();
        assert!(!is_invalid);

        // Verify user can login
        let credentials = Credentials {
            email: "john@example.com".to_string(),
            password: "secure_password_123".to_string(),
            remember: false,
        };

        // Mark email as verified for login test
        let mut verified_user = user.clone();
        verified_user.mark_email_as_verified();

        let result = service.attempt(&verified_user, &credentials);
        assert!(result.is_ok());

        // Verify session is created
        let session = service.create_session(&verified_user, false);
        assert_eq!(session.user_id, verified_user.id);
        assert!(!session.token.is_empty());
        assert!(!session.is_expired());
    }

    #[test]
    fn test_registration_validation() {
        let service = create_test_auth_service();

        // Test password mismatch
        let mismatched_data = RegisterData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "password123".to_string(),
            password_confirmation: "different_password".to_string(),
        };

        let result = service.register(mismatched_data);
        assert!(result.is_err());

        // Test short password
        let short_password_data = RegisterData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "short".to_string(),
            password_confirmation: "short".to_string(),
        };

        let result = service.register(short_password_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_login_flow() {
        // Setup
        let service = create_test_auth_service();

        // Create user
        let password_hash = service.hash_password("correct_password").unwrap();
        let mut user = User::new(
            "Jane Smith".to_string(),
            "jane@example.com".to_string(),
            password_hash,
        );
        user.mark_email_as_verified();

        // Login with correct credentials
        let correct_credentials = Credentials {
            email: "jane@example.com".to_string(),
            password: "correct_password".to_string(),
            remember: false,
        };

        let result = service.attempt(&user, &correct_credentials);
        assert!(result.is_ok());

        // Verify session created
        let session = service.create_session(&user, false);
        assert_eq!(session.user_id, user.id);
        assert!(!session.is_expired());

        // Login with wrong password fails
        let wrong_credentials = Credentials {
            email: "jane@example.com".to_string(),
            password: "wrong_password".to_string(),
            remember: false,
        };

        let result = service.attempt(&user, &wrong_credentials);
        assert!(result.is_err());

        // Verify no session created on failure (handled by application logic)
    }

    #[test]
    fn test_remember_me_functionality() {
        let service = create_test_auth_service();

        let user = User::new(
            "John".to_string(),
            "john@example.com".to_string(),
            "hash".to_string(),
        );

        // Session without remember me
        let short_session = service.create_session(&user, false);
        let short_lifetime = (short_session.expires_at - short_session.created_at).num_seconds();

        // Session with remember me
        let long_session = service.create_session(&user, true);
        let long_lifetime = (long_session.expires_at - long_session.created_at).num_seconds();

        // Remember me session should last longer
        assert!(long_lifetime > short_lifetime);
        assert_eq!(short_lifetime, 7200); // 2 hours
        assert_eq!(long_lifetime, 2592000); // 30 days
    }

    #[test]
    fn test_password_reset_flow() {
        use foundry_auth_scaffolding::password::PasswordResetManager;
        use foundry_auth_scaffolding::models::PasswordReset;

        // Setup
        let service = create_test_auth_service();
        let reset_manager = PasswordResetManager::new();

        // User forgets password
        let user_email = "user@example.com".to_string();

        // Request password reset
        let reset_token = service.generate_token();
        let password_reset = PasswordReset::new(
            user_email.clone(),
            reset_token.clone(),
            3600, // 1 hour
        );

        reset_manager.store(password_reset.clone());

        // Verify token created
        let found_reset = reset_manager.find(&reset_token);
        assert!(found_reset.is_some());
        assert_eq!(found_reset.as_ref().unwrap().email, user_email);
        assert!(!found_reset.unwrap().is_expired());

        // Reset password with token
        let new_password = "new_secure_password_456";
        let new_password_hash = service.hash_password(new_password).unwrap();

        // Update user with new password
        let mut user = User::new(
            "User".to_string(),
            user_email.clone(),
            "old_hash".to_string(),
        );
        user.password_hash = new_password_hash.clone();
        user.mark_email_as_verified();

        // Verify old password doesn't work
        let old_credentials = Credentials {
            email: user_email.clone(),
            password: "old_password".to_string(),
            remember: false,
        };
        let result = service.attempt(&user, &old_credentials);
        assert!(result.is_err());

        // Verify new password works
        let new_credentials = Credentials {
            email: user_email.clone(),
            password: new_password.to_string(),
            remember: false,
        };
        let result = service.attempt(&user, &new_credentials);
        assert!(result.is_ok());

        // Clean up - delete used token
        reset_manager.delete(&reset_token);
        assert!(reset_manager.find(&reset_token).is_none());
    }

    #[test]
    fn test_expired_password_reset_token() {
        use foundry_auth_scaffolding::password::PasswordResetManager;
        use foundry_auth_scaffolding::models::PasswordReset;

        let reset_manager = PasswordResetManager::new();

        // Create expired token
        let expired_reset = PasswordReset::new(
            "user@example.com".to_string(),
            "expired_token".to_string(),
            -3600, // Expired 1 hour ago
        );

        reset_manager.store(expired_reset);

        let found = reset_manager.find("expired_token").unwrap();
        assert!(found.is_expired());

        // Cleanup expired tokens
        reset_manager.cleanup_expired();
        assert!(reset_manager.find("expired_token").is_none());
    }

    #[test]
    fn test_email_verification_flow() {
        use foundry_auth_scaffolding::email_verification::EmailVerificationManager;
        use foundry_auth_scaffolding::models::EmailVerification;

        // Setup
        let service = create_test_auth_service_with_email_verification();
        let verification_manager = EmailVerificationManager::new();

        // Register user (unverified)
        let register_data = RegisterData {
            name: "New User".to_string(),
            email: "newuser@example.com".to_string(),
            password: "password123".to_string(),
            password_confirmation: "password123".to_string(),
        };

        let user = service.register(register_data).unwrap();
        assert!(!user.has_verified_email());

        // Generate verification token
        let verification_token = service.generate_token();
        let email_verification = EmailVerification::new(
            user.id,
            verification_token.clone(),
            86400, // 24 hours
        );

        verification_manager.store(email_verification);

        // Verify token exists
        let found = verification_manager.find(&verification_token);
        assert!(found.is_some());
        assert_eq!(found.as_ref().unwrap().user_id, user.id);

        // Verify email
        let mut verified_user = user.clone();
        verified_user.mark_email_as_verified();

        // Check user is verified
        assert!(verified_user.has_verified_email());

        // User can now login
        let credentials = Credentials {
            email: "newuser@example.com".to_string(),
            password: "password123".to_string(),
            remember: false,
        };

        let result = service.attempt(&verified_user, &credentials);
        assert!(result.is_ok());

        // Clean up verification token
        verification_manager.delete(&verification_token);
    }

    #[test]
    fn test_unverified_email_blocks_login() {
        let service = create_test_auth_service_with_email_verification();

        // Create user without verified email
        let password_hash = service.hash_password("password123").unwrap();
        let user = User::new(
            "Unverified User".to_string(),
            "unverified@example.com".to_string(),
            password_hash,
        );

        assert!(!user.has_verified_email());

        // Attempt login
        let credentials = Credentials {
            email: "unverified@example.com".to_string(),
            password: "password123".to_string(),
            remember: false,
        };

        let result = service.attempt(&user, &credentials);
        assert!(result.is_err());

        if let Err(auth::AuthError::EmailNotVerified) = result {
            // Expected error
        } else {
            panic!("Expected EmailNotVerified error");
        }
    }

    #[cfg(feature = "two-factor")]
    #[test]
    fn test_two_factor_authentication_flow() {
        use foundry_auth_scaffolding::two_factor::TwoFactorService;

        // Setup
        let service = create_test_auth_service();
        let two_factor = TwoFactorService::new("Test App".to_string());

        // Enable 2FA for user
        let password_hash = service.hash_password("password123").unwrap();
        let mut user = User::new(
            "Secure User".to_string(),
            "secure@example.com".to_string(),
            password_hash,
        );
        user.mark_email_as_verified();

        // Generate TOTP secret
        let secret = two_factor.generate_secret();
        let recovery_codes = two_factor.generate_recovery_codes(8);

        user.enable_two_factor(secret.clone(), recovery_codes.clone());

        assert!(user.has_two_factor());
        assert_eq!(user.two_factor_recovery_codes.as_ref().unwrap().len(), 8);

        // Attempt login (should require 2FA)
        let credentials = Credentials {
            email: "secure@example.com".to_string(),
            password: "password123".to_string(),
            remember: false,
        };

        let result = service.attempt(&user, &credentials);
        assert!(result.is_err());

        if let Err(auth::AuthError::TwoFactorRequired) = result {
            // Expected - 2FA required
        } else {
            panic!("Expected TwoFactorRequired error");
        }

        // Generate QR code for setup
        let qr_code = two_factor.generate_qr_code("secure@example.com", &secret);
        assert!(qr_code.is_ok());
        assert!(!qr_code.unwrap().is_empty());
    }

    #[cfg(feature = "two-factor")]
    #[test]
    fn test_recovery_codes_work() {
        use foundry_auth_scaffolding::two_factor::TwoFactorService;

        let two_factor = TwoFactorService::new("Test App".to_string());

        // Generate recovery codes
        let mut recovery_codes = two_factor.generate_recovery_codes(10);
        assert_eq!(recovery_codes.len(), 10);

        // Verify recovery code format
        for code in &recovery_codes {
            assert_eq!(code.len(), 9); // XXXX-XXXX format
            assert!(code.contains('-'));
        }

        // Use a recovery code
        let test_code = recovery_codes[0].clone();
        assert!(two_factor.verify_recovery_code(&recovery_codes, &test_code));

        let used = two_factor.use_recovery_code(&mut recovery_codes, &test_code);
        assert!(used);
        assert_eq!(recovery_codes.len(), 9);

        // Verify it's removed
        assert!(!two_factor.use_recovery_code(&mut recovery_codes, &test_code));
    }

    #[test]
    fn test_session_management() {
        use foundry_auth_scaffolding::session::SessionManager;

        // Setup
        let service = create_test_auth_service();
        let session_manager = SessionManager::new();

        let user_id = Uuid::new_v4();
        let user = User::new(
            "Multi Session User".to_string(),
            "user@example.com".to_string(),
            "hash".to_string(),
        );

        // Create multiple sessions
        let session1 = service.create_session(&user, false);
        let session2 = service.create_session(&user, false);
        let session3 = service.create_session(&user, false);

        session_manager.store(session1.clone());
        session_manager.store(session2.clone());
        session_manager.store(session3.clone());

        // Verify sessions exist
        assert!(session_manager.find(&session1.token).is_some());
        assert!(session_manager.find(&session2.token).is_some());
        assert!(session_manager.find(&session3.token).is_some());

        // Logout specific session
        session_manager.delete(&session1.token);
        assert!(session_manager.find(&session1.token).is_none());
        assert!(session_manager.find(&session2.token).is_some());

        // Logout all sessions
        session_manager.delete_for_user(user_id);
        // Note: sessions don't match user_id in this test, so they remain
        // In real app, sessions would have the correct user_id

        // Test with proper user_id
        let session4 = Session::new(user_id, "token4".to_string(), 3600);
        let session5 = Session::new(user_id, "token5".to_string(), 3600);

        session_manager.store(session4.clone());
        session_manager.store(session5.clone());

        // Verify sessions are cleaned up
        session_manager.delete_for_user(user_id);
        assert!(session_manager.find("token4").is_none());
        assert!(session_manager.find("token5").is_none());
    }

    #[test]
    fn test_expired_session_cleanup() {
        use foundry_auth_scaffolding::session::SessionManager;

        let session_manager = SessionManager::new();

        let user_id = Uuid::new_v4();

        // Create expired session
        let expired_session = Session::new(user_id, "expired_token".to_string(), -3600);
        session_manager.store(expired_session.clone());

        // Create valid session
        let valid_session = Session::new(user_id, "valid_token".to_string(), 3600);
        session_manager.store(valid_session.clone());

        // Verify expired session exists
        let found = session_manager.find("expired_token");
        assert!(found.is_some());
        assert!(found.unwrap().is_expired());

        // Cleanup expired sessions
        session_manager.cleanup_expired();

        // Expired session removed
        assert!(session_manager.find("expired_token").is_none());

        // Valid session remains
        assert!(session_manager.find("valid_token").is_some());
    }

    #[test]
    fn test_password_hashing_is_secure() {
        let service = create_test_auth_service();

        let password = "my_secure_password";

        // Hash same password multiple times
        let hash1 = service.hash_password(password).unwrap();
        let hash2 = service.hash_password(password).unwrap();

        // Hashes should be different (due to random salt)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(service.verify_password(password, &hash1).unwrap());
        assert!(service.verify_password(password, &hash2).unwrap());

        // Wrong password should not verify
        assert!(!service.verify_password("wrong", &hash1).unwrap());
    }

    #[test]
    fn test_session_token_uniqueness() {
        let service = create_test_auth_service();

        let user = User::new(
            "Test".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
        );

        // Generate multiple sessions
        let session1 = service.create_session(&user, false);
        let session2 = service.create_session(&user, false);
        let session3 = service.create_session(&user, false);

        // All tokens should be unique
        assert_ne!(session1.token, session2.token);
        assert_ne!(session1.token, session3.token);
        assert_ne!(session2.token, session3.token);
    }

    #[test]
    fn test_password_reset_for_multiple_requests() {
        use foundry_auth_scaffolding::password::PasswordResetManager;
        use foundry_auth_scaffolding::models::PasswordReset;

        let service = create_test_auth_service();
        let reset_manager = PasswordResetManager::new();

        let email = "user@example.com".to_string();

        // User requests password reset multiple times
        let token1 = service.generate_token();
        let token2 = service.generate_token();
        let token3 = service.generate_token();

        let reset1 = PasswordReset::new(email.clone(), token1.clone(), 3600);
        let reset2 = PasswordReset::new(email.clone(), token2.clone(), 3600);
        let reset3 = PasswordReset::new(email.clone(), token3.clone(), 3600);

        reset_manager.store(reset1);
        reset_manager.store(reset2);
        reset_manager.store(reset3);

        // All tokens exist
        assert!(reset_manager.find(&token1).is_some());
        assert!(reset_manager.find(&token2).is_some());
        assert!(reset_manager.find(&token3).is_some());

        // Clean up all tokens for email after successful reset
        reset_manager.delete_for_email(&email);

        // All tokens should be removed
        assert!(reset_manager.find(&token1).is_none());
        assert!(reset_manager.find(&token2).is_none());
        assert!(reset_manager.find(&token3).is_none());
    }

    #[test]
    fn test_user_profile_updates() {
        let _service = create_test_auth_service();

        let mut user = User::new(
            "Old Name".to_string(),
            "user@example.com".to_string(),
            "hash".to_string(),
        );

        // Update profile
        user.name = "New Name".to_string();
        user.updated_at = chrono::Utc::now();

        assert_eq!(user.name, "New Name");

        // Enable 2FA
        user.enable_two_factor("secret".to_string(), vec!["code1".to_string()]);
        assert!(user.has_two_factor());

        // Disable 2FA
        user.disable_two_factor();
        assert!(!user.has_two_factor());
    }
}
