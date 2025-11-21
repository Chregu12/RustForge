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
    pub async fn user(&self) -> Option<super::User> {
        // Implementation: User::find(self.user_id)
        None
    }

    /// MorphTo: Get the commentable entity (Post or Product)
    pub async fn commentable(&self) -> Option<Commentable> {
        match self.commentable_type.as_str() {
            "Post" => {
                // Post::find(self.commentable_id).map(Commentable::Post)
                None
            }
            "Product" => {
                // Product::find(self.commentable_id).map(Commentable::Product)
                None
            }
            _ => None,
        }
    }

    /// Query scope: Only approved comments
    pub fn approved() -> String {
        "approved = 1".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Commentable {
    Post(super::Post),
    Product(super::Product),
}
