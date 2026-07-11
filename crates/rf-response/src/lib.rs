//! Response helpers for RustForge.
//!
//! Provides convenient response builders for Axum applications and a set of
//! free-function helpers that mirror the `rf::prelude::*` surface.
//!
//! ## Free-function helpers (available via `rf::prelude::*`)
//!
//! These are the response helpers you use directly in handlers:
//!
//! ```rust,no_run
//! use rf_response::{json, view, back, download};
//! use serde_json::json as sjson;
//!
//! # async fn handler() {
//! // Serialize any `Serialize` value to an application/json response.
//! let r = json(sjson!({"status": "ok"}));
//!
//! // Render resources/views/home.blade.html with {{ title }} substituted.
//! let r = view("home", sjson!({"title": "Welcome"}));
//!
//! // Redirect to the Referer (falls back to "/").
//! let r = back();
//!
//! // Serve a file with Content-Disposition: attachment; filename=invoice.pdf
//! let r = download("/path/to/file.pdf");
//! # }
//! ```
//!
//! ## `Response` builder (method-chaining API)
//!
//! ```rust
//! use rf_response::Response;
//! use axum::http::StatusCode;
//!
//! # async fn example() {
//! let r = Response::json(&serde_json::json!({"status": "ok"}))
//!     .status(StatusCode::CREATED);
//!
//! let r = Response::redirect("/dashboard");
//! let r = Response::back();
//! let r = Response::no_content();
//! let r = Response::text("plain text");
//! # }
//! ```

mod macros;
mod response;
pub mod view;

pub use response::{back, download, json, redirect, Response, ResponseBuilder, StreamBody};
pub use view::{view, ViewResponse};

// Re-export commonly used types
pub use axum::http::StatusCode;
pub use axum::response::IntoResponse;
