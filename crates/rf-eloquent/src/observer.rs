//! # Typed Model Observer System
//!
//! Provides a type-safe Laravel-style observer pattern for model lifecycle events.
//! Observers implement the [`Observer<M>`] trait and are registered globally
//! via [`observe`] or [`GLOBAL_OBSERVERS`].
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_eloquent::observer::{Observer, observe, dispatch_observers};
//! use rf_eloquent::events::{EventResult, ModelEvent};
//! use async_trait::async_trait;
//!
//! struct User { id: i64, email: String }
//!
//! struct UserObserver;
//!
//! #[async_trait]
//! impl Observer<User> for UserObserver {
//!     async fn created(&self, model: &User) -> EventResult {
//!         println!("User created: {}", model.email);
//!         Ok(())
//!     }
//!
//!     async fn deleting(&self, model: &User) -> EventResult {
//!         println!("About to delete user {}", model.id);
//!         Ok(())
//!     }
//! }
//!
//! // At application startup:
//! observe::<User, _>(UserObserver);
//!
//! // In your save/delete logic:
//! # async fn example(user: &User) -> EventResult {
//! dispatch_observers(ModelEvent::Created, user).await?;
//! # Ok(())
//! # }
//! ```

use crate::events::{EventError, EventResult, ModelEvent};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

/// Type-safe observer trait for model lifecycle events.
///
/// Implement this for your observer struct. All methods have default no-op
/// implementations so you only need to override the events you care about.
#[async_trait]
pub trait Observer<M: Send + Sync>: Send + Sync {
    async fn creating(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn created(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn updating(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn updated(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn saving(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn saved(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn deleting(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn deleted(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn restoring(&self, _model: &M) -> EventResult {
        Ok(())
    }
    async fn restored(&self, _model: &M) -> EventResult {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Type-erased dispatch infrastructure
// ---------------------------------------------------------------------------

/// Type-erased observer dispatch trait.
///
/// Internal use only. Allows storing observers with different model types
/// in the same collection.
trait ObserverDispatch: Send + Sync {
    fn dispatch_event<'a>(
        &'a self,
        event: ModelEvent,
        model: &'a (dyn Any + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = EventResult> + Send + 'a>>;
}

/// Typed wrapper that bridges `Observer<M>` to `ObserverDispatch`.
struct TypedObserverWrapper<M, O> {
    observer: O,
    _phantom: PhantomData<M>,
}

impl<M, O> TypedObserverWrapper<M, O>
where
    M: Send + Sync + 'static,
    O: Observer<M> + Send + Sync + 'static,
{
    fn new(observer: O) -> Self {
        Self {
            observer,
            _phantom: PhantomData,
        }
    }
}

impl<M, O> ObserverDispatch for TypedObserverWrapper<M, O>
where
    M: Send + Sync + 'static,
    O: Observer<M> + Send + Sync + 'static,
{
    fn dispatch_event<'a>(
        &'a self,
        event: ModelEvent,
        model: &'a (dyn Any + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = EventResult> + Send + 'a>> {
        Box::pin(async move {
            if let Some(m) = model.downcast_ref::<M>() {
                match event {
                    ModelEvent::Creating => self.observer.creating(m).await,
                    ModelEvent::Created => self.observer.created(m).await,
                    ModelEvent::Updating => self.observer.updating(m).await,
                    ModelEvent::Updated => self.observer.updated(m).await,
                    ModelEvent::Saving => self.observer.saving(m).await,
                    ModelEvent::Saved => self.observer.saved(m).await,
                    ModelEvent::Deleting => self.observer.deleting(m).await,
                    ModelEvent::Deleted => self.observer.deleted(m).await,
                    ModelEvent::Restoring => self.observer.restoring(m).await,
                    ModelEvent::Restored => self.observer.restored(m).await,
                }
            } else {
                Ok(())
            }
        })
    }
}

// ---------------------------------------------------------------------------
// ObserverRegistry
// ---------------------------------------------------------------------------

/// Registry that stores typed observers keyed by model `TypeId`.
pub struct ObserverRegistry {
    observers: RwLock<HashMap<TypeId, Vec<Arc<dyn ObserverDispatch>>>>,
}

impl ObserverRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            observers: RwLock::new(HashMap::new()),
        }
    }

    /// Register an observer for a model type.
    ///
    /// Multiple observers can be registered for the same type; they are
    /// dispatched in registration order.
    pub fn register<M, O>(&self, observer: O)
    where
        M: Send + Sync + 'static,
        O: Observer<M> + Send + Sync + 'static,
    {
        let wrapper = TypedObserverWrapper::new(observer);
        let boxed: Arc<dyn ObserverDispatch> = Arc::new(wrapper);
        let type_id = TypeId::of::<M>();
        if let Ok(mut map) = self.observers.write() {
            map.entry(type_id).or_default().push(boxed);
        }
    }

    /// Dispatch a model lifecycle event to all registered observers for `M`.
    pub async fn dispatch<M>(&self, event: ModelEvent, model: &M) -> EventResult
    where
        M: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<M>();
        let dispatchers = {
            let map = self
                .observers
                .read()
                .map_err(|_| EventError::HandlerFailed("Observer registry poisoned".to_string()))?;
            map.get(&type_id).cloned().unwrap_or_default()
        };

        let model_any: &(dyn Any + Send + Sync) = model;
        for dispatcher in dispatchers {
            dispatcher.dispatch_event(event, model_any).await?;
        }

        Ok(())
    }

    /// Remove all observers for a model type.
    pub fn clear<M: 'static>(&self) {
        let type_id = TypeId::of::<M>();
        if let Ok(mut map) = self.observers.write() {
            map.remove(&type_id);
        }
    }
}

impl Default for ObserverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Global registry + convenience functions
// ---------------------------------------------------------------------------

/// Global observer registry shared across the application.
pub static GLOBAL_OBSERVERS: Lazy<ObserverRegistry> = Lazy::new(ObserverRegistry::new);

/// Register an observer globally for a model type.
///
/// Call at application startup, typically in your `main` function or a
/// `boot` method.
///
/// # Example
///
/// ```rust,no_run
/// use rf_eloquent::observer::observe;
/// # struct User; struct UserObserver;
/// # #[async_trait::async_trait]
/// # impl rf_eloquent::observer::Observer<User> for UserObserver {}
///
/// observe::<User, _>(UserObserver);
/// ```
pub fn observe<M, O>(observer: O)
where
    M: Send + Sync + 'static,
    O: Observer<M> + Send + Sync + 'static,
{
    GLOBAL_OBSERVERS.register::<M, O>(observer);
}

/// Dispatch a lifecycle event to all globally registered observers for `M`.
///
/// # Example
///
/// ```rust,no_run
/// use rf_eloquent::observer::dispatch_observers;
/// use rf_eloquent::events::ModelEvent;
/// # struct User;
///
/// # async fn example(user: &User) -> rf_eloquent::events::EventResult {
/// dispatch_observers(ModelEvent::Created, user).await?;
/// # Ok(())
/// # }
/// ```
pub async fn dispatch_observers<M>(event: ModelEvent, model: &M) -> EventResult
where
    M: Send + Sync + 'static,
{
    GLOBAL_OBSERVERS.dispatch(event, model).await
}

/// Convenience macro for registering an observer.
///
/// ```rust,no_run
/// use rf_eloquent::observe;
/// # struct User; struct UserObserver;
/// # #[async_trait::async_trait]
/// # impl rf_eloquent::observer::Observer<User> for UserObserver {}
///
/// observe!(UserObserver, User);
/// ```
#[macro_export]
macro_rules! observe {
    ($observer:expr, $model:ty) => {
        $crate::observer::observe::<$model, _>($observer)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ModelEvent;

    struct TestModel {
        name: String,
    }

    struct TestObserver {
        log: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Observer<TestModel> for TestObserver {
        async fn created(&self, model: &TestModel) -> EventResult {
            self.log.lock().unwrap().push(format!("created:{}", model.name));
            Ok(())
        }

        async fn deleting(&self, model: &TestModel) -> EventResult {
            self.log.lock().unwrap().push(format!("deleting:{}", model.name));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_observer_registry_dispatch() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        registry.register::<TestModel, _>(TestObserver { log: log.clone() });

        let model = TestModel { name: "alice".to_string() };
        registry.dispatch(ModelEvent::Created, &model).await.unwrap();
        registry.dispatch(ModelEvent::Deleting, &model).await.unwrap();

        let entries = log.lock().unwrap().clone();
        assert_eq!(entries, vec!["created:alice", "deleting:alice"]);
    }

    #[tokio::test]
    async fn test_observer_no_op_default_methods() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

        registry.register::<TestModel, _>(TestObserver { log: log.clone() });

        let model = TestModel { name: "bob".to_string() };
        // Updating has the default no-op implementation
        registry.dispatch(ModelEvent::Updating, &model).await.unwrap();

        assert!(log.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_multiple_observers() {
        let registry = ObserverRegistry::new();
        let log1 = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log2 = Arc::new(std::sync::Mutex::new(Vec::new()));

        registry.register::<TestModel, _>(TestObserver { log: log1.clone() });
        registry.register::<TestModel, _>(TestObserver { log: log2.clone() });

        let model = TestModel { name: "charlie".to_string() };
        registry.dispatch(ModelEvent::Created, &model).await.unwrap();

        assert_eq!(log1.lock().unwrap().len(), 1);
        assert_eq!(log2.lock().unwrap().len(), 1);
    }
}
