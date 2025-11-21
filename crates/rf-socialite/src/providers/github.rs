//! GitHub OAuth provider

pub struct GitHubProvider;

impl GitHubProvider {
    pub fn authorize_url() -> String {
        "https://github.com/login/oauth/authorize".to_string()
    }

    pub fn token_url() -> String {
        "https://github.com/login/oauth/access_token".to_string()
    }

    pub fn user_url() -> String {
        "https://api.github.com/user".to_string()
    }

    pub fn default_scopes() -> Vec<String> {
        vec!["user:email".to_string()]
    }
}
