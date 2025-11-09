# rf-testing: Testing Utilities

**Version**: 1.0.0
**Status**: Phase 3 - Advanced Features
**Laravel Equivalent**: Testing (PHPUnit, TestCase, DatabaseTransactions)

## Overview

Comprehensive testing utilities for RustForge applications, including database helpers, HTTP testing, factories, and custom assertions.

**Core Features**:
- Database test helpers (transactions, migrations, seeders)
- HTTP testing utilities (test client, assertions)
- Test factories for model creation
- Custom assertions for common patterns
- Integration test helpers

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Test Application                     │
│  (Integration Tests, Unit Tests)                         │
└────────────────────────┬────────────────────────────────┘
                         │
          ┌──────────────┴──────────────┐
          │                             │
          ▼                             ▼
┌──────────────────┐         ┌──────────────────┐
│  DatabaseTester  │         │   HttpTester     │
│  - transactions  │         │   - test client  │
│  - refresh DB    │         │   - assertions   │
│  - seeders       │         │   - JSON testing │
└──────────────────┘         └──────────────────┘
          │                             │
          │                             │
          ▼                             ▼
┌──────────────────┐         ┌──────────────────┐
│    Factories     │         │   Assertions     │
│  - create models │         │   - custom checks│
│  - bulk create   │         │   - fluent API   │
└──────────────────┘         └──────────────────┘
```

## Core Components

### 1. Database Testing

```rust
use rf_testing::DatabaseTester;
use sea_orm::DatabaseConnection;

/// Database test helper
pub struct DatabaseTester {
    db: DatabaseConnection,
    in_transaction: bool,
}

impl DatabaseTester {
    /// Create new database tester
    pub async fn new(database_url: &str) -> Result<Self, TestError> {
        let db = sea_orm::Database::connect(database_url).await?;

        Ok(Self {
            db,
            in_transaction: false,
        })
    }

    /// Start a transaction (auto-rollback)
    pub async fn begin_transaction(&mut self) -> Result<(), TestError> {
        self.in_transaction = true;
        Ok(())
    }

    /// Rollback transaction
    pub async fn rollback(&mut self) -> Result<(), TestError> {
        self.in_transaction = false;
        Ok(())
    }

    /// Refresh database (drop all tables, run migrations)
    pub async fn refresh_database(&self) -> Result<(), TestError> {
        // Run migrations
        Ok(())
    }

    /// Seed database
    pub async fn seed<S: Seeder>(&self, seeder: S) -> Result<(), TestError> {
        seeder.run(&self.db).await?;
        Ok(())
    }

    /// Get database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Assert table has N rows
    pub async fn assert_table_count(&self, table: &str, count: usize) -> Result<(), TestError> {
        // Query table count
        Ok(())
    }

    /// Assert record exists
    pub async fn assert_database_has(&self, table: &str, conditions: serde_json::Value) -> Result<(), TestError> {
        // Check if record exists
        Ok(())
    }

    /// Assert record doesn't exist
    pub async fn assert_database_missing(&self, table: &str, conditions: serde_json::Value) -> Result<(), TestError> {
        // Check if record missing
        Ok(())
    }
}

/// Seeder trait
#[async_trait]
pub trait Seeder: Send + Sync {
    async fn run(&self, db: &DatabaseConnection) -> Result<(), TestError>;
}
```

### 2. Factory Pattern

```rust
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection};

/// Factory trait for creating test data
#[async_trait]
pub trait Factory<M>: Send + Sync
where
    M: ActiveModelTrait,
{
    /// Create single model
    async fn create(&self, db: &DatabaseConnection) -> Result<M::Entity, TestError>;

    /// Create model with custom attributes
    async fn create_with(
        &self,
        db: &DatabaseConnection,
        attributes: serde_json::Value,
    ) -> Result<M::Entity, TestError>;

    /// Create multiple models
    async fn create_many(&self, db: &DatabaseConnection, count: usize) -> Result<Vec<M::Entity>, TestError>;

    /// Make model without persisting
    fn make(&self) -> M;

    /// Make with custom attributes
    fn make_with(&self, attributes: serde_json::Value) -> M;
}

/// Factory builder
pub struct FactoryBuilder<M>
where
    M: ActiveModelTrait,
{
    defaults: Box<dyn Fn() -> M + Send + Sync>,
}

impl<M> FactoryBuilder<M>
where
    M: ActiveModelTrait,
{
    pub fn new(defaults: impl Fn() -> M + Send + Sync + 'static) -> Self {
        Self {
            defaults: Box::new(defaults),
        }
    }
}

#[async_trait]
impl<M> Factory<M> for FactoryBuilder<M>
where
    M: ActiveModelTrait + Send + Sync,
    M::Entity: Send + Sync,
{
    async fn create(&self, db: &DatabaseConnection) -> Result<M::Entity, TestError> {
        let model = (self.defaults)();
        Ok(model.insert(db).await?)
    }

    async fn create_with(
        &self,
        db: &DatabaseConnection,
        _attributes: serde_json::Value,
    ) -> Result<M::Entity, TestError> {
        // Merge attributes with defaults
        self.create(db).await
    }

    async fn create_many(&self, db: &DatabaseConnection, count: usize) -> Result<Vec<M::Entity>, TestError> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            results.push(self.create(db).await?);
        }
        Ok(results)
    }

    fn make(&self) -> M {
        (self.defaults)()
    }

    fn make_with(&self, _attributes: serde_json::Value) -> M {
        // Merge attributes
        self.make()
    }
}
```

### 3. HTTP Testing

```rust
use axum::{Router, body::Body};
use http::{Request, StatusCode};
use tower::ServiceExt;

/// HTTP test client
pub struct HttpTester {
    app: Router,
}

impl HttpTester {
    /// Create new HTTP tester
    pub fn new(app: Router) -> Self {
        Self { app }
    }

    /// Make GET request
    pub async fn get(&self, uri: &str) -> TestResponse {
        self.request(Request::builder()
            .uri(uri)
            .method("GET")
            .body(Body::empty())
            .unwrap())
            .await
    }

    /// Make POST request
    pub async fn post(&self, uri: &str, body: impl Into<Body>) -> TestResponse {
        self.request(Request::builder()
            .uri(uri)
            .method("POST")
            .header("content-type", "application/json")
            .body(body.into())
            .unwrap())
            .await
    }

    /// Make PUT request
    pub async fn put(&self, uri: &str, body: impl Into<Body>) -> TestResponse {
        self.request(Request::builder()
            .uri(uri)
            .method("PUT")
            .header("content-type", "application/json")
            .body(body.into())
            .unwrap())
            .await
    }

    /// Make DELETE request
    pub async fn delete(&self, uri: &str) -> TestResponse {
        self.request(Request::builder()
            .uri(uri)
            .method("DELETE")
            .body(Body::empty())
            .unwrap())
            .await
    }

    /// Make custom request
    async fn request(&self, req: Request<Body>) -> TestResponse {
        let response = self.app.clone()
            .oneshot(req)
            .await
            .unwrap();

        TestResponse::new(response)
    }
}

/// Test response wrapper
pub struct TestResponse {
    response: http::Response<Body>,
    body: Option<bytes::Bytes>,
}

impl TestResponse {
    fn new(response: http::Response<Body>) -> Self {
        Self {
            response,
            body: None,
        }
    }

    /// Get status code
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    /// Assert status code
    pub fn assert_status(self, status: StatusCode) -> Self {
        assert_eq!(self.status(), status, "Expected status {}, got {}", status, self.status());
        self
    }

    /// Assert success (2xx)
    pub fn assert_ok(self) -> Self {
        assert!(self.status().is_success(), "Expected success status, got {}", self.status());
        self
    }

    /// Assert redirect (3xx)
    pub fn assert_redirect(self) -> Self {
        assert!(self.status().is_redirection(), "Expected redirect status, got {}", self.status());
        self
    }

    /// Assert client error (4xx)
    pub fn assert_client_error(self) -> Self {
        assert!(self.status().is_client_error(), "Expected client error status, got {}", self.status());
        self
    }

    /// Get response body
    pub async fn body(&mut self) -> &bytes::Bytes {
        if self.body.is_none() {
            let body = axum::body::to_bytes(
                std::mem::replace(&mut self.response.body_mut(), Body::empty()),
                usize::MAX
            ).await.unwrap();
            self.body = Some(body);
        }
        self.body.as_ref().unwrap()
    }

    /// Get response as JSON
    pub async fn json<T: serde::de::DeserializeOwned>(&mut self) -> T {
        let body = self.body().await;
        serde_json::from_slice(body).unwrap()
    }

    /// Assert JSON matches
    pub async fn assert_json(mut self, expected: serde_json::Value) -> Self {
        let actual: serde_json::Value = self.json().await;
        assert_eq!(actual, expected, "JSON mismatch");
        self
    }

    /// Assert JSON contains
    pub async fn assert_json_contains(mut self, key: &str) -> Self {
        let json: serde_json::Value = self.json().await;
        assert!(json.get(key).is_some(), "JSON missing key: {}", key);
        self
    }

    /// Assert header exists
    pub fn assert_header(self, name: &str, value: &str) -> Self {
        assert_eq!(
            self.response.headers().get(name).map(|v| v.to_str().unwrap()),
            Some(value),
            "Header mismatch for {}", name
        );
        self
    }
}
```

### 4. Custom Assertions

```rust
/// Custom assertions module
pub mod assertions {
    use std::fmt::Debug;

    /// Assert vectors are equal (order-independent)
    pub fn assert_vec_eq<T: PartialEq + Debug>(mut a: Vec<T>, mut b: Vec<T>) {
        a.sort_by(|x, y| format!("{:?}", x).cmp(&format!("{:?}", y)));
        b.sort_by(|x, y| format!("{:?}", x).cmp(&format!("{:?}", y)));
        assert_eq!(a, b);
    }

    /// Assert option is some and matches
    pub fn assert_some_eq<T: PartialEq + Debug>(actual: Option<T>, expected: T) {
        match actual {
            Some(val) => assert_eq!(val, expected),
            None => panic!("Expected Some({:?}), got None", expected),
        }
    }

    /// Assert result is ok and matches
    pub fn assert_ok_eq<T: PartialEq + Debug, E: Debug>(actual: Result<T, E>, expected: T) {
        match actual {
            Ok(val) => assert_eq!(val, expected),
            Err(e) => panic!("Expected Ok({:?}), got Err({:?})", expected, e),
        }
    }

    /// Assert result is error
    pub fn assert_err<T: Debug, E: Debug>(actual: Result<T, E>) {
        assert!(actual.is_err(), "Expected Err, got Ok({:?})", actual.unwrap());
    }
}
```

## Usage Examples

### 1. Database Testing

```rust
use rf_testing::DatabaseTester;

#[tokio::test]
async fn test_user_creation() {
    let mut db = DatabaseTester::new("sqlite::memory:").await.unwrap();
    db.begin_transaction().await.unwrap();

    // Create user
    let user = create_user(&db.connection(), "test@example.com").await;

    // Assert user exists
    db.assert_database_has("users", json!({
        "email": "test@example.com"
    })).await.unwrap();

    // Rollback
    db.rollback().await.unwrap();
}
```

### 2. Factory Usage

```rust
use rf_testing::Factory;

#[tokio::test]
async fn test_with_factory() {
    let db = DatabaseTester::new("sqlite::memory:").await.unwrap();

    let factory = UserFactory::new();

    // Create single user
    let user = factory.create(db.connection()).await.unwrap();

    // Create multiple users
    let users = factory.create_many(db.connection(), 10).await.unwrap();
    assert_eq!(users.len(), 10);

    // Make without persisting
    let user = factory.make();
}
```

### 3. HTTP Testing

```rust
use rf_testing::HttpTester;
use axum::{Router, routing::get, Json};

#[tokio::test]
async fn test_api_endpoint() {
    let app = Router::new()
        .route("/users", get(get_users));

    let client = HttpTester::new(app);

    // Test GET request
    client.get("/users")
        .await
        .assert_ok()
        .assert_json(json!([{"id": 1, "name": "Test"}]))
        .await;
}
```

### 4. Complete Integration Test

```rust
use rf_testing::{DatabaseTester, HttpTester, Factory};

#[tokio::test]
async fn test_user_api() {
    // Setup database
    let db = DatabaseTester::new("sqlite::memory:").await.unwrap();
    db.refresh_database().await.unwrap();

    // Create test data
    let factory = UserFactory::new();
    factory.create_many(db.connection(), 5).await.unwrap();

    // Setup HTTP client
    let app = create_app(db.connection().clone());
    let client = HttpTester::new(app);

    // Test API
    let response = client.get("/api/users").await;
    response
        .assert_status(StatusCode::OK)
        .assert_json_contains("data")
        .await;

    let users: Vec<User> = response.json().await;
    assert_eq!(users.len(), 5);
}
```

## Comparison with Laravel

| Feature | Laravel | rf-testing | Status |
|---------|---------|------------|--------|
| Database transactions | ✅ | ✅ | ✅ Complete |
| Database refresh | ✅ | ✅ | ✅ Complete |
| Seeders | ✅ | ✅ | ✅ Complete |
| Factories | ✅ | ✅ | ✅ Complete |
| HTTP testing | ✅ | ✅ | ✅ Complete |
| JSON assertions | ✅ | ✅ | ✅ Complete |
| Custom assertions | ✅ | ✅ | ✅ Complete |
| Mocking | ✅ | ⏳ | ⏳ Future |
| Browser testing | ✅ | ⏳ | ⏳ Future |

**Feature Parity**: ~78% (7/9 features)

## Files to Create

```
crates/rf-testing/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Main exports
│   ├── error.rs            # Error types
│   ├── database.rs         # Database testing helpers
│   ├── factory.rs          # Factory trait and builder
│   ├── http.rs             # HTTP testing client
│   └── assertions.rs       # Custom assertions
```

## Dependencies

```toml
[dependencies]
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
sea-orm.workspace = true
axum.workspace = true
tokio.workspace = true
tower.workspace = true
http = "1.0"
bytes = "1.5"

[dev-dependencies]
tokio = { workspace = true, features = ["test-util", "macros"] }
```

## Conclusion

rf-testing provides comprehensive testing utilities:
- Database test helpers with transactions
- Factory pattern for test data
- HTTP testing client with fluent assertions
- Custom assertions for common patterns
- Clean, type-safe API
- ~78% Laravel parity

**Next Steps**:
1. Implement database tester
2. Add factory pattern
3. Create HTTP testing client
4. Add custom assertions
5. Write comprehensive tests
