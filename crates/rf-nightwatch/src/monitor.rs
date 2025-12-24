//! Event monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, OnceLock, RwLock};
use uuid::Uuid;

static MONITOR: OnceLock<Arc<Monitor>> = OnceLock::new();

/// Event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Request,
    Response,
    Query,
    Cache,
    Job,
    Mail,
    Notification,
    Event,
    Log,
    Exception,
    Custom(String),
}

/// A monitored event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: EventType,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, message: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            message: message.to_string(),
            timestamp: Utc::now(),
            metadata: None,
            duration_ms: None,
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// Event monitor
pub struct Monitor {
    events: RwLock<VecDeque<Event>>,
    max_events: usize,
}

impl Monitor {
    /// Create a new monitor
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_events)),
            max_events,
        }
    }

    /// Get the global monitor
    pub fn global() -> Arc<Self> {
        MONITOR
            .get_or_init(|| Arc::new(Self::new(10000)))
            .clone()
    }

    /// Record an event
    pub fn record(&self, event_type: EventType, message: &str) {
        self.add_event(Event::new(event_type, message));
    }

    /// Add an event
    pub fn add_event(&self, event: Event) {
        let mut events = self.events.write().unwrap();
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Get recent events
    pub fn recent(&self, limit: usize) -> Vec<Event> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get events by type
    pub fn by_type(&self, event_type: &EventType, limit: usize) -> Vec<Event> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .rev()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(event_type))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get event count
    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Clear all events
    pub fn clear(&self) {
        let mut events = self.events.write().unwrap();
        events.clear();
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new(10000)
    }
}
