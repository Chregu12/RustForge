//! Event System for RustForge
//!
//! This crate provides event dispatching and listener management.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::RwLock;

/// Event errors
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Listener error: {0}")]
    ListenerError(String),

    #[error("Dispatch error: {0}")]
    DispatchError(String),
}

pub type EventResult<T> = Result<T, EventError>;

/// Event trait that all events must implement
pub trait Event: Send + Sync + 'static {
    /// Get the event name
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Event listener trait
#[async_trait]
pub trait EventListener: Send + Sync {
    /// Handle the event
    async fn handle(&self, event: &(dyn Any + Send + Sync)) -> EventResult<()>;

    /// Get listener priority (higher = earlier execution)
    fn priority(&self) -> i32 {
        0
    }
}

/// Typed event listener wrapper
struct TypedListener<E: Event, L: EventListenerFor<E>> {
    listener: L,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event, L: EventListenerFor<E>> TypedListener<E, L> {
    fn new(listener: L) -> Self {
        Self {
            listener,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<E: Event, L: EventListenerFor<E>> EventListener for TypedListener<E, L> {
    async fn handle(&self, event: &(dyn Any + Send + Sync)) -> EventResult<()> {
        if let Some(event) = event.downcast_ref::<E>() {
            self.listener.handle(event).await
        } else {
            Err(EventError::DispatchError("Type mismatch".to_string()))
        }
    }

    fn priority(&self) -> i32 {
        self.listener.priority()
    }
}

/// Typed event listener trait
#[async_trait]
pub trait EventListenerFor<E: Event>: Send + Sync + 'static {
    /// Handle the event
    async fn handle(&self, event: &E) -> EventResult<()>;

    /// Get listener priority
    fn priority(&self) -> i32 {
        0
    }
}

/// Event dispatcher
pub struct EventDispatcher {
    listeners: Arc<RwLock<HashMap<TypeId, Vec<Box<dyn EventListener>>>>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an event listener
    pub async fn listen<E: Event, L: EventListenerFor<E>>(&self, listener: L) {
        let mut listeners = self.listeners.write().await;
        let type_id = TypeId::of::<E>();

        let boxed: Box<dyn EventListener> = Box::new(TypedListener::new(listener));

        listeners
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(boxed);

        // Sort by priority (descending)
        if let Some(list) = listeners.get_mut(&type_id) {
            list.sort_by(|a, b| b.priority().cmp(&a.priority()));
        }
    }

    /// Dispatch an event
    pub async fn dispatch<E: Event>(&self, event: E) -> EventResult<()> {
        let listeners = self.listeners.read().await;
        let type_id = TypeId::of::<E>();

        if let Some(list) = listeners.get(&type_id) {
            for listener in list {
                listener.handle(&event as &(dyn Any + Send + Sync)).await?;
            }
        }

        Ok(())
    }

    /// Get listener count for an event type
    pub async fn listener_count<E: Event>(&self) -> usize {
        let listeners = self.listeners.read().await;
        let type_id = TypeId::of::<E>();

        listeners.get(&type_id).map(|l| l.len()).unwrap_or(0)
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Event history for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub listener_count: usize,
}

impl EventRecord {
    pub fn new(event_name: String, listener_count: usize) -> Self {
        Self {
            event_name,
            timestamp: chrono::Utc::now(),
            listener_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestEvent {
        message: String,
    }

    impl Event for TestEvent {}

    struct TestListener {
        called: Arc<RwLock<bool>>,
    }

    #[async_trait]
    impl EventListenerFor<TestEvent> for TestListener {
        async fn handle(&self, _event: &TestEvent) -> EventResult<()> {
            let mut called = self.called.write().await;
            *called = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_dispatcher() {
        let dispatcher = EventDispatcher::new();
        let called = Arc::new(RwLock::new(false));

        let listener = TestListener {
            called: called.clone(),
        };

        dispatcher.listen(listener).await;

        assert_eq!(dispatcher.listener_count::<TestEvent>().await, 1);

        dispatcher
            .dispatch(TestEvent {
                message: "test".to_string(),
            })
            .await
            .unwrap();

        assert!(*called.read().await);
    }

    struct PriorityListener {
        priority: i32,
        order: Arc<RwLock<Vec<i32>>>,
    }

    #[async_trait]
    impl EventListenerFor<TestEvent> for PriorityListener {
        async fn handle(&self, _event: &TestEvent) -> EventResult<()> {
            let mut order = self.order.write().await;
            order.push(self.priority);
            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[tokio::test]
    async fn test_listener_priority() {
        let dispatcher = EventDispatcher::new();
        let order = Arc::new(RwLock::new(Vec::new()));

        // Add listeners in random order
        dispatcher
            .listen(PriorityListener {
                priority: 1,
                order: order.clone(),
            })
            .await;
        dispatcher
            .listen(PriorityListener {
                priority: 10,
                order: order.clone(),
            })
            .await;
        dispatcher
            .listen(PriorityListener {
                priority: 5,
                order: order.clone(),
            })
            .await;

        dispatcher
            .dispatch(TestEvent {
                message: "test".to_string(),
            })
            .await
            .unwrap();

        let execution_order = order.read().await;
        // Should execute in priority order: 10, 5, 1
        assert_eq!(*execution_order, vec![10, 5, 1]);
    }

    #[tokio::test]
    async fn test_multiple_listeners() {
        let dispatcher = EventDispatcher::new();
        let count = Arc::new(RwLock::new(0));

        for _ in 0..5 {
            let count_clone = count.clone();
            struct CountListener {
                count: Arc<RwLock<i32>>,
            }

            #[async_trait]
            impl EventListenerFor<TestEvent> for CountListener {
                async fn handle(&self, _event: &TestEvent) -> EventResult<()> {
                    let mut c = self.count.write().await;
                    *c += 1;
                    Ok(())
                }
            }

            dispatcher
                .listen(CountListener {
                    count: count_clone,
                })
                .await;
        }

        assert_eq!(dispatcher.listener_count::<TestEvent>().await, 5);

        dispatcher
            .dispatch(TestEvent {
                message: "test".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(*count.read().await, 5);
    }

    #[test]
    fn test_event_record() {
        let record = EventRecord::new("TestEvent".to_string(), 3);
        assert_eq!(record.event_name, "TestEvent");
        assert_eq!(record.listener_count, 3);
    }

    #[tokio::test]
    async fn test_no_listeners() {
        let dispatcher = EventDispatcher::new();

        // Should not error when no listeners
        dispatcher
            .dispatch(TestEvent {
                message: "test".to_string(),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_listener_count_empty() {
        let dispatcher = EventDispatcher::new();
        assert_eq!(dispatcher.listener_count::<TestEvent>().await, 0);
    }

    #[derive(Clone)]
    struct AnotherEvent;
    impl Event for AnotherEvent {}

    #[tokio::test]
    async fn test_multiple_event_types() {
        let dispatcher = EventDispatcher::new();

        struct TestListener;

        #[async_trait]
        impl EventListenerFor<TestEvent> for TestListener {
            async fn handle(&self, _event: &TestEvent) -> EventResult<()> {
                Ok(())
            }
        }

        struct AnotherListener;

        #[async_trait]
        impl EventListenerFor<AnotherEvent> for AnotherListener {
            async fn handle(&self, _event: &AnotherEvent) -> EventResult<()> {
                Ok(())
            }
        }

        dispatcher.listen(TestListener).await;
        dispatcher.listen(AnotherListener).await;

        assert_eq!(dispatcher.listener_count::<TestEvent>().await, 1);
        assert_eq!(dispatcher.listener_count::<AnotherEvent>().await, 1);
    }
}
