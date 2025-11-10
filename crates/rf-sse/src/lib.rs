//! Server-Sent Events (SSE) for RustForge
//!
//! This crate provides SSE streaming for real-time updates.

use axum::{
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        IntoResponse,
    },
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

pub use axum::response::sse::KeepAlive as SseKeepAlive;

/// SSE errors
#[derive(Debug, Error)]
pub enum SseError {
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Failed to send event: {0}")]
    SendError(String),
}

pub type SseResult<T> = Result<T, SseError>;

/// Event builder for SSE
#[derive(Debug, Clone)]
pub struct Event {
    id: Option<String>,
    event: Option<String>,
    data: String,
    retry: Option<u64>,
}

impl Event {
    /// Create a new event
    pub fn new() -> Self {
        Self {
            id: None,
            event: None,
            data: String::new(),
            retry: None,
        }
    }

    /// Set event ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set event type
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Set event data
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = data.into();
        self
    }

    /// Set event data as JSON
    pub fn json<T: Serialize>(mut self, data: &T) -> Result<Self, serde_json::Error> {
        self.data = serde_json::to_string(data)?;
        Ok(self)
    }

    /// Set retry timeout (milliseconds)
    pub fn retry(mut self, ms: u64) -> Self {
        self.retry = Some(ms);
        self
    }

    /// Convert to Axum SSE event
    pub fn into_sse_event(self) -> SseEvent {
        let mut event = SseEvent::default().data(self.data);

        if let Some(id) = self.id {
            event = event.id(id);
        }

        if let Some(event_type) = self.event {
            event = event.event(event_type);
        }

        event
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE channel for broadcasting events
struct Channel {
    sender: broadcast::Sender<Event>,
}

impl Channel {
    fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    fn send(&self, event: Event) -> SseResult<()> {
        self.sender
            .send(event)
            .map(|_| ())
            .map_err(|e| SseError::SendError(e.to_string()))
    }

    fn subscribe(&self) -> BroadcastStream<Event> {
        BroadcastStream::new(self.sender.subscribe())
    }
}

/// SSE connection manager
#[derive(Clone)]
pub struct SseManager {
    channels: Arc<RwLock<HashMap<String, Channel>>>,
    default_capacity: usize,
}

impl SseManager {
    /// Create a new SSE manager
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            default_capacity: 100,
        }
    }

    /// Create a new SSE manager with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            default_capacity: capacity,
        }
    }

    /// Create or get a channel
    async fn get_or_create_channel(&self, channel: &str) -> Channel {
        let mut channels = self.channels.write().await;
        channels
            .entry(channel.to_string())
            .or_insert_with(|| Channel::new(self.default_capacity))
            .clone()
    }

    /// Subscribe to a channel
    pub async fn subscribe(&self, channel: &str) -> EventStream {
        let ch = self.get_or_create_channel(channel).await;
        EventStream::new(ch.subscribe())
    }

    /// Broadcast an event to a channel
    pub async fn broadcast(&self, channel: &str, event: Event) -> SseResult<()> {
        let ch = self.get_or_create_channel(channel).await;
        ch.send(event)
    }

    /// Remove a channel
    pub async fn remove_channel(&self, channel: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(channel);
    }

    /// Get number of active channels
    pub async fn channel_count(&self) -> usize {
        let channels = self.channels.read().await;
        channels.len()
    }
}

impl Default for SseManager {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE event stream
pub struct EventStream {
    inner: BroadcastStream<Event>,
}

impl EventStream {
    fn new(stream: BroadcastStream<Event>) -> Self {
        Self { inner: stream }
    }
}

impl Stream for EventStream {
    type Item = Result<SseEvent, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                Poll::Ready(Some(Ok(event.into_sse_event())))
            }
            Poll::Ready(Some(Err(_))) => {
                // Lagged, skip
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create SSE response
pub fn create_sse_stream(stream: EventStream) -> impl IntoResponse {
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let event = Event::new()
            .id("123")
            .event("message")
            .data("Hello, World!")
            .retry(5000);

        assert_eq!(event.id, Some("123".to_string()));
        assert_eq!(event.event, Some("message".to_string()));
        assert_eq!(event.data, "Hello, World!");
        assert_eq!(event.retry, Some(5000));
    }

    #[test]
    fn test_event_json() {
        #[derive(Serialize)]
        struct TestData {
            message: String,
        }

        let data = TestData {
            message: "Hello".to_string(),
        };

        let event = Event::new().json(&data).unwrap();
        assert!(event.data.contains("Hello"));
    }

    #[tokio::test]
    async fn test_sse_manager() {
        let manager = SseManager::new();
        assert_eq!(manager.channel_count().await, 0);

        // Subscribe creates channel
        let _stream = manager.subscribe("test").await;
        assert_eq!(manager.channel_count().await, 1);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let manager = SseManager::new();

        let event = Event::new().data("test message");
        let result = manager.broadcast("test", event).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_channel() {
        let manager = SseManager::new();

        manager.subscribe("test").await;
        assert_eq!(manager.channel_count().await, 1);

        manager.remove_channel("test").await;
        assert_eq!(manager.channel_count().await, 0);
    }

    #[tokio::test]
    async fn test_with_capacity() {
        let manager = SseManager::with_capacity(50);
        assert_eq!(manager.default_capacity, 50);
    }

    #[test]
    fn test_event_default() {
        let event = Event::default();
        assert!(event.id.is_none());
        assert!(event.event.is_none());
        assert_eq!(event.data, "");
        assert!(event.retry.is_none());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let manager = SseManager::new();

        let _stream1 = manager.subscribe("test").await;
        let _stream2 = manager.subscribe("test").await;

        // Both should subscribe to same channel
        assert_eq!(manager.channel_count().await, 1);

        let event = Event::new().data("broadcast");
        let result = manager.broadcast("test", event).await;
        assert!(result.is_ok());
    }
}
