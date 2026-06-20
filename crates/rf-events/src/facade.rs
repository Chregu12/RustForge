//! Event facade providing Laravel-style static event API

use crate::event_manager::GLOBAL_EVENT;
use serde::Serialize;
use serde_json::Value;

/// The EventFacade providing a static-like API for event dispatching.
///
/// Simple, Laravel-style API - no `.await` needed anywhere!
///
/// # Examples
///
/// ```rust,no_run
/// use rf_events::EventFacade;
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
///     EventFacade::dispatch("user.created", UserCreated {
///         user_id: 1,
///         email: "user@example.com".to_string(),
///     })?;
///
///     // Listen for events
///     EventFacade::listen("user.created", |event: &serde_json::Value| {
///         println!("Event received: {:?}", event);
///     });
///
///     // Check if event has listeners
///     if EventFacade::has_listeners("user.created") {
///         println!("Event has listeners");
///     }
///     Ok(())
/// }
/// ```
pub struct EventFacade;

impl EventFacade {
    /// Dispatch an event
    pub fn dispatch<T: Serialize>(event_name: &str, data: T) -> Result<(), String> {
        let value = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize event data: {}", e))?;

        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.dispatch(event_name, value)
    }

    /// Listen for an event
    pub fn listen<F>(event_name: &str, callback: F)
    where
        F: Fn(&Value) + Send + Sync + 'static,
    {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.listen(event_name, callback);
    }

    /// Check if an event has listeners
    pub fn has_listeners(event_name: &str) -> bool {
        let manager = GLOBAL_EVENT.read().unwrap();
        manager.has_listeners(event_name)
    }

    /// Get the number of listeners for an event
    pub fn listener_count(event_name: &str) -> usize {
        let manager = GLOBAL_EVENT.read().unwrap();
        manager.listener_count(event_name)
    }

    /// Forget all listeners for an event
    pub fn forget(event_name: &str) {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.forget(event_name);
    }

    /// Forget all event listeners
    pub fn forget_all() {
        let mut manager = GLOBAL_EVENT.write().unwrap();
        manager.forget_all();
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

    /// Serializes every test in this module. They all share the process-global
    /// event manager: the history test races a concurrent `clear_history()` /
    /// `dispatch()`, and the listener tests race `forget_all()` (which wipes all
    /// listeners regardless of event name). `into_inner` ignores poisoning so a
    /// failing test does not cascade into the others.
    static EVENT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[derive(serde::Serialize)]
    struct TestEvent {
        message: String,
    }

    #[test]
    fn test_event_dispatch() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::clear_history();

        let event = TestEvent {
            message: "test".to_string(),
        };

        let result = EventFacade::dispatch("test.event", event);
        assert!(result.is_ok());

        let history = EventFacade::history();
        assert!(!history.is_empty());
    }

    #[test]
    fn test_event_listen() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let event_name = format!("test.listen.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        EventFacade::listen(&event_name, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(EventFacade::has_listeners(&event_name));
        assert_eq!(EventFacade::listener_count(&event_name), 1);
    }

    #[test]
    fn test_event_forget() {
        let _guard = EVENT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::forget_all();

        EventFacade::listen("test.forget", |_| {});
        assert!(EventFacade::has_listeners("test.forget"));

        EventFacade::forget("test.forget");
        assert!(!EventFacade::has_listeners("test.forget"));
    }
}
