//! **EXPERIMENTAL — not part of the RustForge 1.0 supported surface; API may change without a SemVer bump.**
//!
//! # rf-telescope
//!
//! Laravel Telescope equivalent for Rust - Debugging dashboard for request/query monitoring.
//!
//! ## Features
//!
//! - Request monitoring with detailed information
//! - Database query logging and analysis
//! - Exception tracking with stack traces
//! - Job monitoring
//! - Email preview
//! - Web dashboard
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_telescope::Telescope;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let telescope = Telescope::new()
//!         .watch_requests()
//!         .watch_queries()
//!         .watch_exceptions()
//!         .enabled_in_production(false);
//!
//!     // Use as middleware
//!     // app.layer(telescope.middleware());
//!
//!     Ok(())
//! }
//! ```

pub mod dashboard;
pub mod middleware;
pub mod storage;
pub mod watchers;

pub use dashboard::Dashboard;
pub use middleware::telescope_layer;
pub use storage::{Entry, EntryType, Storage};
pub use watchers::{
    cache::CacheWatcher, exception::ExceptionWatcher, job::JobWatcher, mail::MailWatcher,
    query::QueryWatcher, request::RequestWatcher,
};

use anyhow::Result;
use std::sync::Arc;

/// Main Telescope instance for application monitoring
#[derive(Clone)]
pub struct Telescope {
    config: Arc<TelescopeConfig>,
    storage: Storage,
}

/// Telescope configuration
#[derive(Debug, Clone)]
pub struct TelescopeConfig {
    pub enabled: bool,
    pub watch_requests: bool,
    pub watch_queries: bool,
    pub watch_exceptions: bool,
    pub watch_jobs: bool,
    pub watch_mail: bool,
    pub enabled_in_production: bool,
    pub retention_hours: u32,
}

impl Default for TelescopeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            watch_requests: false,
            watch_queries: false,
            watch_exceptions: false,
            watch_jobs: false,
            watch_mail: false,
            enabled_in_production: false,
            retention_hours: 24,
        }
    }
}

impl Telescope {
    /// Create a new Telescope instance
    pub fn new() -> Self {
        Self {
            config: Arc::new(TelescopeConfig::default()),
            storage: Storage::new(),
        }
    }

    /// Enable request watching
    pub fn watch_requests(mut self) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.watch_requests = true;
        self
    }

    /// Enable query watching
    pub fn watch_queries(mut self) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.watch_queries = true;
        self
    }

    /// Enable exception watching
    pub fn watch_exceptions(mut self) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.watch_exceptions = true;
        self
    }

    /// Enable job watching
    pub fn watch_jobs(mut self) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.watch_jobs = true;
        self
    }

    /// Enable mail watching
    pub fn watch_mail(mut self) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.watch_mail = true;
        self
    }

    /// Set whether Telescope is enabled in production
    pub fn enabled_in_production(mut self, enabled: bool) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.enabled_in_production = enabled;
        self
    }

    /// Set data retention period in hours
    pub fn retention_hours(mut self, hours: u32) -> Self {
        let config = Arc::make_mut(&mut self.config);
        config.retention_hours = hours;
        self
    }

    /// Get the storage instance
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Start the Telescope dashboard server
    pub async fn serve(self, addr: &str) -> Result<()> {
        dashboard::serve(self, addr).await
    }

    /// Check if Telescope is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Create middleware for Axum
    pub fn middleware(&self) -> TelescopeMiddleware {
        TelescopeMiddleware {
            telescope: self.clone(),
        }
    }
}

impl Default for Telescope {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware for integrating Telescope with Axum
#[derive(Clone)]
pub struct TelescopeMiddleware {
    telescope: Telescope,
}

impl TelescopeMiddleware {
    /// Get the underlying Telescope instance
    pub fn telescope(&self) -> &Telescope {
        &self.telescope
    }
}
