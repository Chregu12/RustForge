//! Production-ready middleware for Axum

pub mod compression;
pub mod cors;
pub mod request_id;
pub mod timeout;
pub mod tracing;

pub use compression::compression_layer;
pub use cors::{cors_layer, CorsConfig};
pub use timeout::{default_timeout_layer, timeout_layer};
pub use tracing::tracing_layer;
