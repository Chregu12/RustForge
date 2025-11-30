//! OAuth Access Token model

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

/// OAuth Access Token Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth_access_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User ID (None for client credentials grant)
    pub user_id: Option<i64>,

    /// Client ID
    pub client_id: i64,

    /// Token name (for personal access tokens)
    pub name: Option<String>,

    /// Scopes (JSON array)
    pub scopes: Json,

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

    /// Check if token has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        if let Some(scopes) = self.scopes.as_array() {
            for s in scopes {
                if let Some(scope_str) = s.as_str() {
                    if scope_str == "*" || scope_str == scope {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if token has any of the scopes
    pub fn has_any_scope(&self, scopes: &[&str]) -> bool {
        scopes.iter().any(|&scope| self.has_scope(scope))
    }

    /// Check if token has all scopes
    pub fn has_all_scopes(&self, scopes: &[&str]) -> bool {
        scopes.iter().all(|&scope| self.has_scope(scope))
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

    /// Check if token is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.revoked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_token_id() {
        let id = Model::generate_token_id();
        assert_eq!(id.len(), 80);
    }

    #[test]
    fn test_token_scopes() {
        let token = Model {
            id: "test".to_string(),
            user_id: Some(1),
            client_id: 1,
            name: None,
            scopes: json!(["read:posts", "write:posts"]),
            revoked: false,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(token.has_scope("read:posts"));
        assert!(token.has_scope("write:posts"));
        assert!(!token.has_scope("delete:posts"));
        assert!(token.has_any_scope(&["read:posts", "delete:posts"]));
        assert!(!token.has_all_scopes(&["read:posts", "delete:posts"]));
    }

    #[test]
    fn test_wildcard_scope() {
        let token = Model {
            id: "test".to_string(),
            user_id: Some(1),
            client_id: 1,
            name: None,
            scopes: json!(["*"]),
            revoked: false,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(token.has_scope("anything"));
        assert!(token.has_scope("read:posts"));
        assert!(token.has_scope("delete:everything"));
    }

    #[test]
    fn test_token_validity() {
        let valid_token = Model {
            id: "test".to_string(),
            user_id: Some(1),
            client_id: 1,
            name: None,
            scopes: json!([]),
            revoked: false,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(valid_token.is_valid());
        assert!(!valid_token.is_expired());

        let expired_token = Model {
            expires_at: Utc::now() - chrono::Duration::hours(1),
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
