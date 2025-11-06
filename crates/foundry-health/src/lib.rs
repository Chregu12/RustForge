//! # Foundry Health Checks
//!
//! Comprehensive health check and diagnostics system for Foundry applications.
//!
//! ## Features
//!
//! - System diagnostics (CPU, memory, disk space)
//! - Rust version verification
//! - Database connectivity checks
//! - Cache connectivity checks
//! - Environment validation
//! - File permissions verification
//! - Migration status checks
//! - Parallel execution for fast results
//!
//! ## Usage
//!
//! ```rust,no_run
//! use foundry_health::{HealthChecker, HealthCheckConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = HealthCheckConfig::default();
//! let checker = HealthChecker::new(config);
//! let results = checker.check_all().await?;
//!
//! for result in results {
//!     println!("{}: {}", result.name, result.status);
//! }
//! # Ok(())
//! # }
//! ```

pub mod checks;
pub mod command;
pub mod config;
pub mod report;

pub use checks::*;
pub use command::HealthCheckCommand;
pub use config::HealthCheckConfig;
pub use report::{CheckResult, CheckStatus, HealthReport};

use anyhow::Result;

/// Main health checker
pub struct HealthChecker {
    config: HealthCheckConfig,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: HealthCheckConfig) -> Self {
        Self { config }
    }

    /// Run all health checks in parallel
    pub async fn check_all(&self) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();

        // Create check instances outside of tokio::join! to avoid lifetime issues
        let env_check = EnvCheck::new(self.config.required_env_vars.clone());
        let files_check = FilePermissionsCheck::new(self.config.required_files.clone());

        // Run checks in parallel
        let (rust, disk, memory, env, files) = tokio::join!(
            RustVersionCheck.run(),
            DiskSpaceCheck.run(),
            MemoryCheck.run(),
            env_check.run(),
            files_check.run(),
        );

        results.push(rust);
        results.push(disk);
        results.push(memory);
        results.push(env);
        results.push(files);

        // Add database check if configured
        if let Some(db_url) = &self.config.database_url {
            results.push(DatabaseCheck::new(db_url.clone()).run().await);
        }

        // Add cache check if configured
        if let Some(cache_url) = &self.config.cache_url {
            results.push(CacheCheck::new(cache_url.clone()).run().await);
        }

        Ok(results)
    }

    /// Run a single check by name
    pub async fn check_one(&self, name: &str) -> Result<CheckResult> {
        match name {
            "rust" => Ok(RustVersionCheck.run().await),
            "disk" => Ok(DiskSpaceCheck.run().await),
            "memory" => Ok(MemoryCheck.run().await),
            "env" => Ok(EnvCheck::new(self.config.required_env_vars.clone()).run().await),
            "files" => Ok(FilePermissionsCheck::new(self.config.required_files.clone()).run().await),
            "database" => {
                if let Some(db_url) = &self.config.database_url {
                    Ok(DatabaseCheck::new(db_url.clone()).run().await)
                } else {
                    Ok(CheckResult::skipped("database", "No database URL configured"))
                }
            }
            "cache" => {
                if let Some(cache_url) = &self.config.cache_url {
                    Ok(CacheCheck::new(cache_url.clone()).run().await)
                } else {
                    Ok(CheckResult::skipped("cache", "No cache URL configured"))
                }
            }
            _ => anyhow::bail!("Unknown check: {}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_checker_new() {
        let config = HealthCheckConfig::default();
        let checker = HealthChecker::new(config);
        assert!(checker.config.required_env_vars.is_empty());
    }

    #[tokio::test]
    async fn test_check_all() {
        let config = HealthCheckConfig::default();
        let checker = HealthChecker::new(config);
        let results = checker.check_all().await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_check_one() {
        let config = HealthCheckConfig::default();
        let checker = HealthChecker::new(config);

        let result = checker.check_one("rust").await.unwrap();
        assert_eq!(result.name, "rust");
    }
}
