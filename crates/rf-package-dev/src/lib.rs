//! Package development tools for RustForge
//!
//! This crate provides utilities for developing RustForge packages, similar to Laravel's
//! package development features. It includes:
//!
//! - Package structure generation
//! - Auto-discovery mechanisms
//! - Asset publishing (configs, views, migrations)
//! - Service provider integration
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use rf_package_dev::{Package, AssetType};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let package = Package::new("my-awesome-package")
//!     .description("An awesome RustForge package")
//!     .author("Your Name <you@example.com>")
//!     .config("config/my-package.toml")
//!     .migration("create_my_table")
//!     .view("templates/my-package")
//!     .build()?;
//!
//! // Publish assets
//! package.publish(AssetType::Config, "target/config").await?;
//! package.publish(AssetType::Migrations, "target/migrations").await?;
//! # Ok(())
//! # }
//! ```

mod discovery;
mod package;
mod publishing;

pub use discovery::{Discovery, DiscoveryConfig};
pub use package::{Package, PackageBuilder, PackageError, PackageResult};
pub use publishing::{AssetType, PublishConfig, Publisher};
