//! # Foundry Asset Publishing
//!
//! Asset publishing and management system for Foundry applications.
//!
//! ## Features
//!
//! - Copy static assets to public directory
//! - Content-based hashing for cache busting
//! - Asset manifest generation
//! - Recursive directory processing
//! - File filtering and exclusion

pub mod command;
pub mod hasher;
pub mod manifest;
pub mod publisher;

pub use command::AssetPublishCommand;
pub use hasher::AssetHasher;
pub use manifest::{AssetManifest, AssetEntry};
pub use publisher::{AssetPublisher, PublishConfig, PublishResult};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
