//! # rf-inertia
//!
//! Inertia.js adapter for RustForge, enabling the creation of modern single-page
//! applications using classic server-side routing and controllers.
//!
//! ## Features
//!
//! - **Props serialization**: Automatic JSON serialization of data
//! - **Shared data**: Global props accessible across all requests
//! - **Version checking**: Asset versioning for cache busting
//! - **Partial reloads**: Only reload changed components
//! - **Lazy props**: Deferred evaluation for performance
//! - **Axum integration**: First-class support for Axum extractors
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rf_inertia::{Inertia, InertiaConfig};
//! use axum::{Router, routing::get};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = InertiaConfig::new()
//!         .version("v1.0.0")
//!         .root_view("app");
//!
//!     let app = Router::new()
//!         .route("/", get(index))
//!         .layer(Inertia::layer(config));
//!
//!     // ... start server
//! }
//!
//! async fn index() -> Inertia {
//!     Inertia::render("Dashboard/Index")
//!         .with("user", user_data())
//!         .with("stats", stats_data())
//! }
//! ```

pub mod config;
pub mod error;
pub mod middleware;
pub mod props;
pub mod render;
pub mod response;
pub mod ssr;
pub mod version;

pub use config::InertiaConfig;
pub use error::{InertiaError, Result};
pub use middleware::{InertiaMiddleware, InertiaMiddlewareLayer};
pub use props::{LazyProp, Props, SharedProps};
pub use render::Inertia;
pub use response::InertiaResponse;
pub use version::AssetVersion;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        config::InertiaConfig,
        error::{InertiaError, Result},
        props::{LazyProp, Props, SharedProps},
        render::Inertia,
        response::InertiaResponse,
    };
}
