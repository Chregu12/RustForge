use criterion::{black_box, criterion_group, criterion_main, Criterion};
use foundry_auth_scaffolding::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

// ============================================================================
// Password Hashing Benchmarks
// ============================================================================

fn password_hashing_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("password_hashing");

    let config = AuthConfig::default();
    let service = AuthService::new(config);

    let password = "my_secure_password_12345";
    let hash = service.hash_password(password).unwrap();

    // Argon2 hash generation (expensive!)
    group.bench_function("hash_password_argon2", |b| {
        b.iter(|| {
            service.hash_password(black_box(password))
        })
    });

    // Argon2 verification (expensive!)
    group.bench_function("verify_password_argon2", |b| {
        b.iter(|| {
            service.verify_password(black_box(password), black_box(&hash))
        })
    });

    // Verify with wrong password (should still be expensive)
    group.bench_function("verify_password_wrong", |b| {
        b.iter(|| {
            service.verify_password(black_box("wrong_password"), black_box(&hash))
        })
    });

    group.finish();
}

// ============================================================================
// Session Operations Benchmarks
// ============================================================================

fn session_operations_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_operations");

    let manager = SessionManager::new();
    let config = AuthConfig::default();
    let service = AuthService::new(config.clone());

    let user = models::User::new(
        "John Doe".to_string(),
        "john@example.com".to_string(),
        "hash".to_string(),
    );

    // Session creation
    group.bench_function("create_session", |b| {
        b.iter(|| {
            service.create_session(black_box(&user), black_box(false))
        })
    });

    // Session creation with remember me (longer lifetime)
    group.bench_function("create_session_remember_me", |b| {
        b.iter(|| {
            service.create_session(black_box(&user), black_box(true))
        })
    });

    // Session storage
    let session = service.create_session(&user, false);
    group.bench_function("store_session", |b| {
        b.iter(|| {
            manager.store(black_box(session.clone()))
        })
    });

    // Store initial session for lookup tests
    let lookup_session = service.create_session(&user, false);
    let token = lookup_session.token.clone();
    manager.store(lookup_session);

    // Session lookup
    group.bench_function("find_session", |b| {
        b.iter(|| {
            manager.find(black_box(&token))
        })
    });

    // Session deletion
    group.bench_function("delete_session", |b| {
        let test_session = service.create_session(&user, false);
        let test_token = test_session.token.clone();
        manager.store(test_session);

        b.iter(|| {
            manager.delete(black_box(&test_token))
        })
    });

    // Delete all sessions for user
    group.bench_function("delete_all_user_sessions", |b| {
        // Pre-populate with multiple sessions
        for _ in 0..10 {
            let sess = service.create_session(&user, false);
            manager.store(sess);
        }

        b.iter(|| {
            manager.delete_for_user(black_box(user.id))
        })
    });

    // Session cleanup (expired sessions)
    group.bench_function("cleanup_expired_sessions", |b| {
        b.iter(|| {
            manager.cleanup_expired()
        })
    });

    group.finish();
}

// ============================================================================
// Two-Factor Authentication Benchmarks
// ============================================================================

#[cfg(feature = "two-factor")]
fn two_factor_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_factor_auth");

    let service = two_factor::TwoFactorService::new("Test App".to_string());

    // Generate TOTP secret
    group.bench_function("generate_totp_secret", |b| {
        b.iter(|| {
            service.generate_secret()
        })
    });

    // Generate recovery codes
    group.bench_function("generate_recovery_codes_8", |b| {
        b.iter(|| {
            service.generate_recovery_codes(black_box(8))
        })
    });

    group.bench_function("generate_recovery_codes_10", |b| {
        b.iter(|| {
            service.generate_recovery_codes(black_box(10))
        })
    });

    // Generate QR code
    let secret = service.generate_secret();
    group.bench_function("generate_qr_code", |b| {
        b.iter(|| {
            service.generate_qr_code(black_box("user@example.com"), black_box(&secret))
        })
    });

    // TOTP code verification
    // Note: This will fail since we're using a random secret, but measures performance
    group.bench_function("verify_totp_code", |b| {
        let test_secret = "JBSWY3DPEHPK3PXP"; // Base32 encoded test secret
        b.iter(|| {
            service.verify_code(black_box(test_secret), black_box("123456"))
        })
    });

    // Recovery code verification (constant-time)
    let recovery_codes = vec![
        "1234-5678".to_string(),
        "8765-4321".to_string(),
        "1111-2222".to_string(),
        "3333-4444".to_string(),
        "5555-6666".to_string(),
    ];

    group.bench_function("verify_recovery_code_hit", |b| {
        b.iter(|| {
            service.verify_recovery_code(black_box(&recovery_codes), black_box("1234-5678"))
        })
    });

    group.bench_function("verify_recovery_code_miss", |b| {
        b.iter(|| {
            service.verify_recovery_code(black_box(&recovery_codes), black_box("9999-9999"))
        })
    });

    // Use recovery code (remove from list)
    group.bench_function("use_recovery_code", |b| {
        let codes = recovery_codes.clone();
        b.iter(|| {
            let mut test_codes = codes.clone();
            service.use_recovery_code(black_box(&mut test_codes), black_box("1234-5678"))
        })
    });

    group.finish();
}

// ============================================================================
// Token Generation Benchmarks
// ============================================================================

fn token_generation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_generation");

    let config = AuthConfig::default();
    let service = AuthService::new(config);

    // Generate random token (for password reset, email verification, etc.)
    group.bench_function("generate_random_token", |b| {
        b.iter(|| {
            service.generate_token()
        })
    });

    // Generate session token (64 bytes)
    group.bench_function("generate_session_token", |b| {
        b.iter(|| {
            use rand::Rng;
            let random_bytes: Vec<u8> = rand::thread_rng()
                .sample_iter(rand::distributions::Standard)
                .take(64)
                .collect();
            URL_SAFE_NO_PAD.encode(&random_bytes)
        })
    });

    // Generate smaller token (32 bytes)
    group.bench_function("generate_token_32_bytes", |b| {
        b.iter(|| {
            use rand::Rng;
            let random_bytes: Vec<u8> = rand::thread_rng()
                .sample_iter(rand::distributions::Standard)
                .take(32)
                .collect();
            URL_SAFE_NO_PAD.encode(&random_bytes)
        })
    });

    group.finish();
}

// ============================================================================
// User Registration Benchmarks
// ============================================================================

fn user_registration_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("user_registration");

    let config = AuthConfig::default();
    let service = AuthService::new(config);

    let register_data = RegisterData {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        password: "secure_password_123".to_string(),
        password_confirmation: "secure_password_123".to_string(),
    };

    // Full registration flow (includes password hashing)
    group.bench_function("register_user", |b| {
        b.iter(|| {
            service.register(black_box(register_data.clone()))
        })
    });

    // Validation only (no hashing)
    group.bench_function("validate_registration_data", |b| {
        b.iter(|| {
            black_box(&register_data).validate()
        })
    });

    // Failed validation (mismatched passwords)
    let invalid_data = RegisterData {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        password: "password123".to_string(),
        password_confirmation: "different".to_string(),
    };

    group.bench_function("validate_registration_data_invalid", |b| {
        b.iter(|| {
            black_box(&invalid_data).validate()
        })
    });

    group.finish();
}

// ============================================================================
// Authentication Flow Benchmarks
// ============================================================================

fn authentication_flow_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("authentication_flow");

    let config = AuthConfig::default();
    let service = AuthService::new(config);

    let password = "password123";
    let password_hash = service.hash_password(password).unwrap();
    let mut user = models::User::new(
        "John Doe".to_string(),
        "john@example.com".to_string(),
        password_hash,
    );
    user.mark_email_as_verified();

    let credentials = Credentials {
        email: "john@example.com".to_string(),
        password: password.to_string(),
        remember: false,
    };

    let credentials_remember = Credentials {
        email: "john@example.com".to_string(),
        password: password.to_string(),
        remember: true,
    };

    // Full authentication attempt (includes password verification)
    group.bench_function("attempt_authentication", |b| {
        b.iter(|| {
            service.attempt(black_box(&user), black_box(&credentials))
        })
    });

    // Authentication with remember me
    group.bench_function("attempt_authentication_remember", |b| {
        b.iter(|| {
            service.attempt(black_box(&user), black_box(&credentials_remember))
        })
    });

    // Failed authentication (wrong password)
    let bad_credentials = Credentials {
        email: "john@example.com".to_string(),
        password: "wrong_password".to_string(),
        remember: false,
    };

    group.bench_function("attempt_authentication_wrong_password", |b| {
        b.iter(|| {
            service.attempt(black_box(&user), black_box(&bad_credentials))
        })
    });

    group.finish();
}

// ============================================================================
// Full Login Flow Benchmarks
// ============================================================================

fn full_login_flow_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_login_flow");

    let config = AuthConfig::default();
    let service = AuthService::new(config);
    let manager = SessionManager::new();

    let password = "password123";
    let password_hash = service.hash_password(password).unwrap();
    let mut user = models::User::new(
        "John Doe".to_string(),
        "john@example.com".to_string(),
        password_hash,
    );
    user.mark_email_as_verified();

    let credentials = Credentials {
        email: "john@example.com".to_string(),
        password: password.to_string(),
        remember: false,
    };

    // Complete login flow: authenticate + create session + store session
    group.bench_function("complete_login_flow", |b| {
        b.iter(|| {
            // Step 1: Authenticate
            service.attempt(black_box(&user), black_box(&credentials)).unwrap();

            // Step 2: Create session
            let session = service.create_session(black_box(&user), black_box(false));

            // Step 3: Store session
            manager.store(black_box(session));
        })
    });

    // Complete login flow with remember me
    let credentials_remember = Credentials {
        email: "john@example.com".to_string(),
        password: password.to_string(),
        remember: true,
    };

    group.bench_function("complete_login_flow_remember", |b| {
        b.iter(|| {
            service.attempt(black_box(&user), black_box(&credentials_remember)).unwrap();
            let session = service.create_session(black_box(&user), black_box(true));
            manager.store(black_box(session));
        })
    });

    group.finish();
}

// ============================================================================
// Password Reset Benchmarks
// ============================================================================

fn password_reset_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("password_reset");

    use foundry_auth_scaffolding::password::PasswordResetManager;

    let manager = PasswordResetManager::new();
    let config = AuthConfig::default();
    let service = AuthService::new(config);

    // Generate password reset token
    group.bench_function("generate_reset_token", |b| {
        b.iter(|| {
            service.generate_token()
        })
    });

    // Store password reset
    group.bench_function("store_password_reset", |b| {
        let token = service.generate_token();
        let reset = models::PasswordReset::new(
            "user@example.com".to_string(),
            token,
            3600,
        );

        b.iter(|| {
            manager.store(black_box(reset.clone()))
        })
    });

    // Find password reset by token
    let token = service.generate_token();
    let reset = models::PasswordReset::new(
        "user@example.com".to_string(),
        token.clone(),
        3600,
    );
    manager.store(reset);

    group.bench_function("find_password_reset", |b| {
        b.iter(|| {
            manager.find(black_box(&token))
        })
    });

    // Delete password reset
    group.bench_function("delete_password_reset", |b| {
        let test_token = service.generate_token();
        let test_reset = models::PasswordReset::new(
            "test@example.com".to_string(),
            test_token.clone(),
            3600,
        );
        manager.store(test_reset);

        b.iter(|| {
            manager.delete(black_box(&test_token))
        })
    });

    // Delete all resets for email
    group.bench_function("delete_resets_for_email", |b| {
        let email = "user@example.com";
        // Pre-populate with multiple resets
        for _ in 0..5 {
            let token = service.generate_token();
            let reset = models::PasswordReset::new(email.to_string(), token, 3600);
            manager.store(reset);
        }

        b.iter(|| {
            manager.delete_for_email(black_box(email))
        })
    });

    // Cleanup expired resets
    group.bench_function("cleanup_expired_resets", |b| {
        b.iter(|| {
            manager.cleanup_expired()
        })
    });

    group.finish();
}

// ============================================================================
// Comparison Benchmarks (Different Hash Complexities)
// ============================================================================

fn argon2_complexity_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("argon2_complexity");

    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        Params,
    };

    let password = "test_password_123";

    // Default Argon2 (balanced)
    group.bench_function("argon2_default", |b| {
        let argon2 = Argon2::default();
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            let _ = argon2.hash_password(black_box(password.as_bytes()), &salt);
        })
    });

    // Low memory (faster, less secure - for testing environments)
    group.bench_function("argon2_low_memory", |b| {
        let params = Params::new(4096, 2, 1, None).unwrap();
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            let _ = argon2.hash_password(black_box(password.as_bytes()), &salt);
        })
    });

    // High memory (slower, more secure - for production)
    group.bench_function("argon2_high_memory", |b| {
        let params = Params::new(65536, 3, 2, None).unwrap();
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            let _ = argon2.hash_password(black_box(password.as_bytes()), &salt);
        })
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

#[cfg(feature = "two-factor")]
criterion_group!(
    benches,
    password_hashing_benchmarks,
    session_operations_benchmarks,
    two_factor_benchmarks,
    token_generation_benchmarks,
    user_registration_benchmarks,
    authentication_flow_benchmarks,
    full_login_flow_benchmarks,
    password_reset_benchmarks,
    argon2_complexity_comparison
);

#[cfg(not(feature = "two-factor"))]
criterion_group!(
    benches,
    password_hashing_benchmarks,
    session_operations_benchmarks,
    token_generation_benchmarks,
    user_registration_benchmarks,
    authentication_flow_benchmarks,
    full_login_flow_benchmarks,
    password_reset_benchmarks,
    argon2_complexity_comparison
);

criterion_main!(benches);
