use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

use crate::error::{EventError, Result};
use crate::listener::{EventListener, ListenerPriority};
use crate::Event;

type ListenerBox<E> = Box<dyn EventListener<E>>;

/// Central event dispatcher for managing and dispatching events
pub struct EventDispatcher {
    listeners: Arc<RwLock<HashMap<TypeId, Vec<ListenerEntry>>>>,
}

struct ListenerEntry {
    id: usize,
    listener: Arc<dyn Any + Send + Sync>,
    priority: ListenerPriority,
    once: bool,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an event listener
    pub async fn listen<E, L>(&self, listener: L) -> usize
    where
        E: Event,
        L: EventListener<E> + 'static,
    {
        let type_id = TypeId::of::<E>();
        let priority = listener.priority();
        let once = listener.once();

        let mut listeners = self.listeners.write().await;
        let entries = listeners.entry(type_id).or_insert_with(Vec::new);

        let id = entries.len();
        let entry = ListenerEntry {
            id,
            listener: Arc::new(listener),
            priority,
            once,
        };

        entries.push(entry);

        // Sort by priority (highest first)
        entries.sort_by(|a, b| b.priority.cmp(&a.priority));

        debug!("Registered listener {} for event type {:?}", id, type_id);

        id
    }

    /// Register a one-time listener
    pub async fn once<E, L>(&self, mut listener: L) -> usize
    where
        E: Event,
        L: EventListener<E> + 'static,
    {
        // Unfortunately we can't modify the listener since it's moved
        // So we need to wrap it in a custom implementation
        struct OnceWrapper<L>(L);

        #[async_trait::async_trait]
        impl<E, L> EventListener<E> for OnceWrapper<L>
        where
            E: Send + Sync + 'static,
            L: EventListener<E> + Send + Sync + 'static,
        {
            async fn handle(&self, event: &E) -> Result<()> {
                self.0.handle(event).await
            }

            fn priority(&self) -> ListenerPriority {
                self.0.priority()
            }

            fn once(&self) -> bool {
                true
            }
        }

        self.listen::<E, _>(OnceWrapper(listener)).await
    }

    /// Dispatch an event to all registered listeners
    pub async fn dispatch<E>(&self, event: E) -> Result<()>
    where
        E: Event,
    {
        let type_id = TypeId::of::<E>();
        debug!("Dispatching event: {}", event.event_name());

        let listeners = self.listeners.read().await;
        let Some(entries) = listeners.get(&type_id) else {
            debug!("No listeners registered for event type {:?}", type_id);
            return Ok(());
        };

        let entries: Vec<_> = entries.iter().collect();
        drop(listeners); // Release read lock

        let mut to_remove = Vec::new();

        for entry in entries {
            let listener = entry.listener.clone();
            let listener = listener
                .downcast::<Box<dyn EventListener<E>>>()
                .or_else(|arc| {
                    // Try downcasting to the actual listener type
                    arc.downcast::<dyn EventListener<E>>()
                })
                .map_err(|_| EventError::DispatchError("Failed to downcast listener".to_string()))?;

            match listener.handle(&event).await {
                Ok(_) => {
                    debug!("Listener {} handled event successfully", entry.id);
                }
                Err(e) => {
                    warn!("Listener {} failed: {}", entry.id, e);
                    // Continue to next listener - don't break the chain
                }
            }

            if entry.once {
                to_remove.push(entry.id);
            }
        }

        // Remove one-time listeners
        if !to_remove.is_empty() {
            let mut listeners = self.listeners.write().await;
            if let Some(entries) = listeners.get_mut(&type_id) {
                entries.retain(|entry| !to_remove.contains(&entry.id));
            }
        }

        Ok(())
    }

    /// Remove a specific listener
    pub async fn remove_listener<E>(&self, listener_id: usize)
    where
        E: Event,
    {
        let type_id = TypeId::of::<E>();
        let mut listeners = self.listeners.write().await;

        if let Some(entries) = listeners.get_mut(&type_id) {
            entries.retain(|entry| entry.id != listener_id);
            debug!("Removed listener {}", listener_id);
        }
    }

    /// Remove all listeners for an event type
    pub async fn clear_listeners<E>(&self)
    where
        E: Event,
    {
        let type_id = TypeId::of::<E>();
        let mut listeners = self.listeners.write().await;
        listeners.remove(&type_id);
        debug!("Cleared all listeners for event type {:?}", type_id);
    }

    /// Remove all listeners
    pub async fn clear_all(&self) {
        let mut listeners = self.listeners.write().await;
        listeners.clear();
        debug!("Cleared all listeners");
    }

    /// Get count of listeners for an event type
    pub async fn listener_count<E>(&self) -> usize
    where
        E: Event,
    {
        let type_id = TypeId::of::<E>();
        let listeners = self.listeners.read().await;
        listeners.get(&type_id).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use crate::listener::FunctionListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_dispatcher_creation() {
        let dispatcher = EventDispatcher::new();
        assert_eq!(dispatcher.listener_count::<CommandFinished>().await, 0);
    }

    #[tokio::test]
    async fn test_register_and_dispatch() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let listener = FunctionListener::new(move |_event: &CommandFinished| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });

        dispatcher.listen(listener).await;

        let event = CommandFinished {
            command: "test".to_string(),
            duration: 100,
            exit_code: 0,
            output: "OK".to_string(),
        };

        dispatcher.dispatch(event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_multiple_listeners() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..3 {
            let counter_clone = counter.clone();
            let listener = FunctionListener::new(move |_event: &CommandFinished| {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            });
            dispatcher.listen(listener).await;
        }

        assert_eq!(dispatcher.listener_count::<CommandFinished>().await, 3);

        let event = CommandFinished {
            command: "test".to_string(),
            duration: 100,
            exit_code: 0,
            output: "OK".to_string(),
        };

        dispatcher.dispatch(event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_once_listener() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let listener = FunctionListener::new(move |_event: &CommandFinished| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        })
        .once();

        dispatcher.listen(listener).await;

        let event = CommandFinished {
            command: "test".to_string(),
            duration: 100,
            exit_code: 0,
            output: "OK".to_string(),
        };

        dispatcher.dispatch(event.clone()).await.unwrap();
        dispatcher.dispatch(event).await.unwrap();

        // Should only be called once
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_priority_execution() {
        let dispatcher = EventDispatcher::new();
        let order = Arc::new(RwLock::new(Vec::new()));

        let order1 = order.clone();
        let listener1 = FunctionListener::new(move |_event: &CommandFinished| {
            let order = order1.clone();
            Box::pin(async move {
                order.write().await.push(1);
                Ok(())
            })
        })
        .with_priority(ListenerPriority::Lowest);

        let order2 = order.clone();
        let listener2 = FunctionListener::new(move |_event: &CommandFinished| {
            let order = order2.clone();
            Box::pin(async move {
                order.write().await.push(2);
                Ok(())
            })
        })
        .with_priority(ListenerPriority::Highest);

        dispatcher.listen(listener1).await;
        dispatcher.listen(listener2).await;

        let event = CommandFinished {
            command: "test".to_string(),
            duration: 100,
            exit_code: 0,
            output: "OK".to_string(),
        };

        dispatcher.dispatch(event).await.unwrap();

        let order = order.read().await;
        // Highest priority should execute first
        assert_eq!(*order, vec![2, 1]);
    }

    #[tokio::test]
    async fn test_clear_listeners() {
        let dispatcher = EventDispatcher::new();

        let listener = FunctionListener::new(|_event: &CommandFinished| {
            Box::pin(async { Ok(()) })
        });

        dispatcher.listen(listener).await;
        assert_eq!(dispatcher.listener_count::<CommandFinished>().await, 1);

        dispatcher.clear_listeners::<CommandFinished>().await;
        assert_eq!(dispatcher.listener_count::<CommandFinished>().await, 0);
    }

    #[tokio::test]
    async fn test_error_handling_continues_execution() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));

        // First listener that fails
        let listener1 = FunctionListener::new(|_event: &CommandFinished| {
            Box::pin(async {
                Err(EventError::ListenerError("Test error".to_string()))
            })
        });

        // Second listener that succeeds
        let counter_clone = counter.clone();
        let listener2 = FunctionListener::new(move |_event: &CommandFinished| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });

        dispatcher.listen(listener1).await;
        dispatcher.listen(listener2).await;

        let event = CommandFinished {
            command: "test".to_string(),
            duration: 100,
            exit_code: 0,
            output: "OK".to_string(),
        };

        dispatcher.dispatch(event).await.unwrap();

        // Second listener should still execute despite first one failing
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
