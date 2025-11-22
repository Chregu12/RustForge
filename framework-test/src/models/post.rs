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
    ///
    /// REAL IMPLEMENTATION pattern:
    /// ```rust,no_run
    /// rf_eloquent::belongs_to::<user::Entity, user::Model, _>(db, self.user_id, user::Column::Id).await
    /// ```
    pub async fn user(&self, db: &crate::AppState) -> anyhow::Result<Option<super::User>> {
        // In a real implementation, this would query the database
        Ok(Some(super::User::factory(
            self.user_id,
            "Demo User",
            "user@example.com",
        )))
    }

    /// BelongsTo: Get the category of this post
    pub async fn category(&self, db: &crate::AppState) -> anyhow::Result<Option<super::Category>> {
        if let Some(category_id) = self.category_id {
            Ok(Some(super::Category {
                id: category_id,
                name: "Demo Category".to_string(),
                slug: "demo-category".to_string(),
                description: Some("A demonstration category".to_string()),
                parent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    /// HasMany: Get all comments on this post
    ///
    /// This demonstrates a polymorphic relationship where comments can belong to posts or products
    pub async fn comments(&self, db: &crate::AppState) -> anyhow::Result<Vec<super::Comment>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::morph_many::<comment::Entity, comment::Model>(db, self.id, "Post", "commentable").await
        Ok(vec![])
    }

    /// MorphMany: Get all images for this post
    pub async fn images(&self, db: &crate::AppState) -> anyhow::Result<Vec<super::Image>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::morph_many::<image::Entity, image::Model>(db, self.id, "Post", "imageable").await
        Ok(vec![])
    }

    /// MorphToMany: Get all tags for this post
    ///
    /// This demonstrates a polymorphic many-to-many relationship
    pub async fn tags(&self, db: &crate::AppState) -> anyhow::Result<Vec<super::Tag>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::morph_to_many::<tag::Entity, taggable::Entity, tag::Model>(
        //     db, self.id, "Post", "taggable"
        // ).await
        Ok(vec![])
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
    pub async fn increment_views(&mut self, db: &crate::AppState) -> anyhow::Result<()> {
        self.view_count += 1;
        self.updated_at = Utc::now();
        // In real implementation: save to database
        Ok(())
    }

    /// Factory method for creating test posts
    pub fn factory(id: i32, user_id: i32, title: &str) -> Self {
        Self {
            id,
            user_id,
            category_id: Some(1),
            title: title.to_string(),
            slug: title.to_lowercase().replace(" ", "-"),
            content: format!("Content for: {}", title),
            excerpt: Some(format!("Excerpt for: {}", title)),
            published_at: Some(Utc::now()),
            featured: false,
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    /// Publish the post
    pub fn publish(&mut self) {
        self.published_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Unpublish the post
    pub fn unpublish(&mut self) {
        self.published_at = None;
        self.updated_at = Utc::now();
    }

    /// Mark as featured
    pub fn set_featured(&mut self, featured: bool) {
        self.featured = featured;
        self.updated_at = Utc::now();
    }

    /// Soft delete
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}
