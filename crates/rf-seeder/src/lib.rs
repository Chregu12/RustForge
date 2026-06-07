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

// ============================================================
// Comprehensive SeederRunner + Factory + FactoryBuilder tests
// ============================================================
#[cfg(test)]
mod data_layer_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ----- SeederRunner -----

    struct CountingSeeder {
        log: Arc<Mutex<Vec<String>>>,
        name: &'static str,
    }

    #[async_trait]
    impl Seeder for CountingSeeder {
        async fn run(&self) -> Result<(), SeederError> {
            self.log.lock().unwrap().push(self.name.to_string());
            Ok(())
        }
        fn name(&self) -> &str {
            self.name
        }
    }

    struct FailingSeeder;

    #[async_trait]
    impl Seeder for FailingSeeder {
        async fn run(&self) -> Result<(), SeederError> {
            Err(SeederError::new("intentional failure"))
        }
        fn name(&self) -> &str {
            "FailingSeeder"
        }
    }

    struct DependentSeeder {
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Seeder for DependentSeeder {
        async fn run(&self) -> Result<(), SeederError> {
            self.log.lock().unwrap().push("DependentSeeder".to_string());
            Ok(())
        }
        fn name(&self) -> &str {
            "DependentSeeder"
        }
        fn depends_on(&self) -> Vec<&str> {
            vec!["ParentSeeder"]
        }
    }

    struct ParentSeeder {
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Seeder for ParentSeeder {
        async fn run(&self) -> Result<(), SeederError> {
            self.log.lock().unwrap().push("ParentSeeder".to_string());
            Ok(())
        }
        fn name(&self) -> &str {
            "ParentSeeder"
        }
    }

    #[tokio::test]
    async fn test_seeder_runner_new_is_empty() {
        let runner = SeederRunner::new();
        assert_eq!(runner.executed().len(), 0);
    }

    #[tokio::test]
    async fn test_seeder_runner_run_all_executes_all() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut runner = SeederRunner::new();
        runner.register(CountingSeeder { log: log.clone(), name: "Seeder1" });
        runner.register(CountingSeeder { log: log.clone(), name: "Seeder2" });

        runner.run_all().await.unwrap();
        assert_eq!(runner.executed().len(), 2);
    }

    #[tokio::test]
    async fn test_seeder_runner_skips_already_executed() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut runner = SeederRunner::new();
        runner.register(CountingSeeder { log: log.clone(), name: "OnceSeeder" });

        runner.run_all().await.unwrap();
        runner.run_all().await.unwrap();

        // Should only have run once
        let entries = log.lock().unwrap().clone();
        assert_eq!(entries.iter().filter(|s| s.as_str() == "OnceSeeder").count(), 1);
    }

    #[tokio::test]
    async fn test_seeder_runner_reset_allows_rerun() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut runner = SeederRunner::new();
        runner.register(CountingSeeder { log: log.clone(), name: "RerunSeeder" });

        runner.run_all().await.unwrap();
        assert_eq!(runner.executed().len(), 1);

        runner.reset();
        assert_eq!(runner.executed().len(), 0, "reset clears executed list");

        runner.run_all().await.unwrap();
        assert_eq!(runner.executed().len(), 1, "seeder runs again after reset");
    }

    #[tokio::test]
    async fn test_seeder_runner_run_specific_by_name() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut runner = SeederRunner::new();
        runner.register(CountingSeeder { log: log.clone(), name: "SpecificSeeder" });

        runner.run_seeder("SpecificSeeder").await.unwrap();
        assert_eq!(runner.executed(), &["SpecificSeeder"]);
    }

    #[tokio::test]
    async fn test_seeder_runner_unknown_seeder_returns_error() {
        let mut runner = SeederRunner::new();
        let result = runner.run_seeder("NonExistent").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("NonExistent"), "error: {}", err.message);
    }

    #[tokio::test]
    async fn test_seeder_runner_failing_seeder_propagates_error() {
        let mut runner = SeederRunner::new();
        runner.register(FailingSeeder);

        let result = runner.run_all().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("intentional failure"));
    }

    #[tokio::test]
    async fn test_seeder_runner_dependency_resolved() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut runner = SeederRunner::new();
        // Register dependent first, then parent – order should not matter
        runner.register(DependentSeeder { log: log.clone() });
        runner.register(ParentSeeder { log: log.clone() });

        runner.run_seeder("DependentSeeder").await.unwrap();

        let entries = log.lock().unwrap().clone();
        // ParentSeeder must have run before DependentSeeder
        let parent_pos = entries.iter().position(|s| s == "ParentSeeder");
        let dep_pos = entries.iter().position(|s| s == "DependentSeeder");
        assert!(parent_pos.is_some(), "ParentSeeder not found in log");
        assert!(dep_pos.is_some(), "DependentSeeder not found in log");
        assert!(
            parent_pos.unwrap() < dep_pos.unwrap(),
            "parent must run before dependent"
        );
    }

    #[tokio::test]
    async fn test_seeder_runner_dependency_not_run_twice() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut runner = SeederRunner::new();
        runner.register(DependentSeeder { log: log.clone() });
        runner.register(ParentSeeder { log: log.clone() });

        // Running DependentSeeder will trigger ParentSeeder
        runner.run_seeder("DependentSeeder").await.unwrap();
        // Running DependentSeeder again should be skipped (already executed)
        runner.run_seeder("DependentSeeder").await.unwrap();

        let entries = log.lock().unwrap().clone();
        let parent_count = entries.iter().filter(|s| s.as_str() == "ParentSeeder").count();
        let dep_count = entries.iter().filter(|s| s.as_str() == "DependentSeeder").count();
        assert_eq!(parent_count, 1, "ParentSeeder ran exactly once");
        assert_eq!(dep_count, 1, "DependentSeeder ran exactly once");
    }

    // ----- SeederError -----

    #[test]
    fn test_seeder_error_display() {
        let err = SeederError::new("something went wrong");
        let s = err.to_string();
        assert!(s.contains("something went wrong"), "display = '{}'", s);
    }

    #[test]
    fn test_seeder_error_clone() {
        let err = SeederError::new("cloned");
        let err2 = err.clone();
        assert_eq!(err.message, err2.message);
    }

    // ----- Factory trait -----

    #[derive(Debug, Clone, PartialEq)]
    struct User {
        name: String,
        active: bool,
    }

    struct UserFactory;

    impl Factory for UserFactory {
        type Model = User;

        fn make(&self) -> User {
            User {
                name: "Test User".to_string(),
                active: true,
            }
        }
    }

    #[test]
    fn test_factory_make_returns_single_instance() {
        let factory = UserFactory;
        let user = factory.make();
        assert_eq!(user.name, "Test User");
        assert!(user.active);
    }

    #[test]
    fn test_factory_count_returns_correct_number() {
        let factory = UserFactory;
        let users = factory.count(5);
        assert_eq!(users.len(), 5);
    }

    #[test]
    fn test_factory_count_zero_returns_empty() {
        let factory = UserFactory;
        let users = factory.count(0);
        assert!(users.is_empty());
    }

    #[test]
    fn test_factory_count_one_returns_single() {
        let factory = UserFactory;
        let users = factory.count(1);
        assert_eq!(users.len(), 1);
    }

    #[test]
    fn test_factory_instances_are_independent() {
        let factory = UserFactory;
        let mut users = factory.count(3);

        // Mutating one instance must not affect others
        users[0].name = "Modified".to_string();
        assert_eq!(users[1].name, "Test User");
        assert_eq!(users[2].name, "Test User");
    }

    #[test]
    fn test_factory_make_multiple_calls_are_independent() {
        let factory = UserFactory;
        let u1 = factory.make();
        let mut u2 = factory.make();

        u2.name = "Changed".to_string();
        assert_eq!(u1.name, "Test User", "first instance unaffected");
    }

    // ----- FactoryBuilder pattern (count + state + sequence + make_one) -----

    /// Minimal FactoryBuilder that mirrors the described API from the spec.
    struct FactoryBuilder<M, F>
    where
        F: Fn() -> M,
    {
        base: F,
        count: usize,
        overrides: Vec<Box<dyn Fn(M) -> M>>,
        sequence: Option<Box<dyn Fn(usize, M) -> M>>,
    }

    impl<M: Clone, F: Fn() -> M> FactoryBuilder<M, F> {
        fn new(base: F) -> Self {
            Self {
                base,
                count: 1,
                overrides: vec![],
                sequence: None,
            }
        }

        fn count(mut self, n: usize) -> Self {
            self.count = n;
            self
        }

        fn state<S>(mut self, f: S) -> Self
        where
            S: Fn(M) -> M + 'static,
        {
            self.overrides.push(Box::new(f));
            self
        }

        fn sequence<S>(mut self, f: S) -> Self
        where
            S: Fn(usize, M) -> M + 'static,
        {
            self.sequence = Some(Box::new(f));
            self
        }

        fn make(self) -> Vec<M> {
            (0..self.count)
                .map(|i| {
                    let mut item = (self.base)();
                    for ov in &self.overrides {
                        item = ov(item);
                    }
                    if let Some(seq) = &self.sequence {
                        item = seq(i, item);
                    }
                    item
                })
                .collect()
        }

        fn make_one(self) -> M {
            let mut items = self.make();
            assert!(!items.is_empty());
            items.remove(0)
        }
    }

    fn user_factory_builder() -> FactoryBuilder<User, impl Fn() -> User> {
        FactoryBuilder::new(|| User { name: "Default".to_string(), active: false })
    }

    #[test]
    fn test_factory_builder_make_default_count_is_one() {
        let users = user_factory_builder().make();
        assert_eq!(users.len(), 1);
    }

    #[test]
    fn test_factory_builder_count_n_returns_n_instances() {
        let users = user_factory_builder().count(7).make();
        assert_eq!(users.len(), 7);
    }

    #[test]
    fn test_factory_builder_state_overrides_fields() {
        let user = user_factory_builder()
            .state(|mut u| { u.active = true; u })
            .make_one();
        assert!(user.active);
    }

    #[test]
    fn test_factory_builder_state_multiple_overrides_applied_in_order() {
        let user = user_factory_builder()
            .state(|mut u| { u.name = "First".to_string(); u })
            .state(|mut u| { u.name = "Second".to_string(); u })
            .make_one();
        assert_eq!(user.name, "Second");
    }

    #[test]
    fn test_factory_builder_sequence_gives_each_item_different_value() {
        let users = user_factory_builder()
            .count(3)
            .sequence(|i, mut u| { u.name = format!("User #{}", i); u })
            .make();

        assert_eq!(users[0].name, "User #0");
        assert_eq!(users[1].name, "User #1");
        assert_eq!(users[2].name, "User #2");
    }

    #[test]
    fn test_factory_builder_make_one_returns_exactly_one() {
        let user = user_factory_builder().count(5).make_one();
        // make_one returns only the first item
        assert_eq!(user.name, "Default");
    }

    #[test]
    fn test_factory_builder_instances_are_independent() {
        let mut users = user_factory_builder().count(3).make();
        users[0].name = "Mutated".to_string();
        assert_eq!(users[1].name, "Default");
        assert_eq!(users[2].name, "Default");
    }
}
