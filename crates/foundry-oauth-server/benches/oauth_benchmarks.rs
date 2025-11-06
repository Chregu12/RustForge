use criterion::{black_box, criterion_group, criterion_main, Criterion};
use foundry_oauth_server::*;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

// ============================================================================
// Token Generation Benchmarks
// ============================================================================

fn token_generation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_generation");

    let config = OAuth2Config::default();
    let generator = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());

    // Access token generation
    group.bench_function("generate_access_token", |b| {
        b.iter(|| {
            generator.generate_access_token(
                black_box(Uuid::new_v4()),
                black_box(Some(Uuid::new_v4())),
                black_box(vec!["users:read".to_string(), "users:write".to_string()]),
                black_box(3600),
            )
        })
    });

    // Access token without user (client credentials)
    group.bench_function("generate_access_token_client_credentials", |b| {
        b.iter(|| {
            generator.generate_access_token(
                black_box(Uuid::new_v4()),
                black_box(None),
                black_box(vec!["api:read".to_string()]),
                black_box(3600),
            )
        })
    });

    // Refresh token generation
    group.bench_function("generate_refresh_token", |b| {
        b.iter(|| {
            generator.generate_refresh_token(
                black_box(Uuid::new_v4()),
                black_box(2592000),
            )
        })
    });

    // Personal access token (long-lived)
    group.bench_function("generate_personal_access_token", |b| {
        b.iter(|| {
            generator.generate_access_token(
                black_box(Uuid::new_v4()),
                black_box(Some(Uuid::new_v4())),
                black_box(vec!["*".to_string()]),
                black_box(31536000), // 1 year
            )
        })
    });

    group.finish();
}

// ============================================================================
// Token Validation Benchmarks
// ============================================================================

fn token_validation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_validation");

    let config = OAuth2Config::default();
    let generator = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());
    let validator = TokenValidator::new(config.jwt_secret.clone(), config.issuer.clone());

    // Generate test tokens
    let access_token = generator.generate_access_token(
        Uuid::new_v4(),
        Some(Uuid::new_v4()),
        vec!["users:read".to_string()],
        3600,
    ).unwrap();

    let multi_scope_token = generator.generate_access_token(
        Uuid::new_v4(),
        Some(Uuid::new_v4()),
        vec!["users:read".to_string(), "users:write".to_string(), "api:read".to_string()],
        3600,
    ).unwrap();

    // JWT validation and parsing
    group.bench_function("validate_access_token", |b| {
        b.iter(|| {
            validator.validate_access_token(black_box(&access_token.token))
        })
    });

    // Validation with multiple scopes
    group.bench_function("validate_token_multiple_scopes", |b| {
        b.iter(|| {
            validator.validate_access_token(black_box(&multi_scope_token.token))
        })
    });

    // Token introspection
    group.bench_function("introspect_token", |b| {
        b.iter(|| {
            validator.introspect(black_box(&access_token.token))
        })
    });

    // Invalid token handling
    group.bench_function("validate_invalid_token", |b| {
        b.iter(|| {
            validator.validate_access_token(black_box("invalid.jwt.token"))
        })
    });

    group.finish();
}

// ============================================================================
// PKCE Operations Benchmarks
// ============================================================================

fn pkce_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("pkce_operations");

    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    // S256 code challenge computation
    group.bench_function("compute_s256_challenge", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(verifier.as_bytes()));
            let hash = hasher.finalize();
            URL_SAFE_NO_PAD.encode(&hash)
        })
    });

    // S256 verification (full flow)
    group.bench_function("verify_s256_challenge", |b| {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(&hash);

        b.iter(|| {
            // Re-compute and compare
            let mut hasher = Sha256::new();
            hasher.update(black_box(verifier.as_bytes()));
            let computed_hash = hasher.finalize();
            let computed_challenge = URL_SAFE_NO_PAD.encode(&computed_hash);

            // Constant-time comparison
            use subtle::ConstantTimeEq;
            computed_challenge.as_bytes().ct_eq(black_box(challenge.as_bytes()))
        })
    });

    // Plain method comparison
    group.bench_function("verify_plain_challenge", |b| {
        b.iter(|| {
            use subtle::ConstantTimeEq;
            verifier.as_bytes().ct_eq(black_box(verifier.as_bytes()))
        })
    });

    // Code verifier generation
    group.bench_function("generate_code_verifier", |b| {
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
// Client Operations Benchmarks
// ============================================================================

fn client_operations_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_operations");

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let repo = clients::InMemoryClientRepository::new();

    // Create test client
    let client = models::Client::new(
        "Test Client".to_string(),
        vec!["http://localhost/callback".to_string()],
    );
    let client_id = client.id;
    let secret = client.secret.clone().unwrap();

    runtime.block_on(async {
        repo.store(client).await.unwrap();
    });

    // Client lookup
    group.bench_function("find_client_by_id", |b| {
        b.to_async(&runtime).iter(|| async {
            repo.find(black_box(client_id)).await
        })
    });

    // Client authentication (includes Argon2 verification)
    group.bench_function("authenticate_client_with_secret", |b| {
        b.to_async(&runtime).iter(|| async {
            repo.find_by_credentials(black_box(client_id), black_box(&secret)).await
        })
    });

    // Failed authentication
    group.bench_function("authenticate_client_wrong_secret", |b| {
        b.to_async(&runtime).iter(|| async {
            repo.find_by_credentials(black_box(client_id), black_box("wrong_secret")).await
        })
    });

    // List all clients
    group.bench_function("list_all_clients", |b| {
        b.to_async(&runtime).iter(|| async {
            repo.list().await
        })
    });

    group.finish();
}

// ============================================================================
// Scope Validation Benchmarks
// ============================================================================

fn scope_validation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("scope_validation");

    let manager = ScopeManager::with_defaults();

    // Parse single scope
    group.bench_function("validate_single_scope", |b| {
        b.iter(|| {
            manager.validate(black_box(&vec!["users:read".to_string()]))
        })
    });

    // Parse multiple scopes
    group.bench_function("validate_multiple_scopes", |b| {
        b.iter(|| {
            manager.validate(black_box(&vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "api:read".to_string(),
                "api:write".to_string(),
            ]))
        })
    });

    // Validate wildcard scope
    group.bench_function("validate_wildcard_scope", |b| {
        b.iter(|| {
            manager.validate(black_box(&vec!["*".to_string()]))
        })
    });

    // Scope satisfaction check (simple)
    group.bench_function("check_scope_satisfaction_simple", |b| {
        b.iter(|| {
            manager.satisfies(
                black_box(&vec!["users:read".to_string()]),
                black_box(&vec!["users:read".to_string()]),
            )
        })
    });

    // Scope satisfaction check (complex)
    group.bench_function("check_scope_satisfaction_complex", |b| {
        b.iter(|| {
            manager.satisfies(
                black_box(&vec![
                    "users:read".to_string(),
                    "users:write".to_string(),
                    "api:read".to_string(),
                ]),
                black_box(&vec!["users:read".to_string(), "api:read".to_string()]),
            )
        })
    });

    // Scope satisfaction with wildcard
    group.bench_function("check_scope_satisfaction_wildcard", |b| {
        b.iter(|| {
            manager.satisfies(
                black_box(&vec!["*".to_string()]),
                black_box(&vec!["users:read".to_string(), "api:write".to_string()]),
            )
        })
    });

    // Filter scopes by pattern
    group.bench_function("filter_scopes_by_pattern", |b| {
        b.iter(|| {
            manager.filter(black_box("users:"))
        })
    });

    group.finish();
}

// ============================================================================
// Authorization Code Grant Benchmarks
// ============================================================================

fn authorization_code_grant_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("authorization_code_grant");

    let config = OAuth2Config::default();
    let token_gen = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());
    let grant = grants::AuthorizationCodeGrant::new(token_gen, 3600, 2592000);

    let client = models::Client::new(
        "Test Client".to_string(),
        vec!["http://localhost/callback".to_string()],
    );

    // Generate authorization code
    group.bench_function("create_authorization_code", |b| {
        b.iter(|| {
            let params = grants::AuthCodeParams {
                user_id: black_box(Uuid::new_v4()),
                redirect_uri: black_box("http://localhost/callback".to_string()),
                scopes: black_box(vec!["users:read".to_string()]),
                code_challenge: black_box(None),
                code_challenge_method: black_box(None),
                lifetime: black_box(600),
            };
            grant.create_authorization_code(black_box(&client), black_box(params))
        })
    });

    // Generate authorization code with PKCE
    group.bench_function("create_authorization_code_with_pkce", |b| {
        let mut hasher = Sha256::new();
        hasher.update(b"test_verifier_123");
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(&hash);

        b.iter(|| {
            let params = grants::AuthCodeParams {
                user_id: black_box(Uuid::new_v4()),
                redirect_uri: black_box("http://localhost/callback".to_string()),
                scopes: black_box(vec!["users:read".to_string()]),
                code_challenge: black_box(Some(challenge.clone())),
                code_challenge_method: black_box(Some("S256".to_string())),
                lifetime: black_box(600),
            };
            grant.create_authorization_code(black_box(&client), black_box(params))
        })
    });

    group.finish();
}

// ============================================================================
// Full OAuth2 Flow Benchmarks
// ============================================================================

fn full_flow_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_oauth_flows");

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let config = OAuth2Config::default();
    let token_gen = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());
    let validator = TokenValidator::new(config.jwt_secret.clone(), config.issuer.clone());

    // Full authorization code flow (without PKCE)
    group.bench_function("full_authorization_code_flow", |b| {
        b.to_async(&runtime).iter(|| async {
            let grant = grants::AuthorizationCodeGrant::new(
                token_gen.clone(),
                3600,
                2592000,
            );

            let client = models::Client::new(
                "Test".to_string(),
                vec!["http://localhost/callback".to_string()],
            );

            // Step 1: Create authorization code
            let params = grants::AuthCodeParams {
                user_id: Uuid::new_v4(),
                redirect_uri: "http://localhost/callback".to_string(),
                scopes: vec!["users:read".to_string()],
                code_challenge: None,
                code_challenge_method: None,
                lifetime: 600,
            };
            let auth_code = grant.create_authorization_code(&client, params).unwrap();

            // Step 2: Exchange code for tokens
            let token_response = grant.exchange_code(&client, &auth_code, None).await.unwrap();

            // Step 3: Validate access token
            validator.validate_access_token(&token_response.access_token).unwrap();
        })
    });

    // Full authorization code flow with PKCE
    group.bench_function("full_authorization_code_flow_with_pkce", |b| {
        b.to_async(&runtime).iter(|| async {
            let grant = grants::AuthorizationCodeGrant::new(
                token_gen.clone(),
                3600,
                2592000,
            );

            let client = models::Client::new(
                "Test".to_string(),
                vec!["http://localhost/callback".to_string()],
            );

            let verifier = "test_verifier_1234567890";
            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            let hash = hasher.finalize();
            let challenge = URL_SAFE_NO_PAD.encode(&hash);

            // Step 1: Create authorization code with PKCE
            let params = grants::AuthCodeParams {
                user_id: Uuid::new_v4(),
                redirect_uri: "http://localhost/callback".to_string(),
                scopes: vec!["users:read".to_string()],
                code_challenge: Some(challenge),
                code_challenge_method: Some("S256".to_string()),
                lifetime: 600,
            };
            let auth_code = grant.create_authorization_code(&client, params).unwrap();

            // Step 2: Exchange code for tokens with verifier
            let token_response = grant.exchange_code(&client, &auth_code, Some(verifier.to_string())).await.unwrap();

            // Step 3: Validate access token
            validator.validate_access_token(&token_response.access_token).unwrap();
        })
    });

    // Client credentials flow
    group.bench_function("full_client_credentials_flow", |b| {
        b.to_async(&runtime).iter(|| async {
            let grant = grants::ClientCredentialsGrant::new(token_gen.clone(), 3600);

            let client = models::Client::new(
                "Test".to_string(),
                vec!["http://localhost/callback".to_string()],
            );

            // Issue token
            let token_response = grant.issue_token(&client, vec!["api:read".to_string()]).await.unwrap();

            // Validate token
            validator.validate_access_token(&token_response.access_token).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    token_generation_benchmarks,
    token_validation_benchmarks,
    pkce_benchmarks,
    client_operations_benchmarks,
    scope_validation_benchmarks,
    authorization_code_grant_benchmarks,
    full_flow_benchmarks
);

criterion_main!(benches);
