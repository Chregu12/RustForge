//! HTTP Middleware
//!
//! Middleware inspects and filters HTTP requests entering your application.
//! RustForge builds on Axum/Tower, so middleware are Tower layers or Axum
//! `middleware::from_fn` functions.
//!
//! Common cross-cutting concerns (tracing, CORS, compression, timeouts) are
//! already provided by `RouterBuilder` in `src/main.rs` — enable them with
//! `.with_tracing(true)`, `.with_cors(true)`, etc.
//!
//! Example of a custom `from_fn` middleware:
//!
//! ```ignore
//! use axum::{extract::Request, middleware::Next, response::Response};
//!
//! pub async fn log_requests(req: Request, next: Next) -> Response {
//!     tracing::info!("{} {}", req.method(), req.uri().path());
//!     next.run(req).await
//! }
//! ```

// Add your middleware here.
