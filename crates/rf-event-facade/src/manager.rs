//! Global event manager

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Global event manager instance
pub static GLOBAL_EVENT: Lazy<Arc<RwLock<EventManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(EventManager::new()))
});

/// Event listener callback type
pub type EventListener = Box<dyn Fn(&Value) + Send + Sync>;

/// Event manager that holds event listeners and dispatches events
#[derive(Default)]
pub struct EventManager {
    /// Event listeners mapped by event name
    listeners: HashMap<String, Vec<Arc<EventListener>>>,
    /// Event history (for testing/debugging)
    history: Vec<(String, Value)>,
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Dispatch an event
    pub fn dispatch(&mut self, event_name: &str, data: Value) -> Result<(), String> {
        // Store in history
        self.history.push((event_name.to_string(), data.clone()));

        // Call all listeners for this event
        if let Some(listeners) = self.listeners.get(event_name) {
            for listener in listeners {
                listener(&data);
            }
        }

        Ok(())
    }

    /// Listen for an event
    pub fn listen<F>(&mut self, event_name: &str, callback: F)
    where
        F: Fn(&Value) + Send + Sync + 'static,
    {
        let listener = Arc::new(Box::new(callback) as EventListener);
        self.listeners
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(listener);
    }

    /// Check if an event has listeners
    pub fn has_listeners(&self, event_name: &str) -> bool {
        self.listeners
            .get(event_name)
            .map(|l| !l.is_empty())
            .unwrap_or(false)
    }

    /// Get the number of listeners for an event
    pub fn listener_count(&self, event_name: &str) -> usize {
        self.listeners
            .get(event_name)
            .map(|l| l.len())
            .unwrap_or(0)
    }

    /// Forget all listeners for an event
    pub fn forget(&mut self, event_name: &str) {
        self.listeners.remove(event_name);
    }

    /// Forget all listeners
    pub fn forget_all(&mut self) {
        self.listeners.clear();
    }

    /// Get event history (for testing)
    pub fn history(&self) -> &[(String, Value)] {
        &self.history
    }

    /// Clear event history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_event_manager_new() {
        let manager = EventManager::new();
        assert_eq!(manager.listener_count("test"), 0);
        assert!(!manager.has_listeners("test"));
    }

    #[test]
    fn test_event_manager_listen() {
        let mut manager = EventManager::new();
        let called = Arc::new(AtomicUsize::new(0));
        let called_clone = called.clone();

        manager.listen("test.event", move |_data| {
            called_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(manager.has_listeners("test.event"));
        assert_eq!(manager.listener_count("test.event"), 1);
    }

    #[test]
    fn test_event_manager_dispatch() {
        let mut manager = EventManager::new();
        let called = Arc::new(AtomicUsize::new(0));
        let called_clone = called.clone();

        manager.listen("test.event", move |_data| {
            called_clone.fetch_add(1, Ordering::SeqCst);
        });

        let data = serde_json::json!({ "message": "test" });
        manager.dispatch("test.event", data).unwrap();

        assert_eq!(called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_manager_multiple_listeners() {
        let mut manager = EventManager::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter1 = counter.clone();
        manager.listen("test.event", move |_| {
            counter1.fetch_add(1, Ordering::SeqCst);
        });

        let counter2 = counter.clone();
        manager.listen("test.event", move |_| {
            counter2.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(manager.listener_count("test.event"), 2);

        manager.dispatch("test.event", serde_json::json!({})).unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_event_manager_forget() {
        let mut manager = EventManager::new();
        manager.listen("test.event", |_| {});

        assert!(manager.has_listeners("test.event"));

        manager.forget("test.event");
        assert!(!manager.has_listeners("test.event"));
    }

    #[test]
    fn test_event_manager_forget_all() {
        let mut manager = EventManager::new();
        manager.listen("event1", |_| {});
        manager.listen("event2", |_| {});

        assert!(manager.has_listeners("event1"));
        assert!(manager.has_listeners("event2"));

        manager.forget_all();
        assert!(!manager.has_listeners("event1"));
        assert!(!manager.has_listeners("event2"));
    }

    #[test]
    fn test_event_manager_history() {
        let mut manager = EventManager::new();
        let data = serde_json::json!({ "test": true });

        manager.dispatch("test.event", data.clone()).unwrap();

        let history = manager.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].0, "test.event");
        assert_eq!(history[0].1, data);
    }

    #[test]
    fn test_event_manager_clear_history() {
        let mut manager = EventManager::new();
        manager.dispatch("test.event", serde_json::json!({})).unwrap();

        assert_eq!(manager.history().len(), 1);

        manager.clear_history();
        assert_eq!(manager.history().len(), 0);
    }
}
