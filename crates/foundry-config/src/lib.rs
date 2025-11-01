//! # Foundry Configuration Management
//!
//! Dynamic configuration with caching and environment support.

pub mod manager;
pub mod cache;
pub mod repository;

pub use manager::ConfigManager;
pub use cache::ConfigCache;
pub use repository::{ConfigRepository, DatabaseConfigRepository};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration not found: {0}")]
    NotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
}

pub type Result<T> = std::result::Result<T, ConfigError>;
