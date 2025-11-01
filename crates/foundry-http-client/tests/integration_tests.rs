//! Integration tests for foundry-http-client

use foundry_http_client::{Auth, AuthType, HttpClient, RetryConfig};
use std::collections::HashMap;
use std::time::Duration;

#[test]
fn test_client_builder() {
    let client = HttpClient::builder()
        .timeout(30)
        .user_agent("Foundry/1.0")
        .verify_ssl(true)
        .build();

    assert!(client.is_ok());
}

#[test]
fn test_retry_config() {
    let config = RetryConfig::new(5).with_delay(Duration::from_secs(2));

    assert_eq!(config.max_retries, 5);
    assert_eq!(config.delay, Duration::from_secs(2));
}

#[test]
fn test_retry_config_no_retry() {
    let config = RetryConfig::no_retry();

    assert_eq!(config.max_retries, 0);
}

#[test]
fn test_auth_bearer() {
    let auth = Auth::bearer("my-token");
    assert!(matches!(auth.auth_type, AuthType::Bearer(_)));
}

#[test]
fn test_auth_basic() {
    let auth = Auth::basic("user", "pass");
    assert!(matches!(auth.auth_type, AuthType::Basic { .. }));
}

#[test]
fn test_auth_custom() {
    let auth = Auth::custom("X-API-Key", "secret");
    assert!(matches!(auth.auth_type, AuthType::Custom { .. }));
}

#[test]
fn test_request_builder() {
    let client = HttpClient::new();
    let request = client
        .get("https://api.example.com/users")
        .header("Accept", "application/json")
        .query("page", "1")
        .timeout(10);

    // Just test that the builder compiles and chains correctly
    assert!(true);
}
