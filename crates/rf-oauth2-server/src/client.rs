//! OAuth2 client management

use crate::types::{GrantType, Scope};
use serde::{Deserialize, Serialize};

/// OAuth2 client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    /// Client ID
    pub id: String,

    /// Client secret (optional for public clients)
    pub secret: Option<String>,

    /// Allowed redirect URIs
    pub redirect_uris: Vec<String>,

    /// Allowed grant types
    pub grants: Vec<GrantType>,

    /// Allowed scopes
    pub scopes: Vec<Scope>,
}

impl Client {
    /// Check if client supports a grant type
    pub fn supports_grant(&self, grant: &GrantType) -> bool {
        self.grants.contains(grant)
    }

    /// Check if redirect URI is valid
    pub fn is_redirect_uri_valid(&self, uri: &str) -> bool {
        self.redirect_uris.iter().any(|u| u == uri)
    }

    /// Check if scope is valid
    pub fn is_scope_valid(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Verify client secret
    pub fn verify_secret(&self, secret: &str) -> bool {
        match &self.secret {
            Some(s) => s == secret,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_supports_grant() {
        let client = Client {
            id: "test".to_string(),
            secret: Some("secret".to_string()),
            redirect_uris: vec![],
            grants: vec![GrantType::AuthorizationCode],
            scopes: vec![],
        };

        assert!(client.supports_grant(&GrantType::AuthorizationCode));
        assert!(!client.supports_grant(&GrantType::ClientCredentials));
    }

    #[test]
    fn test_redirect_uri_validation() {
        let client = Client {
            id: "test".to_string(),
            secret: None,
            redirect_uris: vec!["https://example.com/callback".to_string()],
            grants: vec![],
            scopes: vec![],
        };

        assert!(client.is_redirect_uri_valid("https://example.com/callback"));
        assert!(!client.is_redirect_uri_valid("https://evil.com/callback"));
    }

    #[test]
    fn test_secret_verification() {
        let client = Client {
            id: "test".to_string(),
            secret: Some("secret123".to_string()),
            redirect_uris: vec![],
            grants: vec![],
            scopes: vec![],
        };

        assert!(client.verify_secret("secret123"));
        assert!(!client.verify_secret("wrong"));
    }
}
