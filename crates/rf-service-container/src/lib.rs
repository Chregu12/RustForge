//! Service Container and Dependency Injection system for Foundry.
//!
//! Provides a powerful DI container similar to Laravel's Service Container.

mod binding;
mod container;
mod context;
mod error;
mod provider;

pub mod fast_container;
pub mod providers;

pub use binding::{Binding, BindingType, Factory};
pub use container::Container;
pub use context::ContextualBinding;
pub use error::{ContainerError, Result};
pub use fast_container::{ContainerStats, FastContainer};
pub use provider::{ProviderRegistry, ServiceProvider};

// Re-export commonly used types
pub use async_trait::async_trait;

// Re-export built-in providers
pub use providers::{
    ApplicationServiceProvider, AuthServiceProvider, CacheServiceProvider, DatabaseServiceProvider,
    MailServiceProvider,
};
