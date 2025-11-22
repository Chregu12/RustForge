//! Personal Access Token model

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Personal Access Token stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: i64,
    pub tokenable_type: String, // "User", "App", etc.
    pub tokenable_id: i64,
    pub name: String,
    pub token: String, // SHA256 hash
    pub abilities: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New token with plaintext value (only returned once)
#[derive(Debug, Clone)]
pub struct NewToken {
    pub access_token: String, // Plaintext token (show once)
    pub token: PersonalAccessToken,
}

impl PersonalAccessToken {
    /// Generate a random token
    pub fn generate_token() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..80)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Hash a token using SHA256
    pub fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Check if token has an ability
    pub fn can(&self, ability: &str) -> bool {
        self.abilities.contains(&ability.to_string()) || self.abilities.contains(&"*".to_string())
    }

    /// Check if token has any of the abilities
    pub fn can_any(&self, abilities: &[&str]) -> bool {
        abilities.iter().any(|&ability| self.can(ability))
    }

    /// Check if token has all abilities
    pub fn can_all(&self, abilities: &[&str]) -> bool {
        abilities.iter().all(|&ability| self.can(ability))
    }

    /// Create from database model
    pub fn from_model(model: crate::models::Model) -> Self {
        let abilities = if let Some(arr) = model.abilities.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };

        Self {
            id: model.id,
            tokenable_type: model.tokenable_type,
            tokenable_id: model.tokenable_id,
            name: model.name,
            token: model.token,
            abilities,
            last_used_at: model.last_used_at,
            expires_at: model.expires_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = PersonalAccessToken::generate_token();
        assert_eq!(token.len(), 80);
    }

    #[test]
    fn test_hash_token() {
        let token = "my-secret-token";
        let hash = PersonalAccessToken::hash_token(token);
        assert_eq!(hash.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_token_abilities() {
        let token = PersonalAccessToken {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: vec!["read:posts".to_string(), "write:posts".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(token.can("read:posts"));
        assert!(token.can("write:posts"));
        assert!(!token.can("delete:posts"));
        assert!(token.can_any(&["read:posts", "delete:posts"]));
        assert!(!token.can_all(&["read:posts", "delete:posts"]));
    }

    #[test]
    fn test_wildcard_ability() {
        let token = PersonalAccessToken {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: vec!["*".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(token.can("anything"));
        assert!(token.can("read:posts"));
        assert!(token.can("delete:everything"));
    }
}
