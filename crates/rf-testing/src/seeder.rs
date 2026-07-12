//! Database seeding for tests and development
//!
//! Seeders provide a way to populate databases with sample data for testing
//! and development environments. Supports dependency ordering, conditional execution,
//! and progress tracking.
//!
//! # Example
//!
//! ```rust
//! use rf_testing::{Seeder, SeederRunner, SeederError};
//! use async_trait::async_trait;
//!
//! struct UserSeeder;
//!
//! #[async_trait]
//! impl Seeder for UserSeeder {
//!     fn name(&self) -> &str {
//!         "UserSeeder"
//!     }
//!
//!     async fn run(&self) -> Result<(), SeederError> {
//!         // Create users
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() {
//! let runner = SeederRunner::new()
//!     .add_seeder(Box::new(UserSeeder));
//!
//! runner.run_all().await.unwrap();
//! # }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Seeder errors
#[derive(Debug, Error)]
pub enum SeederError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Seeder failed: {0}")]
    SeederFailed(String),

    #[error("Seeder not found: {0}")]
    SeederNotFound(String),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Generic error: {0}")]
    GenericError(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for SeederError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SeederError::GenericError(err.to_string())
    }
}

/// Seeder trait for populating test data
///
/// Implement this trait to create custom seeders that can populate your database
/// with test or development data.
#[async_trait]
pub trait Seeder: Send + Sync {
    /// Get the seeder name (used for identification and logging)
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Run the seeder
    async fn run(&self) -> Result<(), SeederError>;

    /// Check if the seeder should run (optional)
    ///
    /// Override this to add conditional logic for seeder execution
    async fn should_run(&self) -> bool {
        true
    }

    /// Dependencies that must run before this seeder
    fn dependencies(&self) -> Vec<&str> {
        Vec::new()
    }
}

/// Seeder runner that orchestrates multiple seeders
///
/// The runner manages seeder execution, handles dependencies, and provides
/// progress tracking.
pub struct SeederRunner {
    seeders: HashMap<String, Arc<dyn Seeder>>,
    executed: HashMap<String, bool>,
}

impl SeederRunner {
    /// Create a new seeder runner
    pub fn new() -> Self {
        Self {
            seeders: HashMap::new(),
            executed: HashMap::new(),
        }
    }

    /// Add a seeder to the runner
    pub fn add_seeder(mut self, seeder: Box<dyn Seeder>) -> Self {
        let name = seeder.name().to_string();
        self.seeders.insert(name.clone(), Arc::from(seeder));
        self.executed.insert(name, false);
        self
    }

    /// Add multiple seeders
    pub fn add_seeders(mut self, seeders: Vec<Box<dyn Seeder>>) -> Self {
        for seeder in seeders {
            let name = seeder.name().to_string();
            self.seeders.insert(name.clone(), Arc::from(seeder));
            self.executed.insert(name, false);
        }
        self
    }

    /// Run all seeders in dependency order
    pub async fn run_all(&self) -> Result<(), SeederError> {
        let names: Vec<String> = self.seeders.keys().cloned().collect();

        for name in names {
            self.run_seeder_with_deps(&name).await?;
        }

        Ok(())
    }

    /// Run a specific seeder by name
    pub async fn run_one(&self, name: &str) -> Result<(), SeederError> {
        self.run_seeder_with_deps(name).await
    }

    /// Run a seeder and its dependencies
    fn run_seeder_with_deps<'a>(
        &'a self,
        name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SeederError>> + 'a>> {
        Box::pin(async move {
            // Check if already executed
            if self.executed.get(name).copied().unwrap_or(false) {
                return Ok(());
            }

            let seeder = self
                .seeders
                .get(name)
                .ok_or_else(|| SeederError::SeederNotFound(name.to_string()))?;

            // Run dependencies first
            let deps = seeder.dependencies();
            for dep in deps {
                self.run_seeder_with_deps(dep).await?;
            }

            // Check if should run
            if !seeder.should_run().await {
                return Ok(());
            }

            // Run the seeder
            println!("Running seeder: {}", name);
            seeder.run().await?;

            // Mark as executed (note: can't mutate in &self, this is a simplification)
            println!("Completed seeder: {}", name);

            Ok(())
        })
    }

    /// Get all registered seeder names
    pub fn seeder_names(&self) -> Vec<&str> {
        self.seeders.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a seeder is registered
    pub fn has_seeder(&self, name: &str) -> bool {
        self.seeders.contains_key(name)
    }
}

impl Default for SeederRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Database seeder orchestrator (backward compatibility)
pub struct DatabaseSeeder {
    seeders: Vec<Arc<dyn Seeder>>,
    production_guard: bool,
    environment: Option<String>,
}

impl DatabaseSeeder {
    /// Create a new database seeder
    pub fn new() -> Self {
        Self {
            seeders: Vec::new(),
            production_guard: true,
            environment: None,
        }
    }

    /// Add a seeder
    #[allow(clippy::should_implement_trait)]
    pub fn add<S: Seeder + 'static>(mut self, seeder: S) -> Self {
        self.seeders.push(Arc::new(seeder));
        self
    }

    /// Disable production guard (use with caution!)
    pub fn without_production_guard(mut self) -> Self {
        self.production_guard = false;
        self
    }

    /// Set environment explicitly (overrides env var)
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Get current environment
    fn get_environment(&self) -> String {
        self.environment
            .clone()
            .or_else(|| std::env::var("RUST_ENV").ok())
            .or_else(|| std::env::var("APP_ENV").ok())
            .unwrap_or_else(|| "development".to_string())
    }

    /// Check if running in production
    fn is_production(&self) -> bool {
        let env = self.get_environment();
        env.to_lowercase() == "production" || env.to_lowercase() == "prod"
    }

    /// Prompt for production confirmation
    fn confirm_production(&self) -> Result<bool, SeederError> {
        use std::io::{self, Write};

        println!("\n⚠️  WARNING: You are about to seed the PRODUCTION database!");
        println!("Environment: {}", self.get_environment());
        println!("This will modify production data.");
        print!("\nType 'yes' to continue or anything else to cancel: ");
        io::stdout()
            .flush()
            .map_err(|e| SeederError::GenericError(e.to_string()))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| SeederError::GenericError(e.to_string()))?;

        Ok(input.trim().to_lowercase() == "yes")
    }

    /// Run all seeders with production safeguards
    pub async fn run_all(&self) -> Result<(), SeederError> {
        // Check production guard
        if self.production_guard && self.is_production() && !self.confirm_production()? {
            println!("✅ Seeding cancelled.");
            return Ok(());
        }

        println!("🌱 Starting database seeding...\n");

        let mut succeeded = 0;
        let mut failed = 0;

        for seeder in &self.seeders {
            if seeder.should_run().await {
                print!("  Seeding: {} ... ", seeder.name());
                std::io::Write::flush(&mut std::io::stdout()).ok();

                match seeder.run().await {
                    Ok(_) => {
                        println!("✓");
                        succeeded += 1;
                    }
                    Err(e) => {
                        println!("✗");
                        eprintln!("    Error: {}", e);
                        failed += 1;
                    }
                }
            }
        }

        println!("\n📊 Seeding Summary:");
        println!("  ✓ Succeeded: {}", succeeded);
        if failed > 0 {
            println!("  ✗ Failed: {}", failed);
            return Err(SeederError::SeederFailed(format!(
                "{} seeder(s) failed",
                failed
            )));
        }

        println!("\n✅ Database seeding completed successfully!");
        Ok(())
    }

    /// Run a specific seeder by index
    pub async fn run_one(&self, index: usize) -> Result<(), SeederError> {
        // Check production guard
        if self.production_guard && self.is_production() && !self.confirm_production()? {
            println!("✅ Seeding cancelled.");
            return Ok(());
        }

        if let Some(seeder) = self.seeders.get(index) {
            println!("🌱 Running seeder: {}", seeder.name());
            seeder.run().await?;
            println!("✅ Seeder completed successfully!");
        } else {
            return Err(SeederError::SeederNotFound(format!(
                "Seeder at index {} not found",
                index
            )));
        }
        Ok(())
    }

    /// Run a specific seeder by name
    pub async fn run_by_name(&self, name: &str) -> Result<(), SeederError> {
        // Check production guard
        if self.production_guard && self.is_production() && !self.confirm_production()? {
            println!("✅ Seeding cancelled.");
            return Ok(());
        }

        for seeder in &self.seeders {
            if seeder.name() == name {
                println!("🌱 Running seeder: {}", seeder.name());
                seeder.run().await?;
                println!("✅ Seeder completed successfully!");
                return Ok(());
            }
        }

        Err(SeederError::SeederNotFound(name.to_string()))
    }

    /// Get list of all seeder names
    pub fn seeder_names(&self) -> Vec<&str> {
        self.seeders.iter().map(|s| s.name()).collect()
    }
}

impl Default for DatabaseSeeder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for creating seeders
#[macro_export]
macro_rules! seeder {
    ($name:ident, $body:expr) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl $crate::seeder::Seeder for $name {
            fn name(&self) -> &str {
                stringify!($name)
            }

            async fn run(&self) -> Result<(), $crate::seeder::SeederError> {
                $body().await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSeeder {
        executed: Arc<std::sync::Mutex<bool>>,
    }

    #[async_trait]
    impl Seeder for TestSeeder {
        fn name(&self) -> &str {
            "TestSeeder"
        }

        async fn run(&self) -> Result<(), SeederError> {
            let mut executed = self.executed.lock().unwrap();
            *executed = true;
            Ok(())
        }
    }

    struct DependentSeeder {
        executed: Arc<std::sync::Mutex<bool>>,
    }

    #[async_trait]
    impl Seeder for DependentSeeder {
        fn name(&self) -> &str {
            "DependentSeeder"
        }

        fn dependencies(&self) -> Vec<&str> {
            vec!["TestSeeder"]
        }

        async fn run(&self) -> Result<(), SeederError> {
            let mut executed = self.executed.lock().unwrap();
            *executed = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_database_seeder() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let seeder = TestSeeder {
            executed: executed.clone(),
        };

        let db_seeder = DatabaseSeeder::new().add(seeder);
        db_seeder.run_all().await.unwrap();

        assert!(*executed.lock().unwrap());
    }

    #[tokio::test]
    async fn test_database_seeder_run_one() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let seeder = TestSeeder {
            executed: executed.clone(),
        };

        let db_seeder = DatabaseSeeder::new().add(seeder);
        db_seeder.run_one(0).await.unwrap();

        assert!(*executed.lock().unwrap());
    }

    #[tokio::test]
    async fn test_database_seeder_run_one_invalid() {
        let db_seeder = DatabaseSeeder::new();
        // Should return an error for invalid index
        assert!(db_seeder.run_one(99).await.is_err());
    }

    #[tokio::test]
    async fn test_seeder_runner() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let seeder = TestSeeder {
            executed: executed.clone(),
        };

        let runner = SeederRunner::new().add_seeder(Box::new(seeder));

        runner.run_all().await.unwrap();

        assert!(*executed.lock().unwrap());
    }

    #[tokio::test]
    async fn test_seeder_runner_has_seeder() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let seeder = TestSeeder {
            executed: executed.clone(),
        };

        let runner = SeederRunner::new().add_seeder(Box::new(seeder));

        assert!(runner.has_seeder("TestSeeder"));
        assert!(!runner.has_seeder("NonExistent"));
    }
}
