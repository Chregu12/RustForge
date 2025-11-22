// Category model - demonstrates self-referential relationships (parent/children)
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Category {
    /// HasMany: Get all posts in this category
    pub async fn posts(&self, db: &crate::AppState) -> Result<Vec<super::Post>> {
        // REAL IMPLEMENTATION: rf_eloquent::has_many::<post::Entity, post::Model, _>(db, self.id, post::Column::CategoryId).await
        Ok(vec![])
    }

    /// BelongsTo: Get parent category (self-referential)
    pub async fn parent(&self, db: &crate::AppState) -> Result<Option<Category>> {
        if let Some(parent_id) = self.parent_id {
            // REAL IMPLEMENTATION: rf_eloquent::belongs_to with self-reference
            Ok(Some(Category::factory(parent_id, "Parent Category")))
        } else {
            Ok(None)
        }
    }

    /// HasMany: Get child categories (self-referential)
    pub async fn children(&self, db: &crate::AppState) -> Result<Vec<Category>> {
        // REAL IMPLEMENTATION: rf_eloquent::has_many with self-reference
        Ok(vec![])
    }

    /// Factory method
    pub fn factory(id: i32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            slug: name.to_lowercase().replace(" ", "-"),
            description: Some(format!("Description for {}", name)),
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Check if category is a parent (has no parent)
    pub fn is_parent(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if category is a child (has a parent)
    pub fn is_child(&self) -> bool {
        self.parent_id.is_some()
    }
}
