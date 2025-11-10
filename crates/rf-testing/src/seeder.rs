//! Database seeding for tests
//!
//! Seeders provide a way to populate test databases with sample data.

use async_trait::async_trait;
use std::sync::Arc;

/// Seeder trait for populating test data
#[async_trait]
pub trait Seeder: Send + Sync {
    /// Run the seeder
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Database seeder orchestrator
pub struct DatabaseSeeder {
    seeders: Vec<Arc<dyn Seeder>>,
}

impl DatabaseSeeder {
    /// Create a new database seeder
    pub fn new() -> Self {
        Self {
            seeders: Vec::new(),
        }
    }

    /// Add a seeder
    pub fn add<S: Seeder + 'static>(mut self, seeder: S) -> Self {
        self.seeders.push(Arc::new(seeder));
        self
    }

    /// Run all seeders
    pub async fn run_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for seeder in &self.seeders {
            seeder.run().await?;
        }
        Ok(())
    }

    /// Run a specific seeder by index
    pub async fn run_one(&self, index: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(seeder) = self.seeders.get(index) {
            seeder.run().await?;
        }
        Ok(())
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
            async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        // Should not panic
        db_seeder.run_one(99).await.unwrap();
    }
}
