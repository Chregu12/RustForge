//! OAuth2 provider configurations

mod github;
mod google;
mod facebook;
mod twitter;
mod generic;

pub use github::GitHubProvider;
pub use google::GoogleProvider;
pub use facebook::FacebookProvider;
pub use twitter::TwitterProvider;
pub use generic::GenericProvider;

/// OAuth2 provider
#[derive(Debug, Clone)]
pub enum Provider {
    GitHub,
    Google,
    Facebook,
    Twitter,
    Generic(GenericProvider),
}

impl Provider {
    /// Get provider name
    pub fn name(&self) -> &str {
        match self {
            Provider::GitHub => "github",
            Provider::Google => "google",
            Provider::Facebook => "facebook",
            Provider::Twitter => "twitter",
            Provider::Generic(p) => &p.name,
        }
    }

    /// Get authorization URL
    pub fn authorize_url(&self) -> String {
        match self {
            Provider::GitHub => GitHubProvider::authorize_url(),
            Provider::Google => GoogleProvider::authorize_url(),
            Provider::Facebook => FacebookProvider::authorize_url(),
            Provider::Twitter => TwitterProvider::authorize_url(),
            Provider::Generic(p) => p.authorize_url.clone(),
        }
    }

    /// Get token URL
    pub fn token_url(&self) -> String {
        match self {
            Provider::GitHub => GitHubProvider::token_url(),
            Provider::Google => GoogleProvider::token_url(),
            Provider::Facebook => FacebookProvider::token_url(),
            Provider::Twitter => TwitterProvider::token_url(),
            Provider::Generic(p) => p.token_url.clone(),
        }
    }

    /// Get user info URL
    pub fn user_url(&self) -> String {
        match self {
            Provider::GitHub => GitHubProvider::user_url(),
            Provider::Google => GoogleProvider::user_url(),
            Provider::Facebook => FacebookProvider::user_url(),
            Provider::Twitter => TwitterProvider::user_url(),
            Provider::Generic(p) => p.user_url.clone(),
        }
    }

    /// Get default scopes
    pub fn default_scopes(&self) -> Vec<String> {
        match self {
            Provider::GitHub => GitHubProvider::default_scopes(),
            Provider::Google => GoogleProvider::default_scopes(),
            Provider::Facebook => FacebookProvider::default_scopes(),
            Provider::Twitter => TwitterProvider::default_scopes(),
            Provider::Generic(p) => p.scopes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        assert_eq!(Provider::GitHub.name(), "github");
        assert_eq!(Provider::Google.name(), "google");
        assert_eq!(Provider::Facebook.name(), "facebook");
        assert_eq!(Provider::Twitter.name(), "twitter");
    }

    #[test]
    fn test_provider_urls() {
        assert!(Provider::GitHub.authorize_url().contains("github.com"));
        assert!(Provider::Google.authorize_url().contains("google.com"));
    }
}
