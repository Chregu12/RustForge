//! OAuth Refresh Token model

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

/// OAuth Refresh Token Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth_refresh_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Access token ID
    pub access_token_id: String,

    /// Is this token revoked?
    pub revoked: bool,

    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,

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
    /// Generate a random token ID
    pub fn generate_token_id() -> String {
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

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Check if token is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.revoked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_id() {
        let id = Model::generate_token_id();
        assert_eq!(id.len(), 80);
    }

    #[test]
    fn test_token_validity() {
        let valid_token = Model {
            id: "test".to_string(),
            access_token_id: "access_test".to_string(),
            revoked: false,
            expires_at: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(valid_token.is_valid());
        assert!(!valid_token.is_expired());

        let expired_token = Model {
            expires_at: Utc::now() - chrono::Duration::days(1),
            ..valid_token.clone()
        };

        assert!(!expired_token.is_valid());
        assert!(expired_token.is_expired());

        let revoked_token = Model {
            revoked: true,
            ..valid_token.clone()
        };

        assert!(!revoked_token.is_valid());
    }
}
