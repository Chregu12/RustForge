//! Event fake implementation for testing
//!
//! Provides a fake EventDispatcher implementation that records all dispatched events
//! and allows assertions on what was dispatched.

use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

/// Record of a dispatched event
#[derive(Debug, Clone)]
pub struct EventRecord {
    /// Event type identifier
    pub event_type: String,

    /// Serialized event payload
    pub payload: serde_json::Value,

    /// When the event was dispatched
    pub dispatched_at: chrono::DateTime<chrono::Utc>,
}

/// Event fake for testing
///
/// Records all events that are dispatched and provides
/// assertion methods to verify behavior.
///
/// # Example
///
/// ```ignore
/// use rf_testing::fakes::EventFake;
///
/// let fake = EventFake::new();
///
/// // Dispatch some events
/// fake.dispatch(UserCreated { user_id: 1 }).await?;
///
/// // Assert
/// fake.assert_dispatched("UserCreated");
/// fake.assert_dispatched_times("UserCreated", 1);
/// ```
#[derive(Clone)]
pub struct EventFake {
    records: Arc<Mutex<Vec<EventRecord>>>,
}

impl EventFake {
    /// Create a new event fake
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all dispatched events
    pub fn dispatched_events(&self) -> Vec<EventRecord> {
        self.records.lock().unwrap().clone()
    }

    /// Get dispatched events of a specific type
    pub fn dispatched_events_of_type(&self, event_type: &str) -> Vec<EventRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Assert that an event of the given type was dispatched
    ///
    /// # Panics
    ///
    /// Panics if no event of the given type was dispatched.
    pub fn assert_dispatched(&self, event_type: &str) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| r.event_type == event_type) {
            panic!(
                "Failed asserting that event '{}' was dispatched. Dispatched events: {:?}",
                event_type,
                records.iter().map(|r| &r.event_type).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that an event of the given type was dispatched exactly N times
    ///
    /// # Panics
    ///
    /// Panics if the event was not dispatched exactly N times.
    pub fn assert_dispatched_times(&self, event_type: &str, times: usize) {
        let records = self.records.lock().unwrap();
        let count = records
            .iter()
            .filter(|r| r.event_type == event_type)
            .count();

        if count != times {
            panic!(
                "Failed asserting that event '{}' was dispatched {} times. Actually dispatched {} times.",
                event_type, times, count
            );
        }
    }

    /// Assert that an event of the given type was NOT dispatched
    ///
    /// # Panics
    ///
    /// Panics if the event was dispatched.
    pub fn assert_not_dispatched(&self, event_type: &str) {
        let records = self.records.lock().unwrap();

        if records.iter().any(|r| r.event_type == event_type) {
            panic!(
                "Failed asserting that event '{}' was not dispatched",
                event_type
            );
        }
    }

    /// Assert that no events were dispatched at all
    ///
    /// # Panics
    ///
    /// Panics if any events were dispatched.
    pub fn assert_nothing_dispatched(&self) {
        let records = self.records.lock().unwrap();

        if !records.is_empty() {
            panic!(
                "Failed asserting that no events were dispatched. {} events were dispatched: {:?}",
                records.len(),
                records.iter().map(|r| &r.event_type).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that an event was dispatched with specific payload values
    ///
    /// Uses a closure to inspect the event payload.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fake.assert_dispatched_with("UserCreated", |event| {
    ///     event["user_id"] == 1
    /// });
    /// ```
    pub fn assert_dispatched_with<F>(&self, event_type: &str, predicate: F)
    where
        F: Fn(&serde_json::Value) -> bool,
    {
        let records = self.records.lock().unwrap();

        let found = records
            .iter()
            .filter(|r| r.event_type == event_type)
            .any(|r| predicate(&r.payload));

        if !found {
            panic!(
                "Failed asserting that event '{}' was dispatched with matching payload",
                event_type
            );
        }
    }

    /// Get the first dispatched event of a specific type and deserialize it
    ///
    /// Returns None if no event of that type was dispatched.
    pub fn first_dispatched<T>(&self, event_type: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let records = self.records.lock().unwrap();

        records
            .iter()
            .find(|r| r.event_type == event_type)
            .and_then(|r| serde_json::from_value(r.payload.clone()).ok())
    }

    /// Get all dispatched events of a specific type and deserialize them
    pub fn all_dispatched<T>(&self, event_type: &str) -> Vec<T>
    where
        T: DeserializeOwned,
    {
        let records = self.records.lock().unwrap();

        records
            .iter()
            .filter(|r| r.event_type == event_type)
            .filter_map(|r| serde_json::from_value(r.payload.clone()).ok())
            .collect()
    }

    /// Assert that events were dispatched in a specific order
    ///
    /// # Example
    ///
    /// ```ignore
    /// fake.assert_dispatched_in_order(&["UserCreated", "EmailSent", "WelcomeEmailSent"]);
    /// ```
    pub fn assert_dispatched_in_order(&self, event_types: &[&str]) {
        let records = self.records.lock().unwrap();
        let actual_types: Vec<&str> = records.iter().map(|r| r.event_type.as_str()).collect();

        // Find subsequence
        let mut iter = actual_types.iter();
        for expected in event_types {
            if !iter.any(|&t| t == *expected) {
                panic!(
                    "Failed asserting events were dispatched in order. Expected: {:?}, Actual: {:?}",
                    event_types, actual_types
                );
            }
        }
    }

    /// Get the number of times an event was dispatched
    pub fn dispatch_count(&self, event_type: &str) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.event_type == event_type)
            .count()
    }

    /// Clear all recorded events
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }

    /// Get the total number of dispatched events
    pub fn count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// Record an event dispatch manually (for testing)
    ///
    /// This is primarily used for testing the EventFake itself.
    /// In normal usage, the EventDispatcher trait implementation would call this internally.
    pub fn record_dispatch(&self, record: EventRecord) {
        self.records.lock().unwrap().push(record);
    }

    /// Record a simple event dispatch (helper for testing)
    pub fn dispatch_simple(&self, event_type: &str, payload: serde_json::Value) {
        self.record_dispatch(EventRecord {
            event_type: event_type.to_string(),
            payload,
            dispatched_at: chrono::Utc::now(),
        });
    }
}

impl Default for EventFake {
    fn default() -> Self {
        Self::new()
    }
}

// Note: The actual EventDispatcher trait implementation would be in the integration
// with rf-events. This fake is designed to be used standalone in tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_fake_creation() {
        let fake = EventFake::new();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_record_and_retrieve() {
        let fake = EventFake::new();

        let record = EventRecord {
            event_type: "TestEvent".to_string(),
            payload: serde_json::json!({"test": "data"}),
            dispatched_at: chrono::Utc::now(),
        };

        fake.record_dispatch(record);

        assert_eq!(fake.count(), 1);
        assert_eq!(fake.dispatch_count("TestEvent"), 1);
    }

    #[test]
    fn test_assert_dispatched() {
        let fake = EventFake::new();

        fake.dispatch_simple("TestEvent", serde_json::json!({}));
        fake.assert_dispatched("TestEvent");
    }

    #[test]
    #[should_panic(expected = "Failed asserting that event 'MissingEvent' was dispatched")]
    fn test_assert_dispatched_fails() {
        let fake = EventFake::new();
        fake.assert_dispatched("MissingEvent");
    }

    #[test]
    fn test_assert_dispatched_times() {
        let fake = EventFake::new();

        for _ in 0..3 {
            fake.dispatch_simple("TestEvent", serde_json::json!({}));
        }

        fake.assert_dispatched_times("TestEvent", 3);
    }

    #[test]
    #[should_panic(expected = "Failed asserting that event 'TestEvent' was dispatched 5 times")]
    fn test_assert_dispatched_times_fails() {
        let fake = EventFake::new();
        fake.dispatch_simple("TestEvent", serde_json::json!({}));

        fake.assert_dispatched_times("TestEvent", 5);
    }

    #[test]
    fn test_assert_not_dispatched() {
        let fake = EventFake::new();
        fake.assert_not_dispatched("TestEvent");
    }

    #[test]
    fn test_assert_nothing_dispatched() {
        let fake = EventFake::new();
        fake.assert_nothing_dispatched();
    }

    #[test]
    fn test_clear() {
        let fake = EventFake::new();

        fake.dispatch_simple("TestEvent", serde_json::json!({}));

        assert_eq!(fake.count(), 1);
        fake.clear();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_assert_dispatched_with() {
        let fake = EventFake::new();

        fake.dispatch_simple(
            "UserCreated",
            serde_json::json!({
                "user_id": 1,
                "email": "test@example.com"
            }),
        );

        fake.assert_dispatched_with("UserCreated", |payload| payload["user_id"] == 1);
    }

    #[test]
    fn test_assert_dispatched_in_order() {
        let fake = EventFake::new();

        fake.dispatch_simple("FirstEvent", serde_json::json!({}));
        fake.dispatch_simple("SecondEvent", serde_json::json!({}));
        fake.dispatch_simple("ThirdEvent", serde_json::json!({}));

        fake.assert_dispatched_in_order(&["FirstEvent", "SecondEvent", "ThirdEvent"]);
    }

    #[test]
    fn test_dispatched_events_of_type() {
        let fake = EventFake::new();

        fake.dispatch_simple("EventA", serde_json::json!({}));
        fake.dispatch_simple("EventB", serde_json::json!({}));
        fake.dispatch_simple("EventA", serde_json::json!({}));

        let events = fake.dispatched_events_of_type("EventA");
        assert_eq!(events.len(), 2);
    }
}
