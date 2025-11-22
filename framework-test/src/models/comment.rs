/*!
 * Comment Model
 *
 * Demonstrates:
 * - BelongsTo (user)
 * - MorphTo (commentable - can belong to Post or Product)
 * - Soft Deletes
 */

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub commentable_type: String,
    pub commentable_id: i32,
    pub content: String,
    pub approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Comment {
    /// BelongsTo: Get the user who created this comment
    pub async fn user(&self, db: &crate::AppState) -> anyhow::Result<Option<super::User>> {
        // REAL IMPLEMENTATION: rf_eloquent::belongs_to::<user::Entity, user::Model, _>(db, self.user_id, user::Column::Id).await
        Ok(Some(super::User::factory(self.user_id, "Commenter", "commenter@example.com")))
    }

    /// MorphTo: Get the commentable entity (Post or Product)
    ///
    /// This demonstrates a polymorphic "belongs to" relationship
    pub async fn commentable(&self, db: &crate::AppState) -> anyhow::Result<Option<Commentable>> {
        match self.commentable_type.as_str() {
            "Post" => {
                // REAL IMPLEMENTATION: rf_eloquent::morph_to::<post::Entity, post::Model>(db, self.commentable_id).await
                Ok(Some(Commentable::Post(super::Post::factory(
                    self.commentable_id,
                    1,
                    "Demo Post",
                ))))
            }
            "Product" => {
                // REAL IMPLEMENTATION: rf_eloquent::morph_to::<product::Entity, product::Model>(db, self.commentable_id).await
                Ok(Some(Commentable::Product(super::Product::factory(
                    self.commentable_id,
                    "Demo Product",
                ))))
            }
            _ => Ok(None),
        }
    }

    /// Query scope: Only approved comments
    pub fn approved() -> String {
        "approved = 1".to_string()
    }

    /// Factory method
    pub fn factory(id: i32, user_id: i32, commentable_type: &str, commentable_id: i32, content: &str) -> Self {
        Self {
            id,
            user_id,
            commentable_type: commentable_type.to_string(),
            commentable_id,
            content: content.to_string(),
            approved: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
        }
    }

    /// Approve comment
    pub fn approve(&mut self) {
        self.approved = true;
        self.updated_at = chrono::Utc::now();
    }

    /// Reject/unapprove comment
    pub fn reject(&mut self) {
        self.approved = false;
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Commentable {
    Post(super::Post),
    Product(super::Product),
}
