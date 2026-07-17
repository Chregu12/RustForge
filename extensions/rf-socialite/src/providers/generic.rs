//! Generic OAuth2 provider

#[derive(Debug, Clone)]
pub struct GenericProvider {
    pub name: String,
    pub authorize_url: String,
    pub token_url: String,
    pub user_url: String,
    pub scopes: Vec<String>,
}

impl GenericProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            authorize_url: String::new(),
            token_url: String::new(),
            user_url: String::new(),
            scopes: Vec::new(),
        }
    }

    pub fn authorize_url(mut self, url: impl Into<String>) -> Self {
        self.authorize_url = url.into();
        self
    }

    pub fn token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }

    pub fn user_url(mut self, url: impl Into<String>) -> Self {
        self.user_url = url.into();
        self
    }

    pub fn scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_provider() {
        let provider = GenericProvider::new("custom")
            .authorize_url("https://example.com/oauth/authorize")
            .token_url("https://example.com/oauth/token")
            .user_url("https://example.com/api/user")
            .scopes(vec!["read".to_string(), "write".to_string()]);

        assert_eq!(provider.name, "custom");
        assert_eq!(provider.scopes.len(), 2);
    }
}
