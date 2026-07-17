//! # rf-container: Dependency Injection Container
//!
//! Type-safe dependency injection with three lifecycle scopes:
//! - **Singleton**: One instance for the entire application lifetime
//! - **Scoped**: One instance per request/scope
//! - **Transient**: New instance on every resolution
//!
//! ## Example
//!
//! ```rust
//! use rf_container::{ServiceRegistry, Scope};
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct DatabasePool {
//!     url: String,
//! }
//!
//! let mut registry = ServiceRegistry::new();
//!
//! // Register as singleton
//! registry.register(
//!     Scope::Singleton,
//!     || Arc::new(DatabasePool { url: "postgres://localhost".to_string() })
//! );
//!
//! // Resolve
//! let pool: Arc<DatabasePool> = registry.resolve().expect("Failed to resolve");
//! ```

mod auto_resolve;
mod error;
mod registry;
mod scope;
mod scoped;

pub use auto_resolve::{AutoResolver, Resolvable};
pub use error::{ContainerError, ContainerResult};
pub use registry::ServiceRegistry;
pub use scope::Scope;
pub use scoped::{ScopeManager, ScopedContainer};
