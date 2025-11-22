//! Service provider pattern for RustForge
//!
//! This crate implements Laravel-style service providers for RustForge applications.
//! Service providers are the central place to configure your application's services.
//!
//! # Quick Start
//!
//! ```rust
//! use rf_providers::{ServiceProvider, Application};
//! use async_trait::async_trait;
//!
//! struct MyServiceProvider;
//!
//! #[async_trait]
//! impl ServiceProvider for MyServiceProvider {
//!     async fn register(&self, app: &mut Application) -> anyhow::Result<()> {
//!         // Register bindings
//!         app.bind("database", || {
//!             Box::new("Database connection") as Box<dyn std::any::Any + Send + Sync>
//!         });
//!         Ok(())
//!     }
//!
//!     async fn boot(&self, app: &Application) -> anyhow::Result<()> {
//!         // Boot services
//!         Ok(())
//!     }
//! }
//! ```

mod application;
mod provider;

pub use application::{Application, Container};
pub use provider::{DeferredProvider, ServiceProvider};
