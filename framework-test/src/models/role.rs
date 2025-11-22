// Role model - demonstrates BelongsToMany inverse relationship
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Role {
    /// BelongsToMany: Get all users with this role
    pub async fn users(&self, db: &crate::AppState) -> Result<Vec<super::User>> {
        // REAL IMPLEMENTATION: rf_eloquent::belongs_to_many through role_user pivot (inverse)
        Ok(vec![])
    }

    /// HasMany: Get all permissions for this role
    pub async fn permissions(&self, db: &crate::AppState) -> Result<Vec<super::Permission>> {
        // REAL IMPLEMENTATION: rf_eloquent::has_many or belongs_to_many depending on design
        Ok(vec![])
    }

    /// Factory method
    pub fn factory(id: i32, name: &str, display_name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: Some(format!("Role: {}", display_name)),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Check if this role has a specific permission
    pub async fn has_permission(&self, db: &crate::AppState, permission: &str) -> Result<bool> {
        let permissions = self.permissions(db).await?;
        Ok(permissions.iter().any(|p| p.name == permission))
    }
}
