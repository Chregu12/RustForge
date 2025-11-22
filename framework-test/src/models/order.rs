// Order model - demonstrates BelongsTo and BelongsToMany relationships
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i32,
    pub user_id: i32,
    pub order_number: String,
    pub status: String,
    pub total_amount: Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// BelongsTo: Get the user who placed this order
    pub async fn user(&self, db: &crate::AppState) -> Result<Option<super::User>> {
        // REAL IMPLEMENTATION: rf_eloquent::belongs_to::<user::Entity, user::Model, _>(db, self.user_id, user::Column::Id).await
        Ok(Some(super::User::factory(self.user_id, "Customer", "customer@example.com")))
    }

    /// BelongsToMany: Get all products in this order
    ///
    /// This demonstrates a many-to-many relationship through order_items pivot table
    pub async fn products(&self, db: &crate::AppState) -> Result<Vec<super::Product>> {
        // REAL IMPLEMENTATION: rf_eloquent::belongs_to_many through order_items
        Ok(vec![])
    }

    /// Factory method
    pub fn factory(id: i32, user_id: i32) -> Self {
        Self {
            id,
            user_id,
            order_number: format!("ORD-{:06}", id),
            status: "pending".to_string(),
            total_amount: Decimal::new(9999, 2), // $99.99
            currency: "USD".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Update order status
    pub fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
        self.updated_at = Utc::now();
    }

    /// Check if order is pending
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }

    /// Check if order is completed
    pub fn is_completed(&self) -> bool {
        self.status == "completed"
    }

    /// Cancel order
    pub fn cancel(&mut self) {
        self.status = "cancelled".to_string();
        self.updated_at = Utc::now();
    }
}
