//! Stub System for RustForge
//!
//! This crate provides a customizable stub/template system for code generation
//! commands like `make:model`, `make:controller`, etc.
//!
//! # Features
//!
//! - Load custom stubs from project directory
//! - Template variable substitution
//! - Multiple case conversions (PascalCase, snake_case, kebab-case)
//! - Stub publishing and management
//! - Stub inheritance from defaults
//!
//! # Example
//!
//! ```rust
//! use foundry_stub_system::{StubManager, StubContext};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let manager = StubManager::new("./stubs");
//!
//!     let context = StubContext::new("User")
//!         .with_namespace("app::models")
//!         .with_property("name", "String")
//!         .with_property("email", "String");
//!
//!     let rendered = manager.render("model", context).await?;
//!     println!("{}", rendered);
//!
//!     Ok(())
//! }
//! ```

mod stub;
mod manager;
mod context;
mod variables;
mod error;
mod publisher;

pub use stub::{Stub, StubType};
pub use manager::StubManager;
pub use context::StubContext;
pub use variables::{StubVariables, CaseConverter};
pub use error::{StubError, Result};
pub use publisher::StubPublisher;
