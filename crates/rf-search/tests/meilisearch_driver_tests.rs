//! Meilisearch Driver Integration Tests
//!
//! These tests require the 'meilisearch' feature and a running Meilisearch instance.
//! Set MEILI_URL and optionally MEILI_KEY environment variables.

#![cfg(feature = "meilisearch")]

use rf_search::driver::{ConfigurableDriver, SearchDriver, SearchError};
use rf_search::drivers::MeilisearchDriver;
use rf_search::searchable::{Searchable, SearchOptions};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: String,
    name: String,
    description: String,
    category: String,
    price: f64,
}

#[async_trait]
impl Searchable for Product {
    type Model = Product;

    fn searchable_fields() -> Vec<&'static str> {
        vec!["name", "description"]
    }

    fn to_searchable(&self) -> Self::Model {
        self.clone()
    }

    fn search_id(&self) -> String {
        self.id.clone()
    }

    fn index_name() -> &'static str {
        "products_test"
    }

    fn filterable_fields() -> Vec<&'static str> {
        vec!["category"]
    }

    fn sortable_fields() -> Vec<&'static str> {
        vec!["price", "name"]
    }
}

fn get_meili_url() -> String {
    std::env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7700".to_string())
}

fn get_meili_key() -> Option<String> {
    std::env::var("MEILI_KEY").ok()
}

async fn setup_driver() -> MeilisearchDriver {
    let url = get_meili_url();
    let key = get_meili_key();

    MeilisearchDriver::new(&url, key.as_deref())
        .expect("Failed to create Meilisearch driver. Make sure Meilisearch is running.")
}

async fn cleanup_index(driver: &MeilisearchDriver) {
    // Try to delete the index, ignore errors if it doesn't exist
    let _ = driver.delete_index::<Product>().await;

    // Wait a bit for deletion to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_meilisearch_driver_creation() {
    let driver = setup_driver().await;

    let result = driver.health_check().await;
    assert!(result.is_ok(), "Health check should succeed. Make sure Meilisearch is running at {}", get_meili_url());
}

#[tokio::test]
async fn test_create_and_delete_index() {
    let driver = setup_driver().await;

    // Create index
    let result = driver.create_index::<Product>().await;
    assert!(result.is_ok(), "Should create index");

    // Wait for index creation
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Delete index
    let result = driver.delete_index::<Product>().await;
    assert!(result.is_ok(), "Should delete index");
}

#[tokio::test]
async fn test_configure_index() {
    let driver = setup_driver().await;

    // Create index first
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Configure index
    let result = driver.configure_index::<Product>().await;
    assert!(result.is_ok(), "Should configure index");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_index_single_document() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let product = Product {
        id: "1".to_string(),
        name: "Rust Programming Book".to_string(),
        description: "Learn Rust programming language".to_string(),
        category: "books".to_string(),
        price: 49.99,
    };

    let result = driver.index(&product).await;
    assert!(result.is_ok(), "Should index document");

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_index_many_documents() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let products = vec![
        Product {
            id: "1".to_string(),
            name: "Rust Book".to_string(),
            description: "Programming in Rust".to_string(),
            category: "books".to_string(),
            price: 49.99,
        },
        Product {
            id: "2".to_string(),
            name: "Python Guide".to_string(),
            description: "Learn Python".to_string(),
            category: "books".to_string(),
            price: 39.99,
        },
    ];

    let refs: Vec<&Product> = products.iter().collect();
    let result = driver.index_many(refs).await;
    assert!(result.is_ok(), "Should index multiple documents");

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify count
    let count = driver.count::<Product>().await;
    assert!(count.is_ok(), "Count should succeed");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_search_documents() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    driver.configure_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Index documents
    let products = vec![
        Product {
            id: "1".to_string(),
            name: "Rust Programming".to_string(),
            description: "Learn Rust language".to_string(),
            category: "books".to_string(),
            price: 49.99,
        },
        Product {
            id: "2".to_string(),
            name: "Web Development".to_string(),
            description: "Build web apps".to_string(),
            category: "books".to_string(),
            price: 39.99,
        },
    ];

    let refs: Vec<&Product> = products.iter().collect();
    driver.index_many(refs).await.unwrap();

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Search
    let options = SearchOptions::new().limit(10);
    let result = driver.search::<Product>("Rust", Some(options)).await;
    assert!(result.is_ok(), "Search should succeed");

    let search_result = result.unwrap();
    assert!(!search_result.hits.is_empty(), "Should find at least one result");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_search_with_filters() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    driver.configure_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Index documents
    let products = vec![
        Product {
            id: "1".to_string(),
            name: "Rust Book".to_string(),
            description: "Programming".to_string(),
            category: "books".to_string(),
            price: 49.99,
        },
        Product {
            id: "2".to_string(),
            name: "Rust Mug".to_string(),
            description: "Coffee mug".to_string(),
            category: "merchandise".to_string(),
            price: 15.99,
        },
    ];

    let refs: Vec<&Product> = products.iter().collect();
    driver.index_many(refs).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Search with filter
    let options = SearchOptions::new()
        .filter("category", "books")
        .limit(10);

    let result = driver.search::<Product>("Rust", Some(options)).await;
    assert!(result.is_ok(), "Search with filter should succeed");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_delete_document() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Index a document
    let product = Product {
        id: "delete-test".to_string(),
        name: "Test Product".to_string(),
        description: "For deletion".to_string(),
        category: "test".to_string(),
        price: 10.0,
    };

    driver.index(&product).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Delete it
    let result = driver.delete::<Product>("delete-test").await;
    assert!(result.is_ok(), "Delete should succeed");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_update_document() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Index a document
    let product = Product {
        id: "update-test".to_string(),
        name: "Original Name".to_string(),
        description: "Original description".to_string(),
        category: "test".to_string(),
        price: 10.0,
    };

    driver.index(&product).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Update it
    let updated = Product {
        id: "update-test".to_string(),
        name: "Updated Name".to_string(),
        description: "Updated description".to_string(),
        category: "test".to_string(),
        price: 20.0,
    };

    let result = driver.update(&updated).await;
    assert!(result.is_ok(), "Update should succeed");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_clear_index() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Index some documents
    let products = vec![
        Product {
            id: "1".to_string(),
            name: "Product 1".to_string(),
            description: "Test".to_string(),
            category: "test".to_string(),
            price: 10.0,
        },
        Product {
            id: "2".to_string(),
            name: "Product 2".to_string(),
            description: "Test".to_string(),
            category: "test".to_string(),
            price: 20.0,
        },
    ];

    let refs: Vec<&Product> = products.iter().collect();
    driver.index_many(refs).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Clear the index
    let result = driver.clear_index::<Product>().await;
    assert!(result.is_ok(), "Clear index should succeed");

    cleanup_index(&driver).await;
}

#[tokio::test]
async fn test_count_documents() {
    let driver = setup_driver().await;
    driver.create_index::<Product>().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Index documents
    let products = vec![
        Product {
            id: "1".to_string(),
            name: "Product 1".to_string(),
            description: "Test".to_string(),
            category: "test".to_string(),
            price: 10.0,
        },
        Product {
            id: "2".to_string(),
            name: "Product 2".to_string(),
            description: "Test".to_string(),
            category: "test".to_string(),
            price: 20.0,
        },
    ];

    let refs: Vec<&Product> = products.iter().collect();
    driver.index_many(refs).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let count = driver.count::<Product>().await;
    assert!(count.is_ok(), "Count should succeed");
    assert_eq!(count.unwrap(), 2, "Should have 2 documents");

    cleanup_index(&driver).await;
}

#[cfg(not(feature = "meilisearch"))]
#[test]
fn test_meilisearch_stub_returns_error() {
    use rf_search::drivers::MeilisearchDriver;

    let result = MeilisearchDriver::new("http://localhost:7700", None);
    assert!(result.is_err(), "Should return error when feature not enabled");

    if let Err(SearchError::FeatureNotEnabled(feature)) = result {
        assert_eq!(feature, "meilisearch");
    } else {
        panic!("Expected FeatureNotEnabled error");
    }
}
