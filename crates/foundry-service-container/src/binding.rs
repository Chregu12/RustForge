use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::Result;

/// Type alias for a factory closure that creates service instances
pub type Factory = Arc<dyn Fn() -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Type of binding (transient, singleton, or factory)
#[derive(Debug, Clone)]
pub enum BindingType {
    /// Creates a new instance every time
    Transient,
    /// Single shared instance (lazy initialized)
    Singleton,
    /// Custom factory function
    Factory,
}

/// Represents a service binding in the container
pub struct Binding {
    pub binding_type: BindingType,
    pub factory: Factory,
    pub instance: Arc<RwLock<Option<Arc<dyn Any + Send + Sync>>>>,
    pub tags: Vec<String>,
}

impl Binding {
    /// Create a new transient binding
    pub fn transient(factory: Factory) -> Self {
        Self {
            binding_type: BindingType::Transient,
            factory,
            instance: Arc::new(RwLock::new(None)),
            tags: Vec::new(),
        }
    }

    /// Create a new singleton binding
    pub fn singleton(factory: Factory) -> Self {
        Self {
            binding_type: BindingType::Singleton,
            factory,
            instance: Arc::new(RwLock::new(None)),
            tags: Vec::new(),
        }
    }

    /// Create a new factory binding
    pub fn factory(factory: Factory) -> Self {
        Self {
            binding_type: BindingType::Factory,
            factory,
            instance: Arc::new(RwLock::new(None)),
            tags: Vec::new(),
        }
    }

    /// Add tags to this binding
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Resolve the binding to an instance
    pub async fn resolve(&self) -> Result<Arc<dyn Any + Send + Sync>> {
        match self.binding_type {
            BindingType::Singleton => {
                // Check if instance already exists
                let instance_lock = self.instance.read().await;
                if let Some(instance) = instance_lock.as_ref() {
                    return Ok(instance.clone());
                }
                drop(instance_lock);

                // Create new instance
                let instance = (self.factory)()?;
                let mut instance_lock = self.instance.write().await;
                *instance_lock = Some(instance.clone());
                Ok(instance)
            }
            BindingType::Transient | BindingType::Factory => {
                // Always create new instance
                (self.factory)()
            }
        }
    }
}
