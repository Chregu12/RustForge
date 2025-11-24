//! Event dispatching helper.
//!
//! This module provides a simple event() function for dispatching events.

use std::any::Any;
use std::sync::Arc;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

/// Event trait that all events must implement
pub trait Event: Any + Send + Sync {
    /// Get the event name
    fn name(&self) -> &str;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Event listener type
pub type EventListener = Arc<dyn Fn(&dyn Event) + Send + Sync>;

/// Global event dispatcher
pub struct EventDispatcher {
    listeners: RwLock<Vec<(String, EventListener)>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Register an event listener
    pub fn listen<F>(&self, event_name: impl Into<String>, listener: F)
    where
        F: Fn(&dyn Event) + Send + Sync + 'static,
    {
        self.listeners
            .write()
            .push((event_name.into(), Arc::new(listener)));
    }

    /// Dispatch an event
    pub fn dispatch<E: Event>(&self, event: &E) {
        let listeners = self.listeners.read();
        let event_name = event.name();

        for (name, listener) in listeners.iter() {
            if name == event_name || name == "*" {
                listener(event);
            }
        }
    }

    /// Clear all listeners (useful for testing)
    pub fn clear(&self) {
        self.listeners.write().clear();
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Global event dispatcher instance
static EVENT_DISPATCHER: Lazy<EventDispatcher> = Lazy::new(EventDispatcher::new);

/// Get the global event dispatcher
pub fn global_dispatcher() -> &'static EventDispatcher {
    &EVENT_DISPATCHER
}

/// Dispatch an event to all registered listeners.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::event;
///
/// struct UserCreated {
///     user_id: u64,
/// }
///
/// impl rf_global_helpers::event::Event for UserCreated {
///     fn name(&self) -> &str {
///         "user.created"
///     }
///
///     fn as_any(&self) -> &dyn std::any::Any {
///         self
///     }
/// }
///
/// let user_event = UserCreated { user_id: 123 };
/// event(&user_event);
/// ```
pub fn event<E: Event>(event: &E) {
    global_dispatcher().dispatch(event);
}

/// Register an event listener.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::event::{listen, Event};
///
/// listen("user.created", |event| {
///     println!("User created: {}", event.name());
/// });
/// ```
pub fn listen<F>(event_name: impl Into<String>, listener: F)
where
    F: Fn(&dyn Event) + Send + Sync + 'static,
{
    global_dispatcher().listen(event_name, listener);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestEvent {
        message: String,
    }

    impl Event for TestEvent {
        fn name(&self) -> &str {
            "test.event"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_event_dispatch() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        dispatcher.listen("test.event", move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = TestEvent {
            message: "Hello".to_string(),
        };

        dispatcher.dispatch(&event);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        dispatcher.dispatch(&event);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_multiple_listeners() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter1 = counter.clone();
        dispatcher.listen("test.event", move |_| {
            counter1.fetch_add(1, Ordering::SeqCst);
        });

        let counter2 = counter.clone();
        dispatcher.listen("test.event", move |_| {
            counter2.fetch_add(10, Ordering::SeqCst);
        });

        let event = TestEvent {
            message: "Test".to_string(),
        };

        dispatcher.dispatch(&event);
        assert_eq!(counter.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn test_wildcard_listener() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        dispatcher.listen("*", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event1 = TestEvent {
            message: "Event 1".to_string(),
        };
        let event2 = TestEvent {
            message: "Event 2".to_string(),
        };

        dispatcher.dispatch(&event1);
        dispatcher.dispatch(&event2);

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_event_helper() {
        global_dispatcher().clear();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        listen("test.event", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let test_event = TestEvent {
            message: "Helper test".to_string(),
        };

        event(&test_event);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
