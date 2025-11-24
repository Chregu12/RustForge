//! Integration tests for #[model] macro
//!
//! These tests verify that the macro generates correct code.

use rf_model_macro::model;

// Test 1: Basic model with minimal fields
#[model]
pub struct Product {
    pub name: String,
    pub price: f64,
}

// Test 2: Model with hidden field
#[model]
pub struct Account {
    pub username: String,

    #[hidden]
    pub password_hash: String,
}

// Test 3: Model with all standard fields explicitly defined
#[model]
pub struct Article {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Test 4: Model with optional fields
#[model]
pub struct Author {
    pub name: String,
    pub email: String,
    pub bio: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_has_id() {
        // This test verifies that the model has all expected fields
        // If the macro didn't add them, this wouldn't compile
        let product = Product {
            id: 1,
            name: "Test Product".to_string(),
            price: 99.99,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(product.id, 1);
        assert_eq!(product.name, "Test Product");
    }

    #[test]
    fn test_account_serialization() {
        // Test that #[hidden] works correctly
        let account = Account {
            id: 1,
            username: "testuser".to_string(),
            password_hash: "secret123".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&account).unwrap();

        // Password should not be in serialized output
        assert!(!json.contains("password_hash"));
        assert!(!json.contains("secret123"));

        // Username should be present
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_article_explicit_fields() {
        // When user provides id, created_at, updated_at, they should not be duplicated
        let article = Article {
            id: 1,
            title: "Test".to_string(),
            content: "Content".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(article.id, 1);
    }

    #[test]
    fn test_author_optional_field() {
        let author1 = Author {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            bio: Some("Writer".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let author2 = Author {
            id: 2,
            name: "Jane".to_string(),
            email: "jane@example.com".to_string(),
            bio: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(author1.bio.is_some());
        assert!(author2.bio.is_none());
    }

    #[test]
    fn test_clone_trait() {
        // Verify that Clone is derived
        let product = Product {
            id: 1,
            name: "Test".to_string(),
            price: 10.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let cloned = product.clone();
        assert_eq!(product.id, cloned.id);
    }

    #[test]
    fn test_debug_trait() {
        // Verify that Debug is derived
        let product = Product {
            id: 1,
            name: "Test".to_string(),
            price: 10.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let debug_str = format!("{:?}", product);
        assert!(debug_str.contains("Product"));
    }
}
