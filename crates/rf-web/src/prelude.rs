//! # rf-web Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-web.
//!
//! ## Usage
//!
//! ```rust
//! use rf_web::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: csrf::{csrf_field, csrf_meta, csrf_token, CsrfConfig, CsrfLayer, CsrfMiddleware, CsrfToken, CsrfTokenStore};
pub use crate:: middleware::{compression_layer, cors_layer, timeout_layer, tracing_layer, CorsConfig};
pub use crate:: router::RouterBuilder;
pub use crate::session::{Session, SessionConfig, SessionDriver};
pub use crate:: versioning::{ApiVersion, VersionedRouter};
