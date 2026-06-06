//! # rf-seeder
//!
//! Database seeding for RustForge - Laravel-style seeders.
//!
//! ## Example
//!
//! ```rust,ignore
//! use rf_seeder::{Seeder, SeederRunner};
//!
//! struct UserSeeder;
//!
//! #[async_trait::async_trait]
//! impl Seeder for UserSeeder {
//!     async fn run(&self) -> Result<(), SeederError> {
//!         // Insert seed data
//!         Ok(())
//!     }
//! }
//! ```

pub mod factory;

pub use factory::{FactoryBuilder, ModelFactory};

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Error type for seeder operations
#[derive(Debug, Clone)]
pub struct SeederError {
    pub message: String,
}

impl std::fmt::Display for SeederError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SeederError: {}", self.message)
    }
}

impl std::error::Error for SeederError {}

impl SeederError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Trait for database seeders - Laravel style
#[async_trait]
pub trait Seeder: Send + Sync {
    /// Run the seeder
    async fn run(&self) -> Result<(), SeederError>;

    /// Get the seeder name
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Seeders that should run before this one
    fn depends_on(&self) -> Vec<&str> {
        vec![]
    }
}

/// Seeder runner that manages and executes seeders
pub struct SeederRunner {
    seeders: HashMap<String, Arc<dyn Seeder>>,
    executed: Vec<String>,
}

impl Default for SeederRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SeederRunner {
    /// Create a new seeder runner
    pub fn new() -> Self {
        Self {
            seeders: HashMap::new(),
            executed: Vec::new(),
        }
    }

    /// Register a seeder
    pub fn register<S: Seeder + 'static>(&mut self, seeder: S) -> &mut Self {
        let name = seeder.name().to_string();
        self.seeders.insert(name, Arc::new(seeder));
        self
    }

    /// Run all registered seeders
    pub async fn run_all(&mut self) -> Result<(), SeederError> {
        let names: Vec<String> = self.seeders.keys().cloned().collect();

        for name in names {
            self.run_seeder(&name).await?;
        }

        Ok(())
    }

    /// Run a specific seeder by name
    pub fn run_seeder<'a>(
        &'a mut self,
        name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SeederError>> + Send + 'a>>
    {
        Box::pin(async move {
            // Skip if already executed
            if self.executed.contains(&name.to_string()) {
                return Ok(());
            }

            let seeder = self
                .seeders
                .get(name)
                .ok_or_else(|| SeederError::new(format!("Seeder '{}' not found", name)))?
                .clone();

            // Run dependencies first
            for dep in seeder.depends_on() {
                self.run_seeder(dep).await?;
            }

            // Run the seeder
            tracing::info!("Running seeder: {}", name);
            seeder.run().await?;
            self.executed.push(name.to_string());
            tracing::info!("Completed seeder: {}", name);

            Ok(())
        })
    }

    /// Get list of executed seeders
    pub fn executed(&self) -> &[String] {
        &self.executed
    }

    /// Reset executed state (for re-running)
    pub fn reset(&mut self) {
        self.executed.clear();
    }
}

/// Database seeder trait for calling other seeders
#[async_trait]
pub trait DatabaseSeeder: Seeder {
    /// Call another seeder
    async fn call<S: Seeder + Default + 'static>(&self) -> Result<(), SeederError> {
        let seeder = S::default();
        seeder.run().await
    }
}

/// Factory trait for generating fake data
pub trait Factory {
    type Model;

    /// Create a single instance
    fn make(&self) -> Self::Model;

    /// Create multiple instances
    fn count(&self, n: usize) -> Vec<Self::Model> {
        (0..n).map(|_| self.make()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSeeder;

    #[async_trait]
    impl Seeder for TestSeeder {
        async fn run(&self) -> Result<(), SeederError> {
            Ok(())
        }

        fn name(&self) -> &str {
            "TestSeeder"
        }
    }

    #[tokio::test]
    async fn test_seeder_runner() {
        let mut runner = SeederRunner::new();
        runner.register(TestSeeder);

        let result = runner.run_all().await;
        assert!(result.is_ok());
        assert_eq!(runner.executed().len(), 1);
    }

    #[tokio::test]
    async fn test_seeder_reset() {
        let mut runner = SeederRunner::new();
        runner.register(TestSeeder);

        runner.run_all().await.unwrap();
        assert_eq!(runner.executed().len(), 1);

        runner.reset();
        assert_eq!(runner.executed().len(), 0);
    }
}
