//! Integration tests for rf-socialite

use rf_socialite::*;

mod pkce_tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = Pkce::generate();
        assert!(!pkce.code_verifier.is_empty());
        assert!(!pkce.code_challenge.is_empty());
        assert_eq!(pkce.code_challenge_method, "S256");
    }

    #[test]
    fn test_pkce_verifier_length_requirements() {
        let pkce = Pkce::generate();
        assert!(pkce.code_verifier.len() >= 43, "Verifier too short");
        assert!(pkce.code_verifier.len() <= 128, "Verifier too long");
    }

    #[test]
    fn test_pkce_uniqueness() {
        let pkce1 = Pkce::generate();
        let pkce2 = Pkce::generate();
        assert_ne!(pkce1.code_verifier, pkce2.code_verifier);
        assert_ne!(pkce1.code_challenge, pkce2.code_challenge);
    }

    #[test]
    fn test_pkce_url_safe_encoding() {
        let pkce = Pkce::generate();
        // Should not contain URL-unsafe characters
        assert!(!pkce.code_verifier.contains('+'));
        assert!(!pkce.code_verifier.contains('/'));
        assert!(!pkce.code_verifier.contains('='));
    }
}

mod state_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_state_generation() {
        let manager = StateManager::new();
        let state = manager.generate();
        assert!(!state.is_empty());
        assert!(state.len() > 20);
    }

    #[test]
    fn test_state_verification() {
        let manager = StateManager::new();
        let state = manager.generate();
        assert!(manager.verify(&state));
    }

    #[test]
    fn test_state_invalid() {
        let manager = StateManager::new();
        assert!(!manager.verify("invalid-state-token"));
    }

    #[test]
    fn test_state_one_time_use() {
        let manager = StateManager::new();
        let state = manager.generate();
        assert!(manager.verify(&state), "First verification failed");
        assert!(!manager.verify(&state), "Second verification should fail");
    }

    #[test]
    fn test_state_expiration() {
        let manager = StateManager::with_ttl(Duration::from_millis(100));
        let state = manager.generate();
        std::thread::sleep(Duration::from_millis(150));
        assert!(!manager.verify(&state), "Expired state should not verify");
    }

    #[test]
    fn test_multiple_states() {
        let manager = StateManager::new();
        let state1 = manager.generate();
        let state2 = manager.generate();
        let state3 = manager.generate();

        assert_ne!(state1, state2);
        assert_ne!(state2, state3);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_state_cleanup() {
        let manager = StateManager::with_ttl(Duration::from_millis(50));
        manager.generate();
        manager.generate();

        std::thread::sleep(Duration::from_millis(100));
        manager.cleanup_expired();

        // After cleanup, old states should be gone
        let state = manager.generate();
        assert!(manager.verify(&state));
    }

    #[test]
    fn test_custom_ttl() {
        let ttl = Duration::from_secs(300);
        let manager = StateManager::with_ttl(ttl);
        let state = manager.generate_with_ttl(Duration::from_millis(100));

        std::thread::sleep(Duration::from_millis(150));
        assert!(!manager.verify(&state));
    }
}

mod config_tests {
    use super::*;

    #[test]
    fn test_provider_config_creation() {
        let config = ProviderConfig::new("client-id", "client-secret", "http://localhost/callback");
        assert_eq!(config.client_id, "client-id");
        assert_eq!(config.client_secret, "client-secret");
        assert_eq!(config.redirect_uri, "http://localhost/callback");
        assert!(config.scopes.is_empty());
    }

    #[test]
    fn test_provider_config_with_scopes() {
        let config = ProviderConfig::new("id", "secret", "uri")
            .with_scopes(vec!["email".to_string(), "profile".to_string()]);
        assert_eq!(config.scopes.len(), 2);
        assert_eq!(config.scopes[0], "email");
        assert_eq!(config.scopes[1], "profile");
    }

    #[test]
    fn test_socialite_config_builder() {
        let google = ProviderConfig::new("google-id", "google-secret", "google-uri");
        let github = ProviderConfig::new("github-id", "github-secret", "github-uri");

        let config = SocialiteConfig::new()
            .with_google(google)
            .with_github(github);

        assert!(config.google.is_some());
        assert!(config.github.is_some());
        assert!(config.facebook.is_none());
        assert!(config.twitter.is_none());
    }

    #[test]
    fn test_config_get_provider() {
        let google = ProviderConfig::new("id", "secret", "uri");
        let config = SocialiteConfig::new().with_google(google);

        assert!(config.get_provider("google").is_some());
        assert!(config.get_provider("GOOGLE").is_some()); // Case insensitive
        assert!(config.get_provider("github").is_none());
    }

    #[test]
    fn test_config_all_providers() {
        let config = SocialiteConfig::new()
            .with_google(ProviderConfig::new("g-id", "g-secret", "g-uri"))
            .with_github(ProviderConfig::new("gh-id", "gh-secret", "gh-uri"))
            .with_facebook(ProviderConfig::new("fb-id", "fb-secret", "fb-uri"))
            .with_twitter(ProviderConfig::new("tw-id", "tw-secret", "tw-uri"));

        assert!(config.google.is_some());
        assert!(config.github.is_some());
        assert!(config.facebook.is_some());
        assert!(config.twitter.is_some());
    }
}

mod account_linking_tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_social_account_creation() {
        let account = SocialAccount::new(1, "github", "user123", "token-abc");
        assert_eq!(account.user_id, 1);
        assert_eq!(account.provider, "github");
        assert_eq!(account.provider_user_id, "user123");
        assert_eq!(account.access_token, "token-abc");
        assert!(account.refresh_token.is_none());
        assert!(account.expires_at.is_none());
    }

    #[test]
    fn test_social_account_with_refresh_token() {
        let account = SocialAccount::new(1, "google", "user456", "access-token")
            .with_refresh_token("refresh-token");
        assert_eq!(account.refresh_token, Some("refresh-token".to_string()));
    }

    #[test]
    fn test_social_account_expiration() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let account = SocialAccount::new(1, "github", "123", "token").with_expires_at(past);
        assert!(account.is_expired());
    }

    #[test]
    fn test_social_account_not_expired() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let account = SocialAccount::new(1, "github", "123", "token").with_expires_at(future);
        assert!(!account.is_expired());
    }

    #[test]
    fn test_social_account_needs_refresh() {
        let soon = Utc::now() + chrono::Duration::minutes(3);
        let account = SocialAccount::new(1, "google", "123", "token").with_expires_at(soon);
        assert!(account.needs_refresh());
    }

    #[test]
    fn test_social_account_no_refresh_needed() {
        let later = Utc::now() + chrono::Duration::hours(1);
        let account = SocialAccount::new(1, "google", "123", "token").with_expires_at(later);
        assert!(!account.needs_refresh());
    }

    #[test]
    fn test_linking_strategy_auto_link() {
        let linker = AccountLinker::new(LinkingStrategy::AutoLinkByEmail);
        assert!(linker.should_auto_link());
        assert!(!linker.should_create_new());
        assert!(!linker.should_ask_user());
    }

    #[test]
    fn test_linking_strategy_create_new() {
        let linker = AccountLinker::new(LinkingStrategy::AlwaysCreateNew);
        assert!(!linker.should_auto_link());
        assert!(linker.should_create_new());
        assert!(!linker.should_ask_user());
    }

    #[test]
    fn test_linking_strategy_ask_user() {
        let linker = AccountLinker::new(LinkingStrategy::AskUser);
        assert!(!linker.should_auto_link());
        assert!(!linker.should_create_new());
        assert!(linker.should_ask_user());
    }

    #[test]
    fn test_default_linking_strategy() {
        let linker = AccountLinker::default();
        assert_eq!(linker.strategy(), LinkingStrategy::AutoLinkByEmail);
    }
}

mod provider_tests {
    use super::*;
    use rf_socialite::providers::*;

    #[test]
    fn test_github_provider() {
        assert_eq!(
            GitHubProvider::authorize_url(),
            "https://github.com/login/oauth/authorize"
        );
        assert_eq!(
            GitHubProvider::token_url(),
            "https://github.com/login/oauth/access_token"
        );
        assert_eq!(GitHubProvider::user_url(), "https://api.github.com/user");
        assert!(!GitHubProvider::default_scopes().is_empty());
    }

    #[test]
    fn test_google_provider() {
        assert_eq!(
            GoogleProvider::authorize_url(),
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(
            GoogleProvider::token_url(),
            "https://oauth2.googleapis.com/token"
        );
        assert_eq!(
            GoogleProvider::user_url(),
            "https://www.googleapis.com/oauth2/v2/userinfo"
        );
        assert!(GoogleProvider::default_scopes().len() >= 2);
    }

    #[test]
    fn test_provider_enum_names() {
        assert_eq!(Provider::GitHub.name(), "github");
        assert_eq!(Provider::Google.name(), "google");
        assert_eq!(Provider::Facebook.name(), "facebook");
        assert_eq!(Provider::Twitter.name(), "twitter");
    }

    #[test]
    fn test_provider_enum_urls() {
        assert!(Provider::GitHub.authorize_url().contains("github.com"));
        assert!(Provider::Google.authorize_url().contains("google.com"));
        assert!(Provider::Facebook.authorize_url().contains("facebook.com"));
    }

    #[test]
    fn test_provider_default_scopes() {
        let github_scopes = Provider::GitHub.default_scopes();
        assert!(!github_scopes.is_empty());

        let google_scopes = Provider::Google.default_scopes();
        assert!(google_scopes.len() >= 2);
    }
}

mod driver_tests {
    use super::*;

    #[test]
    fn test_driver_builder() {
        let result = Socialite::driver(Provider::GitHub)
            .client_id("test-id")
            .client_secret("test-secret")
            .redirect_url("http://localhost/callback")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_driver_missing_client_id() {
        let result = Socialite::driver(Provider::GitHub)
            .client_secret("secret")
            .redirect_url("http://localhost/callback")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_driver_missing_client_secret() {
        let result = Socialite::driver(Provider::GitHub)
            .client_id("id")
            .redirect_url("http://localhost/callback")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_driver_missing_redirect_url() {
        let result = Socialite::driver(Provider::GitHub)
            .client_id("id")
            .client_secret("secret")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_driver_custom_scopes() {
        let mut driver = Socialite::driver(Provider::GitHub)
            .client_id("id")
            .client_secret("secret")
            .redirect_url("http://localhost/callback")
            .scope("user:email")
            .scope("repo")
            .build()
            .unwrap();

        let url = driver.redirect();
        assert!(url.is_ok());
    }

    #[test]
    fn test_driver_with_state() {
        let mut driver = Socialite::driver(Provider::GitHub)
            .client_id("id")
            .client_secret("secret")
            .redirect_url("http://localhost/callback")
            .state("random-state-123")
            .build()
            .unwrap();

        let url = driver.redirect().unwrap();
        assert!(url.contains("state=random-state-123"));
    }

    #[test]
    fn test_driver_with_pkce() {
        let mut driver = Socialite::driver(Provider::GitHub)
            .client_id("id")
            .client_secret("secret")
            .redirect_url("http://localhost/callback")
            .with_pkce()
            .build()
            .unwrap();

        let url = driver.redirect().unwrap();
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn test_redirect_url_structure() {
        let mut driver = Socialite::driver(Provider::GitHub)
            .client_id("test-client")
            .client_secret("test-secret")
            .redirect_url("http://localhost:8000/callback")
            .build()
            .unwrap();

        let url = driver.redirect().unwrap();
        assert!(url.contains("client_id=test-client"));
        assert!(url.contains("redirect_uri=http"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope="));
    }
}

mod manager_tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let config = SocialiteConfig::new();
        let manager = SocialiteManager::new(config);
        assert!(manager.config().google.is_none());
    }

    #[test]
    fn test_manager_state_generation() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        let state1 = manager.generate_state();
        let state2 = manager.generate_state();

        assert!(!state1.is_empty());
        assert!(!state2.is_empty());
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_manager_state_verification() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        let state = manager.generate_state();

        assert!(manager.verify_state(&state));
        assert!(!manager.verify_state(&state)); // One-time use
    }

    #[test]
    fn test_manager_driver_without_config() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        let result = manager.driver("google");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_driver_with_config() {
        let config = SocialiteConfig::new().with_google(ProviderConfig::new("id", "secret", "uri"));
        let manager = SocialiteManager::new(config);

        let result = manager.driver("google");
        assert!(result.is_ok());
    }

    #[test]
    fn test_manager_convenience_methods() {
        let config = SocialiteConfig::new()
            .with_google(ProviderConfig::new("g-id", "g-secret", "g-uri"))
            .with_github(ProviderConfig::new("gh-id", "gh-secret", "gh-uri"));

        let manager = SocialiteManager::new(config);

        assert!(manager.google().is_ok());
        assert!(manager.github().is_ok());
        assert!(manager.facebook().is_err());
        assert!(manager.twitter().is_err());
    }
}

mod routes_tests {
    use super::*;
    use rf_socialite::routes::*;

    #[test]
    fn test_route_paths() {
        assert_eq!(RouteHelper::auth_path(), "/auth/{provider}");
        assert_eq!(RouteHelper::callback_path(), "/auth/{provider}/callback");
    }

    #[test]
    fn test_callback_url_generation() {
        let url = RouteHelper::callback_url("http://localhost:8000", "github");
        assert_eq!(url, "http://localhost:8000/auth/github/callback");
    }

    #[test]
    fn test_callback_url_with_trailing_slash() {
        let url = RouteHelper::callback_url("http://localhost:8000/", "google");
        assert_eq!(url, "http://localhost:8000/auth/google/callback");
    }

    #[test]
    fn test_redirect_to_provider() {
        let config = SocialiteConfig::new().with_github(ProviderConfig::new(
            "id",
            "secret",
            "http://localhost/callback",
        ));
        let manager = SocialiteManager::new(config);

        let result = redirect_to_provider(&manager, "github", false);
        assert!(result.is_ok());

        let url = result.unwrap();
        assert!(url.contains("github.com"));
        assert!(url.contains("client_id="));
    }

    #[test]
    fn test_redirect_with_pkce() {
        let config = SocialiteConfig::new().with_github(ProviderConfig::new(
            "id",
            "secret",
            "http://localhost/callback",
        ));
        let manager = SocialiteManager::new(config);

        let result = redirect_to_provider(&manager, "github", true);
        assert!(result.is_ok());

        let url = result.unwrap();
        assert!(url.contains("code_challenge="));
    }
}
