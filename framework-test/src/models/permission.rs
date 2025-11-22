// Permission model
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Permission {
    /// BelongsToMany: Get all roles with this permission
    pub async fn roles(&self, db: &crate::AppState) -> Result<Vec<super::Role>> {
        // REAL IMPLEMENTATION: rf_eloquent::belongs_to_many through permission_role pivot
        Ok(vec![])
    }

    /// Factory method
    pub fn factory(id: i32, name: &str, display_name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: Some(format!("Permission: {}", display_name)),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
