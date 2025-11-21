//! Database models for Sanctum

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

/// Personal Access Token Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "personal_access_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub tokenable_type: String,
    pub tokenable_id: i64,
    pub name: String,
    pub token: String, // SHA256 hash
    pub abilities: Json, // JSON array of abilities
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
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

    // Note: before_save override removed due to SeaORM API changes
    // Timestamps should be handled manually when updating models
}

impl Model {
    /// Check if token has a specific ability
    pub fn can(&self, ability: &str) -> bool {
        if let Some(abilities) = self.abilities.as_array() {
            for a in abilities {
                if let Some(ability_str) = a.as_str() {
                    if ability_str == "*" || ability_str == ability {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if token has any of the abilities
    pub fn can_any(&self, abilities: &[&str]) -> bool {
        abilities.iter().any(|&ability| self.can(ability))
    }

    /// Check if token has all abilities
    pub fn can_all(&self, abilities: &[&str]) -> bool {
        abilities.iter().all(|&ability| self.can(ability))
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Update last_used_at timestamp
    pub async fn touch<C: ConnectionTrait>(&self, db: &C) -> Result<(), DbErr> {
        let mut active: ActiveModel = self.clone().into();
        active.last_used_at = Set(Some(Utc::now()));
        active.update(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_can_ability() {
        let token = Model {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: json!(["read:posts", "write:posts"]),
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(token.can("read:posts"));
        assert!(token.can("write:posts"));
        assert!(!token.can("delete:posts"));
    }

    #[test]
    fn test_wildcard_ability() {
        let token = Model {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: json!(["*"]),
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
