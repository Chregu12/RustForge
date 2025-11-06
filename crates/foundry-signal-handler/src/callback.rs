//! Signal callback types and management

use crate::error::{SignalError, SignalResult};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for signal callback functions
pub type SignalCallbackFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = SignalResult<()>> + Send>> + Send + Sync>;

/// Signal callback wrapper
#[derive(Clone)]
pub struct SignalCallback {
    callback: SignalCallbackFn,
    name: String,
}

impl SignalCallback {
    /// Create new signal callback
    pub fn new<F, Fut>(name: impl Into<String>, callback: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = SignalResult<()>> + Send + 'static,
    {
        let callback = Arc::new(move || Box::pin(callback()) as Pin<Box<dyn Future<Output = SignalResult<()>> + Send>>);
        Self {
            callback,
            name: name.into(),
        }
    }

    /// Create callback from synchronous function
    pub fn sync<F>(name: impl Into<String>, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self::new(name, move || {
            callback();
            async { Ok(()) }
        })
    }

    /// Execute the callback
    pub async fn execute(&self) -> SignalResult<()> {
        tracing::debug!("Executing callback: {}", self.name);
        (self.callback)().await.map_err(|e| {
            SignalError::CallbackFailed(format!("{}: {}", self.name, e))
        })
    }

    /// Get callback name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Callback collection for managing multiple callbacks
#[derive(Clone, Default)]
pub struct CallbackCollection {
    callbacks: Vec<SignalCallback>,
}

impl CallbackCollection {
    /// Create new empty collection
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Add a callback
    pub fn add(&mut self, callback: SignalCallback) {
        self.callbacks.push(callback);
    }

    /// Execute all callbacks in order
    pub async fn execute_all(&self) -> SignalResult<()> {
        for callback in &self.callbacks {
            callback.execute().await?;
        }
        Ok(())
    }

    /// Get number of callbacks
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// Clear all callbacks
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }

    /// Get callback names
    pub fn callback_names(&self) -> Vec<String> {
        self.callbacks.iter().map(|cb| cb.name().to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_callback() {
        let executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed_clone = executed.clone();

        let callback = SignalCallback::sync("test", move || {
            executed_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        callback.execute().await.unwrap();
        assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_async_callback() {
        let executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed_clone = executed.clone();

        let callback = SignalCallback::new("test", move || {
            let executed = executed_clone.clone();
            async move {
                executed.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        });

        callback.execute().await.unwrap();
        assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_callback_collection() {
        let mut collection = CallbackCollection::new();
        assert!(collection.is_empty());

        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        for i in 0..3 {
            let counter_clone = counter.clone();
            collection.add(SignalCallback::sync(format!("callback_{}", i), move || {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }));
        }

        assert_eq!(collection.len(), 3);
        collection.execute_all().await.unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_callback_names() {
        let mut collection = CallbackCollection::new();
        collection.add(SignalCallback::sync("callback1", || {}));
        collection.add(SignalCallback::sync("callback2", || {}));

        let names = collection.callback_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"callback1".to_string()));
        assert!(names.contains(&"callback2".to_string()));
    }
}
