//! Service Container and Dependency Injection system for Foundry.
//!
//! Provides a powerful DI container similar to Laravel's Service Container.

mod container;
mod error;
mod provider;
mod binding;
mod context;

pub mod fast_container;
pub mod providers;

pub use container::Container;
pub use error::{ContainerError, Result};
pub use provider::{ServiceProvider, ProviderRegistry};
pub use binding::{Binding, BindingType, Factory};
pub use context::ContextualBinding;
pub use fast_container::{FastContainer, ContainerStats};

// Re-export commonly used types
pub use async_trait::async_trait;

// Re-export built-in providers
pub use providers::{
    ApplicationServiceProvider, AuthServiceProvider, CacheServiceProvider,
    DatabaseServiceProvider, MailServiceProvider,
};
