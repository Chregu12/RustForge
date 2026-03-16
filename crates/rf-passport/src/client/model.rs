//! OAuth Client model

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// OAuth Client Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth_clients")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// User ID that owns this client (None for system clients)
    pub user_id: Option<i64>,

    /// Client name
    pub name: String,

    /// Client secret (hashed)
    #[serde(skip_serializing)]
    pub secret: Option<String>,

    /// Provider (used for custom auth providers)
    pub provider: Option<String>,

    /// Allowed redirect URIs (JSON array)
    pub redirect: Json,

    /// Is this a personal access client?
    pub personal_access_client: bool,

    /// Is this a password grant client?
    pub password_client: bool,

    /// Is this client revoked?
    pub revoked: bool,

    /// Created at timestamp
    pub created_at: DateTime<Utc>,

    /// Updated at timestamp
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..ActiveModelTrait::default()
        }
    }
}

impl Model {
    /// Generate a random client secret
    pub fn generate_secret() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Hash a client secret using SHA256
    pub fn hash_secret(secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify client secret (constant-time comparison)
    pub fn verify_secret(&self, secret: &str) -> bool {
        if let Some(stored_secret) = &self.secret {
            let hashed = Self::hash_secret(secret);
            let stored_bytes = stored_secret.as_bytes();
            let hashed_bytes = hashed.as_bytes();
            if stored_bytes.len() != hashed_bytes.len() {
                return false;
            }
            let mut result = 0u8;
            for (a, b) in stored_bytes.iter().zip(hashed_bytes.iter()) {
                result |= a ^ b;
            }
            result == 0
        } else {
            false
        }
    }

    /// Check if redirect URI is valid for this client
    pub fn is_redirect_uri_valid(&self, uri: &str) -> bool {
        if let Some(uris) = self.redirect.as_array() {
            for u in uris {
                if let Some(redirect_uri) = u.as_str() {
                    if redirect_uri == uri {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get redirect URIs as Vec<String>
    pub fn redirect_uris(&self) -> Vec<String> {
        if let Some(arr) = self.redirect.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if client is confidential (has a secret)
    pub fn is_confidential(&self) -> bool {
        self.secret.is_some()
    }

    /// Check if client is public (no secret)
    pub fn is_public(&self) -> bool {
        !self.is_confidential()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_secret() {
        let secret = Model::generate_secret();
        assert_eq!(secret.len(), 64);
    }

    #[test]
    fn test_hash_secret() {
        let secret = "my-secret";
        let hash = Model::hash_secret(secret);
        assert_eq!(hash.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_verify_secret() {
        let secret = "my-secret";
        let hashed = Model::hash_secret(secret);

        let client = Model {
            id: 1,
            user_id: Some(1),
            name: "Test Client".to_string(),
            secret: Some(hashed),
            provider: None,
            redirect: json!(["http://localhost:3000/callback"]),
            personal_access_client: false,
            password_client: false,
            revoked: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(client.verify_secret(secret));
        assert!(!client.verify_secret("wrong-secret"));
    }

    #[test]
    fn test_redirect_uri_validation() {
        let client = Model {
            id: 1,
            user_id: Some(1),
            name: "Test Client".to_string(),
            secret: None,
            provider: None,
            redirect: json!(["http://localhost:3000/callback", "http://example.com/auth"]),
            personal_access_client: false,
            password_client: false,
            revoked: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(client.is_redirect_uri_valid("http://localhost:3000/callback"));
        assert!(client.is_redirect_uri_valid("http://example.com/auth"));
        assert!(!client.is_redirect_uri_valid("http://evil.com/steal"));
    }

    #[test]
    fn test_client_types() {
        let confidential = Model {
            id: 1,
            user_id: Some(1),
            name: "Confidential".to_string(),
            secret: Some("hash".to_string()),
            provider: None,
            redirect: json!([]),
            personal_access_client: false,
            password_client: false,
            revoked: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(confidential.is_confidential());
        assert!(!confidential.is_public());

        let public = Model {
            id: 2,
            user_id: Some(1),
            name: "Public".to_string(),
            secret: None,
            provider: None,
            redirect: json!([]),
            personal_access_client: false,
            password_client: false,
            revoked: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!public.is_confidential());
        assert!(public.is_public());
    }
}
