// Tag model - demonstrates MorphToMany polymorphic relationship
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tag {
    /// MorphToMany: Get all posts with this tag
    pub async fn posts(&self, db: &crate::AppState) -> Result<Vec<super::Post>> {
        // REAL IMPLEMENTATION: rf_eloquent::morph_to_many through taggables pivot
        Ok(vec![])
    }

    /// MorphToMany: Get all products with this tag
    pub async fn products(&self, db: &crate::AppState) -> Result<Vec<super::Product>> {
        // REAL IMPLEMENTATION: rf_eloquent::morph_to_many through taggables pivot
        Ok(vec![])
    }

    /// Factory method
    pub fn factory(id: i32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            slug: name.to_lowercase().replace(" ", "-"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
