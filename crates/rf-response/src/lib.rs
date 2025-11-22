//! Response helpers for RustForge
//!
//! Provides convenient response builders for Axum applications.
//!
//! # Quick Start
//!
//! ```rust
//! use rf_response::Response;
//! use axum::http::StatusCode;
//!
//! # async fn example() {
//! // JSON response
//! let response = Response::json(&serde_json::json!({"status": "ok"}))
//!     .status(StatusCode::OK);
//!
//! // Redirect
//! let response = Response::redirect("/dashboard");
//!
//! // Download
//! let response = Response::download("/path/to/file.pdf", "invoice.pdf");
//! # }
//! ```

mod macros;
mod response;

pub use response::{Response, ResponseBuilder, StreamBody};

// Re-export commonly used types
pub use axum::http::StatusCode;
pub use axum::response::IntoResponse;
