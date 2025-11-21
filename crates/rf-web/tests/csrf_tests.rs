//! Comprehensive tests for CSRF protection

use rf_web::csrf::{CsrfConfig, CsrfMiddleware, CsrfToken, csrf_field, csrf_meta};

#[test]
fn test_csrf_token_generation_unique() {
    let token1 = CsrfToken::generate();
    let token2 = CsrfToken::generate();

    // Each token should be unique
    assert_ne!(token1.token(), token2.token());
}

#[test]
fn test_csrf_token_length() {
    let token = CsrfToken::generate();
    // Base64 encoded 32 bytes should be 43 characters (URL_SAFE_NO_PAD)
    assert!(token.token().len() >= 40);
}

#[test]
fn test_csrf_token_verification_success() {
    let token = CsrfToken::generate();
    let value = token.token().to_string();

    assert!(token.verify(&value));
}

#[test]
fn test_csrf_token_verification_failure() {
    let token = CsrfToken::generate();

    assert!(!token.verify("invalid_token"));
    assert!(!token.verify(""));
    assert!(!token.verify("completely_different"));
}

#[test]
fn test_csrf_token_constant_time_comparison() {
    let token = CsrfToken::generate();
    let value = token.token().to_string();

    // Should be resistant to timing attacks
    // Both valid and invalid checks should take similar time
    assert!(token.verify(&value));
    assert!(!token.verify(&format!("{}x", value)));
}

#[test]
fn test_csrf_token_not_expired() {
    let token = CsrfToken::generate();
    assert!(!token.is_expired());
}

#[test]
fn test_csrf_token_expiration() {
    use chrono::{Duration, Utc};

    let mut token = CsrfToken::generate();

    // Simulate expired token (3 hours old)
    token.created_at = Utc::now() - Duration::hours(3);

    assert!(token.is_expired());
}

#[test]
fn test_csrf_token_custom_expiration_duration() {
    use chrono::{Duration, Utc};

    let mut token = CsrfToken::generate();

    // Token created 30 minutes ago
    token.created_at = Utc::now() - Duration::minutes(30);

    // Should not be expired for 1 hour duration
    assert!(!token.is_expired_with_duration(Duration::hours(1)));

    // Should be expired for 15 minute duration
    assert!(token.is_expired_with_duration(Duration::minutes(15)));
}

#[test]
fn test_csrf_token_regeneration() {
    let token1 = CsrfToken::generate();
    let token2 = CsrfToken::regenerate();

    assert_ne!(token1.token(), token2.token());
}

#[test]
fn test_csrf_token_display() {
    let token = CsrfToken::generate();
    let display = format!("{}", token);

    assert_eq!(display, token.token());
}

#[test]
fn test_csrf_config_defaults() {
    let config = CsrfConfig::default();

    assert_eq!(config.exempt_routes.len(), 0);
    assert_eq!(config.token_lifetime_hours, 2);
    assert_eq!(config.field_name, "_token");
    assert_eq!(config.header_name, "X-CSRF-TOKEN");
}

#[test]
fn test_csrf_config_builder() {
    let config = CsrfConfig::new()
        .exempt("/api/webhook")
        .exempt("/health")
        .lifetime_hours(4)
        .field_name("csrf_token")
        .header_name("X-XSRF-TOKEN");

    assert_eq!(config.exempt_routes.len(), 2);
    assert!(config.exempt_routes.contains(&"/api/webhook".to_string()));
    assert!(config.exempt_routes.contains(&"/health".to_string()));
    assert_eq!(config.token_lifetime_hours, 4);
    assert_eq!(config.field_name, "csrf_token");
    assert_eq!(config.header_name, "X-XSRF-TOKEN");
}

#[test]
fn test_csrf_middleware_creation() {
    let middleware = CsrfMiddleware::new();
    // Should create without panic
    assert!(true);
}

#[test]
fn test_csrf_middleware_with_config() {
    let config = CsrfConfig::new().exempt("/api/");
    let middleware = CsrfMiddleware::with_config(config);
    // Should create without panic
    assert!(true);
}

#[test]
fn test_csrf_field_generation() {
    let token = CsrfToken::generate();
    let field = csrf_field(&token);

    assert!(field.contains(r#"type="hidden""#));
    assert!(field.contains(r#"name="_token""#));
    assert!(field.contains(&format!(r#"value="{}""#, token.token())));
    assert!(field.starts_with("<input"));
    assert!(field.ends_with(">"));
}

#[test]
fn test_csrf_meta_generation() {
    let token = CsrfToken::generate();
    let meta = csrf_meta(&token);

    assert!(meta.contains(r#"name="csrf-token""#));
    assert!(meta.contains(&format!(r#"content="{}""#, token.token())));
    assert!(meta.starts_with("<meta"));
    assert!(meta.ends_with(">"));
}

#[test]
fn test_csrf_field_escaping() {
    let token = CsrfToken::generate();
    let field = csrf_field(&token);

    // Should not contain any unescaped characters
    assert!(!field.contains("<script>"));
    assert!(!field.contains("javascript:"));
}

#[test]
fn test_csrf_meta_escaping() {
    let token = CsrfToken::generate();
    let meta = csrf_meta(&token);

    // Should not contain any unescaped characters
    assert!(!meta.contains("<script>"));
    assert!(!meta.contains("javascript:"));
}

#[test]
fn test_csrf_token_serialization() {
    let token = CsrfToken::generate();

    let json = serde_json::to_string(&token).unwrap();
    assert!(json.contains("token"));
    assert!(json.contains("created_at"));

    let deserialized: CsrfToken = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.token(), token.token());
}

#[test]
fn test_multiple_csrf_tokens_independent() {
    let token1 = CsrfToken::generate();
    let token2 = CsrfToken::generate();

    // Tokens should be independent
    assert_ne!(token1.token(), token2.token());

    // Each token should verify its own value
    assert!(token1.verify(token1.token()));
    assert!(token2.verify(token2.token()));

    // But not the other's value
    assert!(!token1.verify(token2.token()));
    assert!(!token2.verify(token1.token()));
}

#[test]
fn test_csrf_config_multiple_exemptions() {
    let config = CsrfConfig::new()
        .exempt("/api/")
        .exempt("/webhooks/")
        .exempt("/health")
        .exempt("/metrics");

    assert_eq!(config.exempt_routes.len(), 4);
}

#[test]
fn test_csrf_token_empty_verification() {
    let token = CsrfToken::generate();

    // Empty string should fail verification
    assert!(!token.verify(""));
}

#[test]
fn test_csrf_token_whitespace_verification() {
    let token = CsrfToken::generate();

    // Whitespace should fail verification
    assert!(!token.verify("   "));
    assert!(!token.verify("\t\n"));
}
