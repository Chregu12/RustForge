//! Testing utilities for RustForge applications
//!
//! Provides comprehensive testing helpers including HTTP testing,
//! custom assertions, database assertions, and test fakes.
//!
//! # Features
//!
//! - **HTTP Testing**: Fluent API for testing Axum applications
//! - **Database Assertions**: Laravel-style database assertions (`assert_database_has!`)
//! - **Queue Fakes**: Test job dispatching without processing (`QueueFake`)
//! - **Event Fakes**: Test event dispatching and listeners (`EventFake`)
//! - **Custom Assertions**: Common assertion patterns
//! - **Factories & Seeders**: Generate test data easily
//!
//! # Quick Start
//!
//! ## Database Assertions
//!
//! ```ignore
//! use rf_testing::{assert_database_has, assert_database_count};
//!
//! #[tokio::test]
//! async fn test_user_creation() {
//!     // Create a user
//!     create_user(&db, "test@example.com").await?;
//!
//!     // Assert user exists
//!     assert_database_has!("users", {
//!         "email" => "test@example.com",
//!         "active" => true
//!     });
//!
//!     assert_database_count!("users", 1);
//! }
//! ```
//!
//! ## Queue Fakes
//!
//! ```
//! use rf_testing::fakes::{QueueFake, queue::JobRecord};
//! use serde_json::json;
//!
//! let fake = QueueFake::new();
//!
//! // Dispatch a job
//! fake.record_push(JobRecord {
//!     job_type: "send_email".to_string(),
//!     payload: json!({"to": "test@example.com"}),
//!     queue: "default".to_string(),
//!     job_id: "123".to_string(),
//!     priority: 0,
//! });
//!
//! // Assert
//! fake.assert_pushed("send_email");
//! fake.assert_pushed_times("send_email", 1);
//! ```
//!
//! ## Event Fakes
//!
//! ```
//! use rf_testing::fakes::EventFake;
//! use serde_json::json;
//!
//! let fake = EventFake::new();
//!
//! // Dispatch an event
//! fake.dispatch_simple("UserCreated", json!({
//!     "user_id": 1,
//!     "email": "test@example.com"
//! }));
//!
//! // Assert
//! fake.assert_dispatched("UserCreated");
//! fake.assert_dispatched_times("UserCreated", 1);
//! ```
//!
//! ## HTTP Testing
//!
//! ```
//! use rf_testing::HttpTester;
//! use axum::{Router, routing::get, Json};
//! use serde_json::json;
//!
//! # async fn example() {
//! async fn get_user() -> Json<serde_json::Value> {
//!     Json(json!({"id": 1, "name": "Test"}))
//! }
//!
//! let app = Router::new().route("/user", get(get_user));
//! let client = HttpTester::new(app);
//!
//! client.get("/user")
//!     .await
//!     .assert_ok()
//!     .assert_json(json!({"id": 1, "name": "Test"}))
//!     .await;
//! # }
//! ```
//!
//! ## Custom Assertions
//!
//! ```
//! use rf_testing::assertions::*;
//!
//! // Option assertions
//! assert_some_eq(Some(42), 42);
//! let value = assert_some(Some(10));
//! assert_none::<i32>(None);
//!
//! // Result assertions
//! assert_ok_eq(Ok::<_, String>(42), 42);
//! let value = assert_ok(Ok::<_, String>(10));
//! let err = assert_err(Err::<i32, _>("error"));
//!
//! // String assertions
//! assert_contains("Hello, World!", "World");
//! assert_not_contains("Hello", "Goodbye");
//!
//! // Range assertions
//! assert_in_range(5, 1, 10);
//! ```
//!
//! For more detailed documentation, see the [Testing Guide](TESTING_GUIDE.md).

pub mod assertions;
pub mod database;
pub mod docker;
mod error;
pub mod factory;
pub mod factory_advanced;
pub mod fake;
pub mod fakes;
mod http;
mod http_client;
pub mod seeder;

pub use database::{refresh_database, DatabaseTestError, TestDatabase, TestDatabaseConfig};
pub use docker::{
    database_available, mailhog_available, postgres_available, redis_available, s3_available,
    DockerCompose, Service,
};
pub use error::{TestError, TestResult};
pub use factory::{Factory, FactoryBuilder, FactoryDefinition, FactoryError};
pub use factory_advanced::{EnhancedFactory, FactoryState, RelationshipBuilder, Sequence};
pub use fake::Fake;
pub use fakes::{EventFake, QueueFake};
pub use http::{HttpTester, TestResponse};
pub use http_client::{RequestBuilder, TestClient, TestResponseBuilder};
pub use seeder::{DatabaseSeeder, Seeder, SeederError, SeederRunner};

// Re-export database assertion macros (defined in database.rs)
// The macros are automatically available at crate root when defined with #[macro_export]
