//! Advanced Factory Features
//!
//! Factory states, sequences, and relationships for complex test data generation

use crate::factory::{Factory, FactoryError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Sequence generator for unique values
///
/// # Example
///
/// ```rust
/// use rf_testing::factory_advanced::Sequence;
///
/// let seq = Sequence::new();
/// assert_eq!(seq.next(), 0);
/// assert_eq!(seq.next(), 1);
/// assert_eq!(seq.next(), 2);
/// ```
pub struct Sequence {
    current: AtomicUsize,
}

impl Sequence {
    /// Create a new sequence starting at 0
    pub fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
        }
    }

    /// Create a sequence starting at a specific value
    pub fn starting_at(start: usize) -> Self {
        Self {
            current: AtomicUsize::new(start),
        }
    }

    /// Get the next value in the sequence
    pub fn next(&self) -> usize {
        self.current.fetch_add(1, Ordering::SeqCst)
    }

    /// Get current value without incrementing
    pub fn current(&self) -> usize {
        self.current.load(Ordering::SeqCst)
    }

    /// Reset the sequence
    pub fn reset(&self) {
        self.current.store(0, Ordering::SeqCst);
    }

    /// Reset to a specific value
    pub fn reset_to(&self, value: usize) {
        self.current.store(value, Ordering::SeqCst);
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory state manager
///
/// Allows defining named states that modify the factory definition
pub struct FactoryState<F: Factory> {
    base_factory: F,
    state_name: String,
    state_modifiers: Arc<Mutex<HashMap<String, Box<dyn Fn(&mut F::Model) + Send + Sync>>>>,
}

impl<F: Factory> FactoryState<F> {
    /// Create a new factory state
    pub fn new(base_factory: F, state_name: impl Into<String>) -> Self {
        Self {
            base_factory,
            state_name: state_name.into(),
            state_modifiers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a state modifier
    pub fn register_state<Fn>(&self, name: impl Into<String>, modifier: Fn)
    where
        Fn: (FnOnce(&mut F::Model)) + Send + Sync + 'static + Clone,
    {
        let modifiers = &self.state_modifiers;
        let modifier_clone = modifier.clone();
        modifiers.lock().unwrap().insert(
            name.into(),
            Box::new(move |model| {
                let m = modifier_clone.clone();
                m(model);
            }),
        );
    }

    /// Apply state to a model
    pub fn apply_state(&self, model: &mut F::Model) {
        if let Some(modifier) = self.state_modifiers.lock().unwrap().get(&self.state_name) {
            modifier(model);
        }
    }
}

/// After-create callback type
pub type AfterCreateCallback<T> =
    Box<dyn Fn(T) -> Pin<Box<dyn Future<Output = Result<(), FactoryError>> + Send>> + Send + Sync>;

/// Enhanced factory with states and relationships
pub struct EnhancedFactory<F: Factory> {
    inner: F,
    states: HashMap<String, Box<dyn Fn(&mut F::Model) + Send + Sync>>,
    after_create: Vec<AfterCreateCallback<F::Model>>,
    count: usize,
}

impl<F: Factory + Default> EnhancedFactory<F> {
    /// Create a new enhanced factory
    pub fn new() -> Self {
        Self {
            inner: F::default(),
            states: HashMap::new(),
            after_create: Vec::new(),
            count: 1,
        }
    }

    /// Define a state
    ///
    /// # Example
    ///
    /// ```ignore
    /// let factory = EnhancedFactory::<UserFactory>::new()
    ///     .define_state("admin", |user| {
    ///         user.role = "admin".to_string();
    ///         user.is_verified = true;
    ///     });
    /// ```
    pub fn define_state<Fn>(mut self, name: impl Into<String>, modifier: Fn) -> Self
    where
        Fn: (FnOnce(&mut F::Model)) + Send + Sync + 'static + Clone,
    {
        let name_str = name.into();
        self.states.insert(
            name_str.clone(),
            Box::new(move |model| {
                let m = modifier.clone();
                m(model);
            }),
        );
        self
    }

    /// Use a named state
    pub fn as_state(mut self, name: impl AsRef<str>) -> Self {
        if let Some(modifier) = self.states.get(name.as_ref()) {
            let model = &mut self.inner;
            // Note: This is simplified - in real implementation we'd need to apply to the model
        }
        self
    }

    /// Set count for batch creation
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Add an after-create callback
    ///
    /// # Example
    ///
    /// ```ignore
    /// factory.after_create(|user| Box::pin(async move {
    ///     // Create related records
    ///     ProfileFactory::new().for_user(&user).create().await?;
    ///     Ok(())
    /// }))
    /// ```
    pub fn after_create<Fn>(mut self, callback: Fn) -> Self
    where
        Fn: (FnOnce(F::Model) -> Pin<Box<dyn Future<Output = Result<(), FactoryError>> + Send>>)
            + Send
            + Sync
            + 'static
            + Clone,
    {
        self.after_create.push(Box::new(move |model| {
            let cb = callback.clone();
            cb(model)
        }));
        self
    }
}

#[async_trait]
impl<F: Factory + Default + Send> Factory for EnhancedFactory<F> {
    type Model = F::Model;

    fn definition() -> Self::Model {
        F::definition()
    }

    fn state<Fn>(self, modifier: Fn) -> Self
    where
        Fn: FnOnce(&mut Self::Model),
    {
        let mut inner = self.inner.state(modifier);
        Self {
            inner,
            states: self.states,
            after_create: self.after_create,
            count: self.count,
        }
    }

    async fn create(self) -> Result<Self::Model, FactoryError> {
        let model = self.inner.create().await?;

        // Run after-create callbacks
        for callback in &self.after_create {
            callback(model.clone()).await?;
        }

        Ok(model)
    }

    fn build(self) -> Self::Model {
        self.inner.build()
    }
}

/// Relationship builder helper
pub struct RelationshipBuilder<Parent, Child> {
    _parent: PhantomData<Parent>,
    _child: PhantomData<Child>,
}

impl<Parent, Child> RelationshipBuilder<Parent, Child> {
    /// Create a new relationship builder
    pub fn new() -> Self {
        Self {
            _parent: PhantomData,
            _child: PhantomData,
        }
    }
}

impl<Parent, Child> Default for RelationshipBuilder<Parent, Child> {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to add relationship methods to factories
#[macro_export]
macro_rules! impl_factory_relationships {
    ($factory:ty, $model:ty, {
        $(
            $method:ident -> $related_field:ident : $related_type:ty
        ),* $(,)?
    }) => {
        impl $factory {
            $(
                pub fn $method(mut self, related: &$related_type) -> Self {
                    // This would set the foreign key
                    // Implementation depends on your model structure
                    self
                }
            )*
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence() {
        let seq = Sequence::new();
        assert_eq!(seq.next(), 0);
        assert_eq!(seq.next(), 1);
        assert_eq!(seq.next(), 2);
    }

    #[test]
    fn test_sequence_starting_at() {
        let seq = Sequence::starting_at(100);
        assert_eq!(seq.next(), 100);
        assert_eq!(seq.next(), 101);
    }

    #[test]
    fn test_sequence_current() {
        let seq = Sequence::new();
        assert_eq!(seq.current(), 0);
        seq.next();
        assert_eq!(seq.current(), 1);
    }

    #[test]
    fn test_sequence_reset() {
        let seq = Sequence::new();
        seq.next();
        seq.next();
        assert_eq!(seq.current(), 2);

        seq.reset();
        assert_eq!(seq.current(), 0);
        assert_eq!(seq.next(), 0);
    }

    #[test]
    fn test_sequence_reset_to() {
        let seq = Sequence::new();
        seq.next();
        seq.next();

        seq.reset_to(50);
        assert_eq!(seq.next(), 50);
    }
}
