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

mod registry;
mod scope;
mod error;
mod scoped;
mod auto_resolve;

pub use registry::ServiceRegistry;
pub use scope::Scope;
pub use error::{ContainerError, ContainerResult};
pub use scoped::{ScopedContainer, ScopeManager};
pub use auto_resolve::{Resolvable, AutoResolver};
