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
    pub async fn imageable(&self) -> Option<Imageable> {
        match self.imageable_type.as_str() {
            "User" => None, // User::find(self.imageable_id).map(Imageable::User)
            "Post" => None, // Post::find(self.imageable_id).map(Imageable::Post)
            "Product" => None, // Product::find(self.imageable_id).map(Imageable::Product)
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Imageable {
    User(super::User),
    Post(super::Post),
    Product(super::Product),
}
