/*!
 * Post Model
 *
 * Demonstrates:
 * - BelongsTo (user, category)
 * - HasMany (comments)
 * - MorphMany (images)
 * - MorphToMany (tags via taggables pivot)
 * - Soft Deletes
 * - Query Scopes
 * - Attribute Casting
 */

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub category_id: Option<i32>,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub featured: bool,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Post {
    // Relationships

    /// BelongsTo: Get the user who created this post
    pub async fn user(&self) -> Option<super::User> {
        // Implementation: User::find(self.user_id)
        None
    }

    /// BelongsTo: Get the category of this post
    pub async fn category(&self) -> Option<super::Category> {
        // Implementation: Category::find(self.category_id?)
        None
    }

    /// HasMany: Get all comments on this post
    pub async fn comments(&self) -> Vec<super::Comment> {
        // Implementation: Comment::where("commentable_type", "Post")
        //                         .where("commentable_id", self.id).get()
        vec![]
    }

    /// MorphMany: Get all images for this post
    pub async fn images(&self) -> Vec<super::Image> {
        // Implementation: Image::where("imageable_type", "Post")
        //                       .where("imageable_id", self.id).get()
        vec![]
    }

    /// MorphToMany: Get all tags for this post
    pub async fn tags(&self) -> Vec<super::Tag> {
        // Implementation: Tag::whereHas("taggables", |q| {
        //     q.where("taggable_type", "Post").where("taggable_id", self.id)
        // }).get()
        vec![]
    }

    // Scopes

    /// Query scope: Only published posts
    pub fn published() -> String {
        "published_at IS NOT NULL AND published_at <= datetime('now')".to_string()
    }

    /// Query scope: Only featured posts
    pub fn featured() -> String {
        "featured = 1".to_string()
    }

    /// Query scope: Recent posts (last 30 days)
    pub fn recent() -> String {
        "created_at >= datetime('now', '-30 days')".to_string()
    }

    /// Query scope: Popular posts (view_count > 1000)
    pub fn popular() -> String {
        "view_count > 1000".to_string()
    }

    // Methods

    /// Check if post is published
    pub fn is_published(&self) -> bool {
        if let Some(published_at) = self.published_at {
            published_at <= Utc::now()
        } else {
            false
        }
    }

    /// Increment view count
    pub async fn increment_views(&mut self) {
        self.view_count += 1;
        // Save to database
    }
}
