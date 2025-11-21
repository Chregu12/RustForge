//! Integration tests for HTTP request validation
//!
//! Tests the request handlers and validation logic.

use app::http::requests::store_product_request::StoreProduct;

mod store_product_request_tests {
    use super::*;

    #[test]
    fn valid_product_request() {
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: Some("A test product".to_string()),
            price: 99.99,
        };

        assert!(product.validate().is_ok());
    }

    #[test]
    fn invalid_name_too_short() {
        let product = StoreProduct {
            name: "AB".to_string(),
            description: None,
            price: 99.99,
        };

        let result = product.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 3 characters"));
    }

    #[test]
    fn invalid_name_empty() {
        let product = StoreProduct {
            name: "   ".to_string(),
            description: None,
            price: 99.99,
        };

        let result = product.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("required"));
    }

    #[test]
    fn invalid_price_zero() {
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: None,
            price: 0.0,
        };

        let result = product.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("greater than 0"));
    }

    #[test]
    fn invalid_price_negative() {
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: None,
            price: -10.0,
        };

        let result = product.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("greater than 0"));
    }

    #[test]
    fn valid_minimal_product() {
        let product = StoreProduct {
            name: "Min".to_string(),
            description: None,
            price: 0.01,
        };

        assert!(product.validate().is_ok());
    }

    #[test]
    fn valid_product_with_long_name() {
        let long_name = "A".repeat(255);
        let product = StoreProduct {
            name: long_name,
            description: None,
            price: 99.99,
        };

        assert!(product.validate().is_ok());
    }

    #[test]
    fn valid_product_with_long_description() {
        let long_description = "A".repeat(1000);
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: Some(long_description),
            price: 99.99,
        };

        assert!(product.validate().is_ok());
    }

    #[test]
    fn product_serialization() {
        let json = r#"{
            "name": "Test Product",
            "description": "A test product",
            "price": 99.99
        }"#;

        let product: StoreProduct = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(product.name, "Test Product");
        assert_eq!(product.description, Some("A test product".to_string()));
        assert_eq!(product.price, 99.99);
    }

    #[test]
    fn product_serialization_minimal() {
        let json = r#"{
            "name": "Test",
            "price": 10.0
        }"#;

        let product: StoreProduct = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(product.name, "Test");
        assert_eq!(product.description, None);
        assert_eq!(product.price, 10.0);
    }

    #[test]
    fn product_validation_boundary_conditions() {
        // Test with exactly 3 character name
        let product = StoreProduct {
            name: "ABC".to_string(),
            description: None,
            price: 0.01,
        };
        assert!(product.validate().is_ok());

        // Test with very large price
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: None,
            price: 999999.99,
        };
        assert!(product.validate().is_ok());
    }
}
