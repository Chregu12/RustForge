//! Facebook OAuth provider

pub struct FacebookProvider;

impl FacebookProvider {
    pub fn authorize_url() -> String {
        "https://www.facebook.com/v18.0/dialog/oauth".to_string()
    }

    pub fn token_url() -> String {
        "https://graph.facebook.com/v18.0/oauth/access_token".to_string()
    }

    pub fn user_url() -> String {
        "https://graph.facebook.com/me?fields=id,name,email,picture".to_string()
    }

    pub fn default_scopes() -> Vec<String> {
        vec!["email".to_string(), "public_profile".to_string()]
    }
}
