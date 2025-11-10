//! Testing utilities for RustForge applications
//!
//! Provides comprehensive testing helpers including HTTP testing,
//! custom assertions, and test utilities.
//!
//! # Features
//!
//! - HTTP testing with fluent API
//! - Custom assertions for common patterns
//! - Test response helpers
//!
//! # Quick Start
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

mod error;
mod http;
pub mod assertions;
pub mod factory;
pub mod seeder;

pub use error::{TestError, TestResult};
pub use http::{HttpTester, TestResponse};
pub use factory::{Factory, FactoryBuilder, FakeData};
pub use seeder::{Seeder, DatabaseSeeder};
