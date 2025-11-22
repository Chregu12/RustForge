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
    ///
    /// This demonstrates a many-to-many relationship through order_items pivot table
    pub async fn orders(&self, db: &crate::AppState) -> anyhow::Result<Vec<super::Order>> {
        // REAL IMPLEMENTATION: rf_eloquent::belongs_to_many through order_items pivot
        Ok(vec![])
    }

    /// MorphOne: Get the featured image
    ///
    /// This demonstrates a polymorphic one-to-one relationship
    pub async fn featured_image(&self, db: &crate::AppState) -> anyhow::Result<Option<super::Image>> {
        // REAL IMPLEMENTATION: rf_eloquent::morph_one with where clause for is_featured
        Ok(None)
    }

    /// MorphMany: Get all images
    pub async fn images(&self, db: &crate::AppState) -> anyhow::Result<Vec<super::Image>> {
        // REAL IMPLEMENTATION: rf_eloquent::morph_many::<image::Entity, image::Model>(db, self.id, "Product", "imageable").await
        Ok(vec![])
    }

    /// MorphToMany: Get all tags
    pub async fn tags(&self, db: &crate::AppState) -> anyhow::Result<Vec<super::Tag>> {
        // REAL IMPLEMENTATION: rf_eloquent::morph_to_many through taggables pivot
        Ok(vec![])
    }

    /// Query scope: Only active products
    pub fn active() -> String {
        "is_active = 1 AND deleted_at IS NULL".to_string()
    }

    /// Query scope: In stock products
    pub fn in_stock() -> String {
        "stock_quantity > 0".to_string()
    }

    /// Factory method
    pub fn factory(id: i32, name: &str) -> Self {
        use rust_decimal::Decimal;
        Self {
            id,
            name: name.to_string(),
            slug: name.to_lowercase().replace(" ", "-"),
            description: Some(format!("Description for {}", name)),
            price: Decimal::new(1999, 2), // $19.99
            sku: format!("SKU-{:04}", id),
            stock_quantity: 100,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
        }
    }

    /// Update stock quantity
    pub fn update_stock(&mut self, quantity: i32) {
        self.stock_quantity = quantity;
        self.updated_at = chrono::Utc::now();
    }

    /// Check if in stock
    pub fn is_in_stock(&self) -> bool {
        self.stock_quantity > 0
    }

    /// Activate/deactivate product
    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        self.updated_at = chrono::Utc::now();
    }
}
