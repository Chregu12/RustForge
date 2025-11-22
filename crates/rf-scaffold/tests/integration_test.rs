//! Integration tests for rf-scaffold

use rf_scaffold::{ModelOptions, ScaffoldEngine};
use tempfile::tempdir;

#[tokio::test]
async fn test_full_workflow() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    // Generate model
    let model_path = scaffold
        .generate_model(
            "Product",
            &ModelOptions {
                fields: vec![("name", "String"), ("price", "f64"), ("in_stock", "bool")],
                with_migration: false,
                with_factory: false,
            },
        )
        .await
        .unwrap();

    assert!(model_path.exists());
    let content = tokio::fs::read_to_string(&model_path).await.unwrap();
    assert!(content.contains("pub struct Product"));
    assert!(content.contains("pub name: String"));
    assert!(content.contains("pub price: f64"));
    assert!(content.contains("pub in_stock: bool"));
}

#[tokio::test]
async fn test_controller_generation() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    // Generate simple controller
    let controller_path = scaffold
        .generate_controller("ProductController", false)
        .await
        .unwrap();
    assert!(controller_path.exists());

    let content = tokio::fs::read_to_string(&controller_path).await.unwrap();
    assert!(content.contains("pub struct ProductController"));
    assert!(content.contains("pub async fn index()"));
}

#[tokio::test]
async fn test_resource_controller_generation() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    // Generate resource controller
    let controller_path = scaffold
        .generate_controller("OrderController", true)
        .await
        .unwrap();
    assert!(controller_path.exists());

    let content = tokio::fs::read_to_string(&controller_path).await.unwrap();
    assert!(content.contains("pub async fn index()"));
    assert!(content.contains("pub async fn store("));
    assert!(content.contains("pub async fn show("));
    assert!(content.contains("pub async fn update("));
    assert!(content.contains("pub async fn destroy("));
}

#[tokio::test]
async fn test_service_generation() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    let service_path = scaffold.generate_service("PaymentService").await.unwrap();
    assert!(service_path.exists());

    let content = tokio::fs::read_to_string(&service_path).await.unwrap();
    assert!(content.contains("pub struct PaymentService"));
    assert!(content.contains("async fn execute(&self)"));
}

#[tokio::test]
async fn test_migration_generation() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    let migration_path = scaffold
        .generate_migration("add_status_to_orders")
        .await
        .unwrap();
    assert!(migration_path.exists());
    assert!(migration_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("add_status_to_orders"));

    let content = tokio::fs::read_to_string(&migration_path).await.unwrap();
    assert!(content.contains("impl MigrationTrait"));
    assert!(content.contains("async fn up(&self"));
    assert!(content.contains("async fn down(&self"));
}

#[tokio::test]
async fn test_model_with_migration() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    // Generate model with migration
    let model_path = scaffold
        .generate_model(
            "Category",
            &ModelOptions {
                fields: vec![("name", "String"), ("description", "String")],
                with_migration: true,
                with_factory: false,
            },
        )
        .await
        .unwrap();

    assert!(model_path.exists());

    // Check migration was created
    let migration_dir = dir.path().join("migrations");
    assert!(migration_dir.exists());

    let entries: Vec<_> = std::fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert!(entries.len() > 0);
    assert!(entries[0]
        .file_name()
        .to_string_lossy()
        .contains("create_categories_table"));
}

#[tokio::test]
async fn test_naming_conventions() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();
    let naming = scaffold.naming();

    // PascalCase
    assert_eq!(naming.to_pascal_case("user_profile"), "UserProfile");
    assert_eq!(naming.to_pascal_case("http-client"), "HttpClient");

    // snake_case
    assert_eq!(naming.to_snake_case("UserProfile"), "user_profile");
    assert_eq!(naming.to_snake_case("HTTPClient"), "http_client");

    // kebab-case
    assert_eq!(naming.to_kebab_case("UserProfile"), "user-profile");

    // Pluralization
    assert_eq!(naming.pluralize("category"), "categories");
    assert_eq!(naming.pluralize("person"), "people");
    assert_eq!(naming.pluralize("box"), "boxes");

    // Singularization
    assert_eq!(naming.singularize("categories"), "category");
    assert_eq!(naming.singularize("people"), "person");

    // Extract base
    assert_eq!(naming.extract_base("UserController"), "User");
    assert_eq!(naming.extract_base("ProductService"), "Product");
}

#[tokio::test]
async fn test_custom_template_registration() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    // Register custom template (just verify no error)
    let result = scaffold
        .register_template(
            "test-template",
            "// Generated: {{name}}\npub struct {{name}} {}",
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_overwrite_protection() {
    let dir = tempdir().unwrap();
    let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

    // Generate first model
    scaffold
        .generate_model(
            "User",
            &ModelOptions {
                fields: vec![("name", "String")],
                with_migration: false,
                with_factory: false,
            },
        )
        .await
        .unwrap();

    // Try to generate again (should fail)
    let result = scaffold
        .generate_model(
            "User",
            &ModelOptions {
                fields: vec![("email", "String")],
                with_migration: false,
                with_factory: false,
            },
        )
        .await;

    assert!(result.is_err());
}
