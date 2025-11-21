//! PostgreSQL Driver Integration Tests
//!
//! These tests require the 'postgres' feature and a running PostgreSQL instance.
//! Set DATABASE_URL environment variable to your test database.

#![cfg(feature = "postgres")]

use rf_search::driver::{SearchDriver, SearchError};
use rf_search::drivers::PostgresSearchDriver;
use rf_search::searchable::{Searchable, SearchOptions};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
    id: String,
    title: String,
    content: String,
    status: String,
}

#[async_trait]
impl Searchable for Article {
    type Model = Article;

    fn searchable_fields() -> Vec<&'static str> {
        vec!["title", "content"]
    }

    fn to_searchable(&self) -> Self::Model {
        self.clone()
    }

    fn search_id(&self) -> String {
        self.id.clone()
    }

    fn index_name() -> &'static str {
        "articles"
    }

    fn filterable_fields() -> Vec<&'static str> {
        vec!["status"]
    }

    fn sortable_fields() -> Vec<&'static str> {
        vec!["title"]
    }
}

async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/rf_search_test".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database. Set DATABASE_URL env var.");

    // Create test table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS articles (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            status TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .expect("Failed to create test table");

    pool
}

async fn cleanup_test_db(pool: &PgPool) {
    sqlx::query("DROP TABLE IF EXISTS articles")
        .execute(pool)
        .await
        .expect("Failed to drop test table");
}

#[tokio::test]
async fn test_postgres_driver_creation() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::new(pool.clone());

    let result = driver.health_check().await;
    assert!(result.is_ok(), "Health check should succeed");

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_postgres_custom_language() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::with_language(pool.clone(), "spanish");

    let result = driver.health_check().await;
    assert!(result.is_ok(), "Health check should succeed");

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_create_and_drop_fts_index() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::new(pool.clone());

    // Create FTS index
    let result = driver.create_fts_index("articles", vec!["title", "content"]).await;
    assert!(result.is_ok(), "Should create FTS index");

    // Drop FTS index
    let result = driver.drop_fts_index("articles").await;
    assert!(result.is_ok(), "Should drop FTS index");

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_count_documents() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::new(pool.clone());

    // Clear any existing data
    sqlx::query("TRUNCATE TABLE articles")
        .execute(&pool)
        .await
        .unwrap();

    // Insert test documents
    sqlx::query(
        "INSERT INTO articles (id, title, content, status) VALUES
         ('1', 'Rust Programming', 'Learn Rust', 'published'),
         ('2', 'Web Development', 'Build web apps', 'published')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let count = driver.count::<Article>().await;
    assert!(count.is_ok(), "Count should succeed");
    assert_eq!(count.unwrap(), 2, "Should have 2 documents");

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_delete_document() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::new(pool.clone());

    // Clear and insert test data
    sqlx::query("TRUNCATE TABLE articles")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO articles (id, title, content, status) VALUES
         ('test-1', 'Test Article', 'Test content', 'published')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Delete the document
    let result = driver.delete::<Article>("test-1").await;
    assert!(result.is_ok(), "Delete should succeed");

    // Verify it's deleted
    let count = driver.count::<Article>().await.unwrap();
    assert_eq!(count, 0, "Should have 0 documents after delete");

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_clear_index() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::new(pool.clone());

    // Insert test data
    sqlx::query(
        "INSERT INTO articles (id, title, content, status) VALUES
         ('1', 'Article 1', 'Content 1', 'published'),
         ('2', 'Article 2', 'Content 2', 'published')
         ON CONFLICT DO NOTHING"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Clear the index
    let result = driver.clear_index::<Article>().await;
    assert!(result.is_ok(), "Clear index should succeed");

    // Verify it's empty
    let count = driver.count::<Article>().await.unwrap();
    assert_eq!(count, 0, "Should have 0 documents after clear");

    cleanup_test_db(&pool).await;
}

#[tokio::test]
async fn test_index_operations_are_noop() {
    let pool = setup_test_db().await;
    let driver = PostgresSearchDriver::new(pool.clone());

    let article = Article {
        id: "1".to_string(),
        title: "Test".to_string(),
        content: "Content".to_string(),
        status: "published".to_string(),
    };

    // Index operations should succeed but are no-ops for PostgreSQL
    let result = driver.index(&article).await;
    assert!(result.is_ok(), "Index should succeed (no-op)");

    let result = driver.index_many(vec![&article]).await;
    assert!(result.is_ok(), "Index many should succeed (no-op)");

    cleanup_test_db(&pool).await;
}

#[cfg(not(feature = "postgres"))]
#[test]
fn test_postgres_stub_returns_error() {
    use rf_search::drivers::PostgresSearchDriver;

    let result = PostgresSearchDriver::new(());
    assert!(result.is_err(), "Should return error when feature not enabled");

    if let Err(SearchError::FeatureNotEnabled(feature)) = result {
        assert_eq!(feature, "postgres");
    } else {
        panic!("Expected FeatureNotEnabled error");
    }
}
