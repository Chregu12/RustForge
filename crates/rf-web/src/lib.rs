//! # rf-web: Web Layer for RustForge Framework
//!
//! This crate provides the web layer with Axum integration:
//! - RFC 7807 HTTP error responses
//! - Production-ready middleware stack
//! - Request context extraction
//! - Router builder for ergonomic setup
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_web::RouterBuilder;
//! use rf_core::{AppError, AppResult, RequestContext};
//! use axum::{extract::Path, Extension, Json, routing::get};
//!
//! #[derive(serde::Serialize)]
//! struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! async fn get_user(
//!     Extension(ctx): Extension<RequestContext>,
//!     Path(id): Path<i32>,
//! ) -> AppResult<Json<User>> {
//!     tracing::info!(trace_id = %ctx.trace_id(), "Fetching user {}", id);
//!
//!     Ok(Json(User {
//!         id,
//!         name: "John Doe".to_string(),
//!     }))
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RouterBuilder::new()
//!         .route("/users/:id", get(get_user))
//!         .build();
//!
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

pub mod extractors;
pub mod middleware;
pub mod response;
pub mod router;
pub mod versioning;

// Re-exports for convenience
pub use middleware::{compression_layer, cors_layer, timeout_layer, tracing_layer, CorsConfig};
pub use router::RouterBuilder;
pub use versioning::{ApiVersion, VersionedRouter};
