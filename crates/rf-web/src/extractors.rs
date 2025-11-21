//! Axum extractors for request context
//!
//! RequestContext is automatically available in handlers through Axum's Extension extractor.
//! The RequestIdMiddleware injects it into request extensions.
//!
//! # Example
//!
//! ```rust,no_run
//! use axum::Extension;
//! use rf_core::RequestContext;
//!
//! async fn handler(Extension(ctx): Extension<RequestContext>) -> String {
//!     format!("Trace ID: {}", ctx.trace_id())
//! }
//! ```

// Note: RequestContext is extracted using axum::Extension<RequestContext>
// The RequestIdMiddleware automatically injects it into request extensions
