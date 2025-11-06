//! Database seeders for test data

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for database seeders
#[async_trait]
pub trait Seeder: Send + Sync {
    /// Run the seeder
    async fn run(&self) -> anyhow::Result<()>;

    /// Get seeder name
    fn name(&self) -> &str;

    /// Dependencies (other seeders that must run first)
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Test seeder manager
pub struct TestSeeder {
    seeders: HashMap<String, Box<dyn Seeder>>,
    executed: Vec<String>,
}

impl TestSeeder {
    pub fn new() -> Self {
        Self {
            seeders: HashMap::new(),
            executed: Vec::new(),
        }
    }

    pub fn register(&mut self, seeder: Box<dyn Seeder>) {
        let name = seeder.name().to_string();
        self.seeders.insert(name, seeder);
    }

    pub async fn run_all(&mut self) -> anyhow::Result<()> {
        let names: Vec<String> = self.seeders.keys().cloned().collect();
        for name in names {
            self.run_seeder(&name).await?;
        }
        Ok(())
    }

    pub async fn run_seeder(&mut self, name: &str) -> anyhow::Result<()> {
        use std::pin::Pin;
        use std::future::Future;

        Box::pin(async move {
            if self.executed.contains(&name.to_string()) {
                return Ok(());
            }

            // Get dependencies first (before borrowing seeder for run)
            let deps = {
                let seeder = self
                    .seeders
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("Seeder '{}' not found", name))?;
                seeder.dependencies()
            };

            // Run dependencies first
            for dep in deps {
                self.run_seeder(&dep).await?;
            }

            // Run the seeder
            let seeder = self
                .seeders
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("Seeder '{}' not found", name))?;
            seeder.run().await?;
            self.executed.push(name.to_string());

            Ok(())
        }).await
    }

    pub fn reset(&mut self) {
        self.executed.clear();
    }
}

impl Default for TestSeeder {
    fn default() -> Self {
        Self::new()
    }
}

/// Example user seeder
pub struct UserSeeder {
    count: usize,
}

impl UserSeeder {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

#[async_trait]
impl Seeder for UserSeeder {
    async fn run(&self) -> anyhow::Result<()> {
        use crate::factory::{Factory, UserFactory};

        let factory = UserFactory;
        let users = factory.create_many(self.count).await?;

        println!("Created {} test users", users.len());
        Ok(())
    }

    fn name(&self) -> &str {
        "users"
    }
}

/// Example post seeder with dependencies
pub struct PostSeeder {
    count: usize,
}

impl PostSeeder {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

#[async_trait]
impl Seeder for PostSeeder {
    async fn run(&self) -> anyhow::Result<()> {
        use crate::factory::{Factory, PostFactory};

        let factory = PostFactory::new();
        let posts = factory.create_many(self.count).await?;

        println!("Created {} test posts", posts.len());
        Ok(())
    }

    fn name(&self) -> &str {
        "posts"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["users".to_string()]
    }
}

/// Seeder builder for fluent API
pub struct SeederBuilder {
    name: String,
    run_fn: Option<Box<dyn Fn() -> anyhow::Result<()> + Send + Sync>>,
    dependencies: Vec<String>,
}

impl SeederBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            run_fn: None,
            dependencies: Vec::new(),
        }
    }

    pub fn run<F>(mut self, f: F) -> Self
    where
        F: Fn() -> anyhow::Result<()> + Send + Sync + 'static,
    {
        self.run_fn = Some(Box::new(f));
        self
    }

    pub fn depends_on(mut self, seeder: impl Into<String>) -> Self {
        self.dependencies.push(seeder.into());
        self
    }
}
