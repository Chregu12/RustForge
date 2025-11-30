//! OAuth Authorization Code model

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

/// OAuth Authorization Code Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth_auth_codes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User ID
    pub user_id: i64,

    /// Client ID
    pub client_id: i64,

    /// Scopes (JSON array)
    pub scopes: Json,

    /// Is this code revoked?
    pub revoked: bool,

    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// PKCE code challenge
    pub code_challenge: Option<String>,

    /// PKCE code challenge method (plain or S256)
    pub code_challenge_method: Option<String>,

    /// Redirect URI used in the authorization request
    pub redirect_uri: String,

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
    /// Generate a random authorization code
    pub fn generate_code() -> String {
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

    /// Check if code is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Check if code is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.revoked
    }

    /// Get scopes as Vec<String>
    pub fn get_scopes(&self) -> Vec<String> {
        if let Some(arr) = self.scopes.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_code() {
        let code = Model::generate_code();
        assert_eq!(code.len(), 80);
    }

    #[test]
    fn test_code_validity() {
        let valid_code = Model {
            id: "test".to_string(),
            user_id: 1,
            client_id: 1,
            scopes: json!([]),
            revoked: false,
            expires_at: Utc::now() + chrono::Duration::minutes(10),
            code_challenge: None,
            code_challenge_method: None,
            redirect_uri: "http://localhost:3000/callback".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(valid_code.is_valid());
        assert!(!valid_code.is_expired());

        let expired_code = Model {
            expires_at: Utc::now() - chrono::Duration::minutes(1),
            ..valid_code.clone()
        };

        assert!(!expired_code.is_valid());
        assert!(expired_code.is_expired());

        let revoked_code = Model {
            revoked: true,
            ..valid_code.clone()
        };

        assert!(!revoked_code.is_valid());
    }
}
