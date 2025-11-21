/*!
 * Product Model
 *
 * Demonstrates:
 * - BelongsToMany (orders via order_items with pivot data)
 * - MorphOne (featured_image)
 * - MorphMany (images)
 * - MorphToMany (tags via taggables)
 * - Soft Deletes
 */

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub sku: String,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Product {
    /// BelongsToMany: Get orders that include this product
    pub async fn orders(&self) -> Vec<super::Order> {
        vec![]
    }

    /// MorphOne: Get the featured image
    pub async fn featured_image(&self) -> Option<super::Image> {
        // Image::where("imageable_type", "Product")
        //       .where("imageable_id", self.id)
        //       .where("is_featured", true)
        //       .first()
        None
    }

    /// MorphMany: Get all images
    pub async fn images(&self) -> Vec<super::Image> {
        vec![]
    }

    /// MorphToMany: Get all tags
    pub async fn tags(&self) -> Vec<super::Tag> {
        vec![]
    }

    /// Query scope: Only active products
    pub fn active() -> String {
        "is_active = 1 AND deleted_at IS NULL".to_string()
    }

    /// Query scope: In stock products
    pub fn in_stock() -> String {
        "stock_quantity > 0".to_string()
    }
}
