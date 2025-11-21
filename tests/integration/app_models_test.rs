//! Integration tests for application models
//!
//! Tests for the core business logic in domain models and request handlers.

use domain::models::account::Account;
use domain::models::product::Product;

mod account_tests {
    use super::*;

    #[test]
    fn account_creation() {
        let account = Account::new(
            1,
            "test@example.com".to_string(),
            "Test User".to_string(),
        );

        assert_eq!(account.id, 1);
        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.name, "Test User");
        assert!(account.is_active);
    }

    #[test]
    fn account_activation_state_change() {
        let mut account = Account::new(
            1,
            "test@example.com".to_string(),
            "Test User".to_string(),
        );

        account.deactivate();
        assert!(!account.is_active);

        account.activate();
        assert!(account.is_active);
    }

    #[test]
    fn account_timestamps_update() {
        let mut account = Account::new(
            1,
            "test@example.com".to_string(),
            "Test User".to_string(),
        );

        let original_updated = account.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        account.deactivate();

        assert!(account.updated_at > original_updated);
    }

    #[test]
    fn account_serialization() {
        let account = Account::new(
            1,
            "test@example.com".to_string(),
            "Test User".to_string(),
        );

        let json = serde_json::to_string(&account).expect("Failed to serialize");
        let deserialized: Account =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.id, account.id);
        assert_eq!(deserialized.email, account.email);
    }
}

mod product_tests {
    use super::*;

    #[test]
    fn product_creation() {
        let product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            10,
        );

        assert_eq!(product.id, 1);
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.price, 99.99);
        assert_eq!(product.stock, 10);
        assert!(product.is_active);
    }

    #[test]
    fn product_with_description() {
        let product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            10,
        )
        .with_description("A test product".to_string());

        assert_eq!(
            product.description,
            Some("A test product".to_string())
        );
    }

    #[test]
    fn product_stock_decrease() {
        let mut product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            10,
        );

        let result = product.decrease_stock(5);
        assert!(result.is_ok());
        assert_eq!(product.stock, 5);
    }

    #[test]
    fn product_stock_decrease_insufficient() {
        let mut product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            5,
        );

        let result = product.decrease_stock(10);
        assert!(result.is_err());
        assert_eq!(product.stock, 5); // Should not change
    }

    #[test]
    fn product_stock_increase() {
        let mut product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            5,
        );

        product.increase_stock(5);
        assert_eq!(product.stock, 10);
    }

    #[test]
    fn product_bulk_operations() {
        let mut product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            20,
        );

        // Decrease stock
        product.decrease_stock(5).expect("Failed to decrease stock");
        assert_eq!(product.stock, 15);

        // Increase stock
        product.increase_stock(10);
        assert_eq!(product.stock, 25);

        // Decrease again
        product
            .decrease_stock(8)
            .expect("Failed to decrease stock");
        assert_eq!(product.stock, 17);
    }

    #[test]
    fn product_deactivation() {
        let mut product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            10,
        );

        product.deactivate();
        assert!(!product.is_active);
    }

    #[test]
    fn product_clone() {
        let product1 = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            10,
        );

        let product2 = product1.clone();

        assert_eq!(product1.id, product2.id);
        assert_eq!(product1.name, product2.name);
        assert_eq!(product1.price, product2.price);
    }

    #[test]
    fn product_serialization() {
        let product = Product::new(
            1,
            "Test Product".to_string(),
            99.99,
            10,
        );

        let json = serde_json::to_string(&product).expect("Failed to serialize");
        let deserialized: Product =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.id, product.id);
        assert_eq!(deserialized.name, product.name);
        assert_eq!(deserialized.price, product.price);
    }
}
