//! Google OAuth provider

pub struct GoogleProvider;

impl GoogleProvider {
    pub fn authorize_url() -> String {
        "https://accounts.google.com/o/oauth2/v2/auth".to_string()
    }

    pub fn token_url() -> String {
        "https://oauth2.googleapis.com/token".to_string()
    }

    pub fn user_url() -> String {
        "https://www.googleapis.com/oauth2/v2/userinfo".to_string()
    }

    pub fn default_scopes() -> Vec<String> {
        vec![
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
        ]
    }
}
