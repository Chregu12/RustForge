use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Product Model
///
/// Represents a product in the catalog.
/// Contains product details, pricing, and inventory information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Unique product identifier
    pub id: i64,
    /// Product name
    pub name: String,
    /// Product description
    pub description: Option<String>,
    /// Product price
    pub price: f64,
    /// Stock quantity
    pub stock: i32,
    /// Whether the product is active
    pub is_active: bool,
    /// Timestamp when the product was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the product was last updated
    pub updated_at: DateTime<Utc>,
}

impl Product {
    /// Creates a new product instance
    pub fn new(id: i64, name: String, price: f64, stock: i32) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            price,
            stock,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the product description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.updated_at = Utc::now();
        self
    }

    /// Decreases stock by the given amount
    pub fn decrease_stock(&mut self, amount: i32) -> Result<(), String> {
        if self.stock < amount {
            return Err("Insufficient stock".to_string());
        }
        self.stock -= amount;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Increases stock by the given amount
    pub fn increase_stock(&mut self, amount: i32) {
        self.stock += amount;
        self.updated_at = Utc::now();
    }

    /// Deactivates the product
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_new_product() {
        let product = Product::new(1, "Test Product".to_string(), 99.99, 10);

        assert_eq!(product.id, 1);
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.price, 99.99);
        assert_eq!(product.stock, 10);
        assert!(product.is_active);
    }

    #[test]
    fn can_set_description() {
        let product = Product::new(1, "Test Product".to_string(), 99.99, 10)
            .with_description("A test product".to_string());

        assert_eq!(product.description, Some("A test product".to_string()));
    }

    #[test]
    fn can_decrease_stock() {
        let mut product = Product::new(1, "Test Product".to_string(), 99.99, 10);
        let result = product.decrease_stock(3);

        assert!(result.is_ok());
        assert_eq!(product.stock, 7);
    }

    #[test]
    fn cannot_decrease_stock_below_zero() {
        let mut product = Product::new(1, "Test Product".to_string(), 99.99, 5);
        let result = product.decrease_stock(10);

        assert!(result.is_err());
        assert_eq!(product.stock, 5); // Should remain unchanged
    }

    #[test]
    fn can_increase_stock() {
        let mut product = Product::new(1, "Test Product".to_string(), 99.99, 5);
        product.increase_stock(5);

        assert_eq!(product.stock, 10);
    }
}
