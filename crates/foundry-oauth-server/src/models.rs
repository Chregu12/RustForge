//! OAuth2 Data Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// OAuth2 Client (Application)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    pub secret: Option<String>, // None for public clients (PKCE)
    pub redirect_uris: Vec<String>,
    pub grants: Vec<String>, // authorization_code, client_credentials, password, refresh_token
    pub scopes: Vec<String>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Client {
    pub fn new(name: String, redirect_uris: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            secret: Some(Self::generate_secret()),
            redirect_uris,
            grants: vec!["authorization_code".to_string(), "refresh_token".to_string()],
            scopes: vec!["*".to_string()],
            revoked: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn public(name: String, redirect_uris: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            secret: None, // Public client (PKCE required)
            redirect_uris,
            grants: vec!["authorization_code".to_string(), "refresh_token".to_string()],
            scopes: vec!["*".to_string()],
            revoked: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn generate_secret() -> String {
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(40)
            .collect();
        STANDARD.encode(&random_bytes)
    }

    pub fn is_confidential(&self) -> bool {
        self.secret.is_some()
    }

    pub fn supports_grant(&self, grant_type: &str) -> bool {
        self.grants.contains(&grant_type.to_string())
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&"*".to_string()) || self.scopes.contains(&scope.to_string())
    }
}

/// Access Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub id: Uuid,
    pub client_id: Uuid,
    pub user_id: Option<Uuid>,
    pub token: String,
    pub scopes: Vec<String>,
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Refresh Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub access_token_id: Uuid,
    pub token: String,
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Authorization Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCode {
    pub id: Uuid,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>, // plain, S256
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Personal Access Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token: String,
    pub scopes: Vec<String>,
    pub revoked: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl PersonalAccessToken {
    pub fn new(user_id: Uuid, name: String, scopes: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            name,
            token: Self::generate_token(),
            scopes,
            revoked: false,
            last_used_at: None,
            expires_at: None, // Never expires by default
            created_at: Utc::now(),
        }
    }

    fn generate_token() -> String {
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(64)
            .collect();
        STANDARD.encode(&random_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidential_client() {
        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        assert!(client.is_confidential());
        assert!(client.secret.is_some());
        assert!(client.supports_grant("authorization_code"));
    }

    #[test]
    fn test_public_client() {
        let client = Client::public(
            "Public App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        assert!(!client.is_confidential());
        assert!(client.secret.is_none());
    }

    #[test]
    fn test_personal_access_token() {
        let user_id = Uuid::new_v4();
        let token = PersonalAccessToken::new(
            user_id,
            "My API Token".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );
        assert_eq!(token.user_id, user_id);
        assert!(!token.revoked);
        assert!(token.expires_at.is_none());
    }
}
