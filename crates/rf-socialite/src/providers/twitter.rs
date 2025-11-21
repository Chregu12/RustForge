//! Twitter/X OAuth provider

pub struct TwitterProvider;

impl TwitterProvider {
    pub fn authorize_url() -> String {
        "https://twitter.com/i/oauth2/authorize".to_string()
    }

    pub fn token_url() -> String {
        "https://api.twitter.com/2/oauth2/token".to_string()
    }

    pub fn user_url() -> String {
        "https://api.twitter.com/2/users/me".to_string()
    }

    pub fn default_scopes() -> Vec<String> {
        vec!["tweet.read".to_string(), "users.read".to_string()]
    }
}
