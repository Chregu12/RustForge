/*!
 * Image Model
 *
 * Demonstrates:
 * - MorphTo (imageable - can belong to User, Post, or Product)
 * - File storage integration
 */

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: i32,
    pub imageable_type: String,
    pub imageable_id: i32,
    pub url: String,
    pub filename: String,
    pub mime_type: String,
    pub size: i32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub is_featured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Image {
    /// MorphTo: Get the imageable entity (User, Post, or Product)
    ///
    /// This demonstrates a polymorphic "belongs to" relationship
    pub async fn imageable(&self, db: &crate::AppState) -> anyhow::Result<Option<Imageable>> {
        match self.imageable_type.as_str() {
            "User" => {
                // REAL IMPLEMENTATION: rf_eloquent::morph_to::<user::Entity, user::Model>(db, self.imageable_id).await
                Ok(Some(Imageable::User(super::User::factory(
                    self.imageable_id,
                    "Demo User",
                    "user@example.com",
                ))))
            }
            "Post" => {
                // REAL IMPLEMENTATION: rf_eloquent::morph_to::<post::Entity, post::Model>(db, self.imageable_id).await
                Ok(Some(Imageable::Post(super::Post::factory(
                    self.imageable_id,
                    1,
                    "Demo Post",
                ))))
            }
            "Product" => {
                // REAL IMPLEMENTATION: rf_eloquent::morph_to::<product::Entity, product::Model>(db, self.imageable_id).await
                Ok(Some(Imageable::Product(super::Product::factory(
                    self.imageable_id,
                    "Demo Product",
                ))))
            }
            _ => Ok(None),
        }
    }

    /// Factory method
    pub fn factory(id: i32, imageable_type: &str, imageable_id: i32, url: &str) -> Self {
        Self {
            id,
            imageable_type: imageable_type.to_string(),
            imageable_id,
            url: url.to_string(),
            filename: "demo-image.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            size: 102400, // 100KB
            width: Some(1920),
            height: Some(1080),
            is_featured: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Mark as featured
    pub fn set_featured(&mut self, featured: bool) {
        self.is_featured = featured;
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Imageable {
    User(super::User),
    Post(super::Post),
    Product(super::Product),
}
