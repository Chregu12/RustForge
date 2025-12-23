//! # rf-config: Type-Safe Configuration Management
//!
//! Provides hierarchical configuration loading with:
//! - Default values
//! - File-based configuration (TOML)
//! - Environment variable overrides
//! - Type-safe access
//! - Validation
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_config::{AppConfig, ConfigLoader};
//!
//! let config = ConfigLoader::new()
//!     .env("development")
//!     .load::<AppConfig>()
//!     .expect("Failed to load config");
//!
//! println!("Server running on {}:{}", config.server.host, config.server.port);
//! ```

pub mod facade;
pub mod loader;
pub mod types;

pub use loader::ConfigLoader;
pub use types::{AppConfig, AuthConfig, DatabaseConfig, ServerConfig};

// Re-export facade types (Laravel-style static API)
pub use facade::{Config, GLOBAL_CONFIG};
