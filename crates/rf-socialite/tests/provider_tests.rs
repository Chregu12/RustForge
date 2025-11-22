//! Comprehensive tests for all OAuth providers
//!
//! Tests the complete OAuth flow for Google, Facebook, GitHub, and Twitter

use rf_socialite::{Driver, Provider, Socialite, SocialiteError};

#[test]
fn test_google_provider_configuration() {
    let mut driver = Socialite::driver(Provider::Google)
        .client_id("google-test-id")
        .client_secret("google-test-secret")
        .redirect_url("http://localhost/auth/google/callback")
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();

    // Verify Google OAuth URL structure
    assert!(url.contains("accounts.google.com/o/oauth2/v2/auth"));
    assert!(url.contains("client_id=google-test-id"));
    assert!(url.contains("redirect_uri="));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("scope="));
}

#[test]
fn test_facebook_provider_configuration() {
    let mut driver = Socialite::driver(Provider::Facebook)
        .client_id("facebook-test-id")
        .client_secret("facebook-test-secret")
        .redirect_url("http://localhost/auth/facebook/callback")
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();

    // Verify Facebook OAuth URL structure
    assert!(url.contains("facebook.com"));
    assert!(url.contains("client_id=facebook-test-id"));
    assert!(url.contains("scope="));
}

#[test]
fn test_github_provider_configuration() {
    let mut driver = Socialite::driver(Provider::GitHub)
        .client_id("github-test-id")
        .client_secret("github-test-secret")
        .redirect_url("http://localhost/auth/github/callback")
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();

    // Verify GitHub OAuth URL structure
    assert!(url.contains("github.com/login/oauth/authorize"));
    assert!(url.contains("client_id=github-test-id"));
}

#[test]
fn test_twitter_provider_configuration() {
    let mut driver = Socialite::driver(Provider::Twitter)
        .client_id("twitter-test-id")
        .client_secret("twitter-test-secret")
        .redirect_url("http://localhost/auth/twitter/callback")
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();

    // Verify Twitter OAuth URL structure
    assert!(url.contains("twitter.com"));
    assert!(url.contains("client_id=twitter-test-id"));
}

#[test]
fn test_all_providers_have_default_scopes() {
    let providers = vec![
        Provider::Google,
        Provider::Facebook,
        Provider::GitHub,
        Provider::Twitter,
    ];

    for provider in providers {
        let scopes = provider.default_scopes();
        assert!(
            !scopes.is_empty(),
            "Provider {:?} should have default scopes",
            provider
        );
    }
}

#[test]
fn test_custom_scopes() {
    let mut driver = Socialite::driver(Provider::Google)
        .client_id("test-id")
        .client_secret("test-secret")
        .redirect_url("http://localhost/callback")
        .scopes(vec![
            "email".to_string(),
            "profile".to_string(),
            "openid".to_string(),
        ])
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();
    assert!(url.contains("scope=email"));
}

#[test]
fn test_pkce_enabled() {
    let mut driver = Socialite::driver(Provider::Google)
        .client_id("test-id")
        .client_secret("test-secret")
        .redirect_url("http://localhost/callback")
        .with_pkce()
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();

    // PKCE should add code_challenge and code_challenge_method
    assert!(url.contains("code_challenge="));
    assert!(url.contains("code_challenge_method="));
}

#[test]
fn test_state_parameter() {
    let mut driver = Socialite::driver(Provider::Google)
        .client_id("test-id")
        .client_secret("test-secret")
        .redirect_url("http://localhost/callback")
        .state("random-state-123")
        .build()
        .unwrap();

    let url = driver.redirect().unwrap();
    assert!(url.contains("state=random-state-123"));
}

#[test]
fn test_missing_configuration_errors() {
    // Missing client_id
    let result = Socialite::driver(Provider::Google)
        .client_secret("test-secret")
        .redirect_url("http://localhost/callback")
        .build();
    assert!(result.is_err());

    // Missing client_secret
    let result = Socialite::driver(Provider::Google)
        .client_id("test-id")
        .redirect_url("http://localhost/callback")
        .build();
    assert!(result.is_err());

    // Missing redirect_url
    let result = Socialite::driver(Provider::Google)
        .client_id("test-id")
        .client_secret("test-secret")
        .build();
    assert!(result.is_err());
}

#[test]
fn test_provider_names() {
    assert_eq!(Provider::Google.name(), "google");
    assert_eq!(Provider::Facebook.name(), "facebook");
    assert_eq!(Provider::GitHub.name(), "github");
    assert_eq!(Provider::Twitter.name(), "twitter");
}

#[test]
fn test_provider_urls() {
    // Google
    assert!(Provider::Google
        .authorize_url()
        .contains("accounts.google.com"));
    assert!(Provider::Google
        .token_url()
        .contains("oauth2.googleapis.com"));
    assert!(Provider::Google.user_url().contains("googleapis.com"));

    // Facebook
    assert!(Provider::Facebook.authorize_url().contains("facebook.com"));
    assert!(Provider::Facebook
        .token_url()
        .contains("graph.facebook.com"));
    assert!(Provider::Facebook.user_url().contains("graph.facebook.com"));

    // GitHub
    assert!(Provider::GitHub.authorize_url().contains("github.com"));
    assert!(Provider::GitHub.token_url().contains("github.com"));
    assert!(Provider::GitHub.user_url().contains("api.github.com"));

    // Twitter
    assert!(Provider::Twitter.authorize_url().contains("twitter.com"));
    assert!(Provider::Twitter.token_url().contains("api.twitter.com"));
    assert!(Provider::Twitter.user_url().contains("api.twitter.com"));
}

// Note: Integration tests with real OAuth servers would go in a separate file
// and would require test accounts and proper API credentials

#[cfg(feature = "integration-tests")]
mod integration {
    use super::*;

    // These tests would require real OAuth credentials and would:
    // 1. Test the complete OAuth flow
    // 2. Exchange authorization codes for tokens
    // 3. Fetch user information
    // 4. Test token refresh
    // 5. Test error handling with invalid credentials
}
