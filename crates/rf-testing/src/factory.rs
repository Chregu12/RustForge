//! Test factories for generating test data
//!
//! Factories provide a convenient way to generate test data with the builder pattern.
//! Supports state modification, batch creation, and relationship handling.
//!
//! # Example
//!
//! ```rust
//! use rf_testing::{Factory, FactoryError, FactoryDefinition, Fake};
//! use async_trait::async_trait;
//!
//! #[derive(Clone, Debug)]
//! struct User {
//!     id: i32,
//!     name: String,
//!     email: String,
//!     role: String,
//! }
//!
//! struct UserFactory {
//!     model: User,
//! }
//!
//! impl Default for UserFactory {
//!     fn default() -> Self {
//!         Self {
//!             model: <UserFactory as FactoryDefinition>::definition(),
//!         }
//!     }
//! }
//!
//! impl FactoryDefinition for UserFactory {
//!     type Model = User;
//!
//!     fn definition() -> Self::Model {
//!         User {
//!             id: 0,
//!             name: Fake::name(),
//!             email: Fake::email(),
//!             role: "user".to_string(),
//!         }
//!     }
//! }
//!
//! rf_testing::impl_factory!(UserFactory, User);
//! ```

use async_trait::async_trait;
use std::marker::PhantomData;
use thiserror::Error;

/// Factory errors
#[derive(Debug, Error)]
pub enum FactoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("Generic error: {0}")]
    GenericError(String),
}

/// Factory trait for creating test data
///
/// This trait provides a comprehensive API for creating test data with support
/// for state modification, batch creation, and relationship handling.
#[async_trait]
pub trait Factory: Sized + Send {
    /// The model type this factory creates
    type Model: Clone + Send;

    /// Define the default state of the model
    ///
    /// This method should return a model instance with fake data filled in.
    fn definition() -> Self::Model;

    /// Create a new factory instance with default definition
    fn new() -> Self
    where
        Self: Default,
    {
        Self::default()
    }

    /// Modify the model state before creation
    ///
    /// # Example
    /// ```ignore
    /// let admin = UserFactory::new()
    ///     .state(|u| u.role = "admin".to_string())
    ///     .create()
    ///     .await?;
    /// ```
    fn state<F>(self, modifier: F) -> Self
    where
        F: FnOnce(&mut Self::Model);

    /// Create and persist the model
    ///
    /// Override this method to add custom persistence logic (e.g., database save)
    async fn create(self) -> Result<Self::Model, FactoryError>;

    /// Build the model without persisting
    ///
    /// Use this when you need the model instance but don't want to save it
    fn build(self) -> Self::Model;

    /// Create multiple instances
    async fn create_many(count: usize) -> Result<Vec<Self::Model>, FactoryError>
    where
        Self: Default,
    {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            let instance = Self::default();
            results.push(instance.create().await?);
        }
        Ok(results)
    }

    /// Create a factory builder for batch operations
    fn count(count: usize) -> FactoryBuilder<Self>
    where
        Self: Default,
    {
        FactoryBuilder::new(count)
    }
}

/// Builder for creating multiple factory instances with shared state
pub struct FactoryBuilder<F: Factory> {
    count: usize,
    states: Vec<Box<dyn FnOnce(&mut F::Model) + Send>>,
    _phantom: PhantomData<F>,
}

impl<F: Factory + Default> FactoryBuilder<F> {
    /// Create a new factory builder
    pub fn new(count: usize) -> Self {
        Self {
            count,
            states: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Add a state modifier that will be applied to all instances
    pub fn state<Fn>(mut self, modifier: Fn) -> Self
    where
        Fn: FnOnce(&mut F::Model) + Send + 'static,
    {
        self.states.push(Box::new(modifier));
        self
    }

    /// Create all instances
    pub async fn create(self) -> Result<Vec<F::Model>, FactoryError> {
        F::create_many(self.count).await
    }
}

/// Helper trait for defining factory definitions
pub trait FactoryDefinition {
    type Model;

    fn definition() -> Self::Model;
}

/// Macro to implement basic Factory trait
#[macro_export]
macro_rules! impl_factory {
    ($factory:ty, $model:ty) => {
        #[async_trait::async_trait]
        impl Factory for $factory {
            type Model = $model;

            fn definition() -> Self::Model {
                <$factory as FactoryDefinition>::definition()
            }

            fn state<F>(mut self, modifier: F) -> Self
            where
                F: FnOnce(&mut Self::Model),
            {
                modifier(&mut self.model);
                self
            }

            async fn create(self) -> Result<Self::Model, FactoryError> {
                Ok(self.model)
            }

            fn build(self) -> Self::Model {
                self.model
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestUser {
        id: i32,
        name: String,
        email: String,
        role: String,
    }

    struct TestUserFactory {
        model: TestUser,
    }

    impl Default for TestUserFactory {
        fn default() -> Self {
            Self {
                model: <TestUserFactory as FactoryDefinition>::definition(),
            }
        }
    }

    impl FactoryDefinition for TestUserFactory {
        type Model = TestUser;

        fn definition() -> Self::Model {
            TestUser {
                id: 0,
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                role: "user".to_string(),
            }
        }
    }

    impl_factory!(TestUserFactory, TestUser);

    #[tokio::test]
    async fn test_factory_create() {
        let user = TestUserFactory::new().create().await.unwrap();
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_factory_state() {
        let admin = TestUserFactory::new()
            .state(|u| u.role = "admin".to_string())
            .create()
            .await
            .unwrap();

        assert_eq!(admin.role, "admin");
    }

    #[tokio::test]
    async fn test_factory_build() {
        let user = TestUserFactory::new().build();
        assert_eq!(user.name, "Test User");
    }

    #[tokio::test]
    async fn test_factory_create_many() {
        let users = TestUserFactory::create_many(5).await.unwrap();
        assert_eq!(users.len(), 5);
    }

    #[tokio::test]
    async fn test_factory_builder() {
        let users = TestUserFactory::count(3).create().await.unwrap();
        assert_eq!(users.len(), 3);
    }
}
