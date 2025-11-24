//! Event facade providing Laravel-style static event API

use crate::manager::GLOBAL_EVENT;
use serde::Serialize;
use serde_json::Value;

/// The Event facade providing a static-like API for event dispatching.
///
/// This is the main entry point for event operations in your application.
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
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Dispatch an event
/// Event::dispatch("user.created", UserCreated {
///     user_id: 1,
///     email: "user@example.com".to_string(),
/// }).await?;
///
/// // Listen for events
/// Event::listen("user.created", |event: &Value| {
///     println!("Event received: {:?}", event);
/// }).await;
///
/// // Check if event has listeners
/// if Event::has_listeners("user.created").await {
///     println!("Event has listeners");
/// }
/// # Ok(())
/// # }
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
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Event::dispatch("order.placed", OrderPlaced {
    ///     order_id: 123,
    ///     total: 99.99,
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn dispatch<T: Serialize>(event_name: &str, data: T) -> Result<(), String> {
        let value = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize event data: {}", e))?;

        let mut manager = GLOBAL_EVENT.write();
        manager.dispatch(event_name, value)
    }

    /// Listen for an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// # async fn example() {
    /// Event::listen("user.created", |data| {
    ///     println!("User created: {:?}", data);
    /// }).await;
    /// # }
    /// ```
    pub async fn listen<F>(event_name: &str, callback: F)
    where
        F: Fn(&Value) + Send + Sync + 'static,
    {
        let mut manager = GLOBAL_EVENT.write();
        manager.listen(event_name, callback);
    }

    /// Check if an event has listeners
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// # async fn example() {
    /// if Event::has_listeners("user.created").await {
    ///     println!("Event has listeners");
    /// }
    /// # }
    /// ```
    pub async fn has_listeners(event_name: &str) -> bool {
        let manager = GLOBAL_EVENT.read();
        manager.has_listeners(event_name)
    }

    /// Get the number of listeners for an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// # async fn example() {
    /// let count = Event::listener_count("user.created").await;
    /// println!("Number of listeners: {}", count);
    /// # }
    /// ```
    pub async fn listener_count(event_name: &str) -> usize {
        let manager = GLOBAL_EVENT.read();
        manager.listener_count(event_name)
    }

    /// Forget all listeners for an event
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// # async fn example() {
    /// Event::forget("user.created").await;
    /// # }
    /// ```
    pub async fn forget(event_name: &str) {
        let mut manager = GLOBAL_EVENT.write();
        manager.forget(event_name);
    }

    /// Forget all event listeners
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_event_facade::Event;
    ///
    /// # async fn example() {
    /// Event::forget_all().await;
    /// # }
    /// ```
    pub async fn forget_all() {
        let mut manager = GLOBAL_EVENT.write();
        manager.forget_all();
    }

    /// Get event dispatch history (for testing/debugging)
    pub async fn history() -> Vec<(String, Value)> {
        let manager = GLOBAL_EVENT.read();
        manager.history().to_vec()
    }

    /// Clear event history
    pub async fn clear_history() {
        let mut manager = GLOBAL_EVENT.write();
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

    #[tokio::test]
    async fn test_event_dispatch() {
        Event::clear_history().await;

        let event = TestEvent {
            message: "test".to_string(),
        };

        let result = Event::dispatch("test.event", event).await;
        assert!(result.is_ok());

        let history = Event::history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].0, "test.event");
    }

    #[tokio::test]
    async fn test_event_listen() {
        Event::forget_all().await;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        Event::listen("test.listen", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        assert!(Event::has_listeners("test.listen").await);
        assert_eq!(Event::listener_count("test.listen").await, 1);
    }

    #[tokio::test]
    async fn test_event_dispatch_and_listen() {
        Event::forget_all().await;
        Event::clear_history().await;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        Event::listen("test.both", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        let event = TestEvent {
            message: "test".to_string(),
        };

        Event::dispatch("test.both", event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_forget() {
        Event::forget_all().await;

        Event::listen("test.forget", |_| {}).await;
        assert!(Event::has_listeners("test.forget").await);

        Event::forget("test.forget").await;
        assert!(!Event::has_listeners("test.forget").await);
    }

    #[tokio::test]
    async fn test_event_forget_all() {
        Event::forget_all().await;

        Event::listen("forget_all_test.event1", |_| {}).await;
        Event::listen("forget_all_test.event2", |_| {}).await;

        assert!(Event::has_listeners("forget_all_test.event1").await);
        assert!(Event::has_listeners("forget_all_test.event2").await);

        Event::forget_all().await;

        assert!(!Event::has_listeners("forget_all_test.event1").await);
        assert!(!Event::has_listeners("forget_all_test.event2").await);
    }

    #[tokio::test]
    async fn test_event_listener_count() {
        let event_name = format!("test.count.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        Event::listen(&event_name, |_| {}).await;
        Event::listen(&event_name, |_| {}).await;

        assert_eq!(Event::listener_count(&event_name).await, 2);
    }

    #[tokio::test]
    async fn test_event_history() {
        let old_count = Event::history().await.len();

        let event1 = TestEvent {
            message: "first".to_string(),
        };
        let event2 = TestEvent {
            message: "second".to_string(),
        };

        Event::dispatch("test.history.unique1", event1).await.unwrap();
        Event::dispatch("test.history.unique2", event2).await.unwrap();

        let history = Event::history().await;
        assert!(history.len() >= old_count + 2);
    }

    #[tokio::test]
    async fn test_event_clear_history() {
        Event::clear_history().await;

        let event = TestEvent {
            message: "test".to_string(),
        };

        Event::dispatch("test.clear.unique", event).await.unwrap();
        let count_before = Event::history().await.len();
        assert!(count_before >= 1);

        Event::clear_history().await;
        assert_eq!(Event::history().await.len(), 0);
    }
}
