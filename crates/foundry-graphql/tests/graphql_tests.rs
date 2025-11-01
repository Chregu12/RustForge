use async_graphql::{EmptySubscription, Schema};
use foundry_graphql::{build_schema, GraphQLContext, MutationRoot, QueryRoot};
use sea_orm::{Database, DatabaseConnection};

async fn setup_test_db() -> DatabaseConnection {
    // Use in-memory SQLite for testing
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to test database");

    // Create tables
    let _ = db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            r#"
            CREATE TABLE products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                price DECIMAL(10,2) NOT NULL,
                stock INTEGER NOT NULL,
                sku TEXT NOT NULL UNIQUE,
                active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )
            "#
            .to_string(),
        ))
        .await;

    let _ = db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            r#"
            CREATE TABLE accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                role TEXT NOT NULL,
                active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )
            "#
            .to_string(),
        ))
        .await;

    db
}

#[tokio::test]
async fn test_product_queries() {
    let db = setup_test_db().await;
    let schema = build_schema(db);

    // Test creating a product
    let query = r#"
        mutation {
            createProduct(input: {
                name: "Test Product"
                description: "A test product"
                price: "19.99"
                stock: 100
                sku: "TEST-001"
                active: true
            }) {
                id
                name
                price
                stock
                sku
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty(), "Query failed: {:?}", response.errors);

    let data = response.data.into_json().unwrap();
    let product = &data["createProduct"];
    assert_eq!(product["name"], "Test Product");
    assert_eq!(product["sku"], "TEST-001");

    // Test querying products
    let query = r#"
        query {
            products {
                id
                name
                price
                stock
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    let products = data["products"].as_array().unwrap();
    assert_eq!(products.len(), 1);
}

#[tokio::test]
async fn test_product_mutations() {
    let db = setup_test_db().await;
    let schema = build_schema(db);

    // Create a product
    let query = r#"
        mutation {
            createProduct(input: {
                name: "Original Name"
                price: "29.99"
                stock: 50
                sku: "ORIG-001"
            }) {
                id
                name
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty());

    let product_id = response.data.into_json().unwrap()["createProduct"]["id"]
        .as_i64()
        .unwrap();

    // Update the product
    let query = format!(
        r#"
        mutation {{
            updateProduct(id: {}, input: {{
                name: "Updated Name"
                stock: 75
            }}) {{
                id
                name
                stock
            }}
        }}
        "#,
        product_id
    );

    let response = schema.execute(&query).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    assert_eq!(data["updateProduct"]["name"], "Updated Name");
    assert_eq!(data["updateProduct"]["stock"], 75);

    // Delete the product
    let query = format!(
        r#"
        mutation {{
            deleteProduct(id: {})
        }}
        "#,
        product_id
    );

    let response = schema.execute(&query).await;
    assert!(response.errors.is_empty());
    assert_eq!(response.data.into_json().unwrap()["deleteProduct"], true);
}

#[tokio::test]
async fn test_account_queries() {
    let db = setup_test_db().await;
    let schema = build_schema(db);

    // Create an account
    let query = r#"
        mutation {
            createAccount(input: {
                email: "test@example.com"
                name: "Test User"
                role: "user"
                active: true
            }) {
                id
                email
                name
                role
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty(), "Query failed: {:?}", response.errors);

    let data = response.data.into_json().unwrap();
    let account = &data["createAccount"];
    assert_eq!(account["email"], "test@example.com");
    assert_eq!(account["name"], "Test User");

    // Query accounts
    let query = r#"
        query {
            accounts {
                id
                email
                name
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    let accounts = data["accounts"].as_array().unwrap();
    assert_eq!(accounts.len(), 1);
}

#[tokio::test]
async fn test_validation_errors() {
    let db = setup_test_db().await;
    let schema = build_schema(db);

    // Try to create product with invalid price
    let query = r#"
        mutation {
            createProduct(input: {
                name: "Test"
                price: "invalid"
                stock: 10
                sku: "TEST"
            }) {
                id
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(!response.errors.is_empty());
    assert!(response.errors[0].message.contains("Invalid price"));

    // Try to create account with invalid email
    let query = r#"
        mutation {
            createAccount(input: {
                email: "notanemail"
                name: "Test"
                role: "user"
            }) {
                id
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(!response.errors.is_empty());
    assert!(response.errors[0].message.contains("Invalid email"));
}

#[tokio::test]
async fn test_duplicate_prevention() {
    let db = setup_test_db().await;
    let schema = build_schema(db);

    // Create first product
    let query = r#"
        mutation {
            createProduct(input: {
                name: "Product 1"
                price: "10.00"
                stock: 5
                sku: "DUP-001"
            }) {
                id
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty());

    // Try to create duplicate SKU
    let query = r#"
        mutation {
            createProduct(input: {
                name: "Product 2"
                price: "20.00"
                stock: 10
                sku: "DUP-001"
            }) {
                id
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(!response.errors.is_empty());
    assert!(response.errors[0].message.contains("already exists"));
}

#[tokio::test]
async fn test_search_functionality() {
    let db = setup_test_db().await;
    let schema = build_schema(db);

    // Create multiple products
    for i in 1..=3 {
        let query = format!(
            r#"
            mutation {{
                createProduct(input: {{
                    name: "Test Product {}"
                    price: "10.00"
                    stock: 10
                    sku: "SEARCH-{:03}"
                }}) {{
                    id
                }}
            }}
            "#,
            i, i
        );

        schema.execute(&query).await;
    }

    // Search for products
    let query = r#"
        query {
            searchProducts(query: "Test Product") {
                name
                sku
            }
        }
    "#;

    let response = schema.execute(query).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    let products = data["searchProducts"].as_array().unwrap();
    assert_eq!(products.len(), 3);
}
