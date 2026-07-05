//! Event facade providing Laravel-style static event API

use crate::manager::GLOBAL_EVENT;
use serde::Serialize;
use serde_json::Value;

/// The Event facade providing a static-like API for event dispatching.
///
/// Simple, Laravel-style API - no `.await` needed anywhere!
///
/// # Examples
///
/// ```rust,no_run
/// use rf_event_facade::Event;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct UserCreated {
///     user_id: u64,
///     email: String,
/// }
///
/// fn example() -> Result<(), Box<dyn std::error::Error>> {
///     // Dispatch an event
///     Event::dispatch("user.created", UserCreated {
///         user_id: 1,
///         email: "user@example.com".to_string(),
///     })?;
///
///     // Listen for events
///     Event::listen("user.created", |event: &serde_json::Value| {
///         println!("Event received: {:?}", event);
///     });
///
///     // Check if event has listeners
///     if Event::has_listeners("user.created") {
///         println!("Event has listeners");
///     }
///     Ok(())
/// }
/// ```
pub struct Event;

impl Event {
    /// Dispatch an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct OrderPlaced {
    ///     order_id: u64,
    ///     total: f64,
    /// }
    ///
    /// fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     Event::dispatch("order.placed", OrderPlaced {
    ///         order_id: 123,
    ///         total: 99.99,
    ///     })?;
    ///     Ok(())
    /// }
    /// ```
    pub fn dispatch<T: Serialize>(event_name: &str, data: T) -> Result<(), String> {
        let value = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize event data: {}", e))?;

        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.dispatch(event_name, value)
    }

    /// Listen for an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// fn example() {
    ///     Event::listen("user.created", |data| {
    ///         println!("User created: {:?}", data);
    ///     });
    /// }
    /// ```
    pub fn listen<F>(event_name: &str, callback: F)
    where
        F: Fn(&Value) + Send + Sync + 'static,
    {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.listen(event_name, callback);
    }

    /// Check if an event has listeners
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// fn example() {
    ///     if Event::has_listeners("user.created") {
    ///         println!("Event has listeners");
    ///     }
    /// }
    /// ```
    pub fn has_listeners(event_name: &str) -> bool {
        let manager = GLOBAL_EVENT.read().unwrap();
        manager.has_listeners(event_name)
    }

    /// Get the number of listeners for an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// fn example() {
    ///     let count = Event::listener_count("user.created");
    ///     println!("Number of listeners: {}", count);
    /// }
    /// ```
    pub fn listener_count(event_name: &str) -> usize {
        let manager = GLOBAL_EVENT.read().unwrap();
        manager.listener_count(event_name)
    }

    /// Forget all listeners for an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// fn example() {
    ///     Event::forget("user.created");
    /// }
    /// ```
    pub fn forget(event_name: &str) {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.forget(event_name);
    }

    /// Forget all event listeners
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// fn example() {
    ///     Event::forget_all();
    /// }
    /// ```
    pub fn forget_all() {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.forget_all();
    }

    /// Dispatch a typed event by its concrete type (synchronous, in-process).
    ///
    /// This is the type-keyed counterpart to [`Event::dispatch`] (which keys by a
    /// string name). Every listener registered via [`Event::listen_typed`] for the
    /// payload's type is invoked synchronously. Returns the number of listeners
    /// fired. See [`crate::typed`] for the free-function form `event(payload)`.
    ///
    /// ```
    /// use rf_event_facade::Event;
    /// struct Shipped { order_id: u64 }
    /// Event::listen_typed::<Shipped, _>(|e| assert_eq!(e.order_id, 9));
    /// let fired = Event::fire(Shipped { order_id: 9 });
    /// assert!(fired >= 1);
    /// ```
    pub fn fire<E: Send + Sync + 'static>(payload: E) -> usize {
        crate::typed::event(payload)
    }

    /// Register a typed listener for events of type `E` (type-keyed dispatch).
    ///
    /// Counterpart to the string-keyed [`Event::listen`]. The closure receives a
    /// reference to the concrete event value when [`Event::fire`] / the free
    /// `event(payload)` dispatches one.
    pub fn listen_typed<E, F>(callback: F)
    where
        E: Send + Sync + 'static,
        F: Fn(&E) + Send + Sync + 'static,
    {
        crate::typed::listen::<E, F>(callback);
    }

    /// Dispatch a typed event after a delay of `delay_secs` seconds.
    ///
    /// Runs a real background thread that sleeps and then performs a synchronous
    /// typed dispatch. Fire-and-forget; no async runtime required. This is the
    /// target of the `dispatch!(delay: n, Event { .. })` macro form.
    pub fn dispatch_later<E: Send + Sync + 'static>(payload: E, delay_secs: u64) {
        crate::typed::event_later(payload, std::time::Duration::from_secs(delay_secs));
    }

    /// Number of typed listeners registered for event type `E`.
    pub fn typed_listener_count<E: 'static>() -> usize {
        crate::typed::typed_listener_count::<E>()
    }

    /// Get event dispatch history (for testing/debugging)
    pub fn history() -> Vec<(String, Value)> {
        let manager = GLOBAL_EVENT.read().unwrap();
        manager.history().to_vec()
    }

    /// Clear event history
    pub fn clear_history() {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.clear_history();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(serde::Serialize)]
    struct TestEvent {
        message: String,
    }

    /// Serializes every test in this module. They all share the process-global
    /// `GLOBAL_EVENT` singleton: history tests race a concurrent
    /// `clear_history()` / `dispatch()`, and listener tests race a concurrent
    /// `forget_all()` (which wipes *all* listeners regardless of event name).
    /// Holding this guard for the whole test makes those operations exclusive.
    /// `into_inner` ignores poisoning so a failing test does not cascade.
    static EVENT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_event_dispatch() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let event_name = format!("test.dispatch.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        let event = TestEvent {
            message: "test".to_string(),
        };

        let result = Event::dispatch(&event_name, event);
        assert!(result.is_ok());

        let history = Event::history();
        assert!(history.iter().any(|(name, _)| name == &event_name));
    }

    #[test]
    fn test_event_listen() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let event_name = format!("test.listen.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        Event::listen(&event_name, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(Event::has_listeners(&event_name));
        assert_eq!(Event::listener_count(&event_name), 1);
    }

    #[test]
    fn test_event_dispatch_and_listen() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let event_name = format!("test.both.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        Event::listen(&event_name, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = TestEvent {
            message: "test".to_string(),
        };

        Event::dispatch(&event_name, event).unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_forget() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let event_name = format!("test.forget.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        Event::listen(&event_name, |_| {});
        assert!(Event::has_listeners(&event_name));

        Event::forget(&event_name);
        assert!(!Event::has_listeners(&event_name));
    }

    #[test]
    fn test_event_forget_all() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let suffix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let event1 = format!("forget_all_test.event1.{}", suffix);
        let event2 = format!("forget_all_test.event2.{}", suffix);

        Event::listen(&event1, |_| {});
        Event::listen(&event2, |_| {});

        assert!(Event::has_listeners(&event1));
        assert!(Event::has_listeners(&event2));

        Event::forget_all();

        assert!(!Event::has_listeners(&event1));
        assert!(!Event::has_listeners(&event2));
    }

    #[test]
    fn test_event_listener_count() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let event_name = format!("test.count.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        Event::listen(&event_name, |_| {});
        Event::listen(&event_name, |_| {});

        assert_eq!(Event::listener_count(&event_name), 2);
    }

    #[test]
    fn test_event_history() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let old_count = Event::history().len();

        let event1 = TestEvent {
            message: "first".to_string(),
        };
        let event2 = TestEvent {
            message: "second".to_string(),
        };

        Event::dispatch("test.history.unique1", event1).unwrap();
        Event::dispatch("test.history.unique2", event2).unwrap();

        let history = Event::history();
        assert!(history.len() >= old_count + 2);
    }

    #[test]
    fn test_event_clear_history() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        Event::clear_history();

        let event = TestEvent {
            message: "test".to_string(),
        };

        Event::dispatch("test.clear.unique", event).unwrap();
        let count_before = Event::history().len();
        assert!(count_before >= 1);

        Event::clear_history();
        assert_eq!(Event::history().len(), 0);
    }
}
