//! # Model Events System
//!
//! Provides lifecycle hooks for model operations like creating, updating, deleting.
//! Events can be used for validation, logging, cache invalidation, etc.
//!
//! ## Supported Events
//!
//! - `creating` - Before a model is inserted
//! - `created` - After a model is inserted
//! - `updating` - Before a model is updated
//! - `updated` - After a model is updated
//! - `saving` - Before a model is saved (insert or update)
//! - `saved` - After a model is saved
//! - `deleting` - Before a model is deleted
//! - `deleted` - After a model is deleted
//! - `restoring` - Before a soft-deleted model is restored
//! - `restored` - After a soft-deleted model is restored
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//! use async_trait::async_trait;
//!
//! #[derive(Clone, Debug)]
//! struct User {
//!     id: i64,
//!     name: String,
//!     email: String,
//!     created_at: chrono::DateTime<Utc>,
//! }
//!
//! #[async_trait]
//! impl ModelEvents for User {
//!     async fn creating(&mut self) -> EventResult {
//!         // Set created_at timestamp
//!         self.created_at = Utc::now();
//!         Ok(())
//!     }
//!
//!     async fn created(&self) -> EventResult {
//!         // Send welcome email
//!         send_welcome_email(&self.email).await?;
//!         Ok(())
//!     }
//!
//!     async fn updating(&mut self) -> EventResult {
//!         // Validate changes
//!         if self.email.is_empty() {
//!             return Err(EventError::ValidationFailed("Email required".to_string()));
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Event system errors
#[derive(Error, Debug)]
pub enum EventError {
    #[error("Event validation failed: {0}")]
    ValidationFailed(String),

    #[error("Event handler failed: {0}")]
    HandlerFailed(String),

    #[error("Event propagation stopped")]
    PropagationStopped,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Event not found: {0}")]
    NotFound(String),
}

pub type EventResult = Result<(), EventError>;

/// Model event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelEvent {
    Creating,
    Created,
    Updating,
    Updated,
    Saving,
    Saved,
    Deleting,
    Deleted,
    Restoring,
    Restored,
}

impl ModelEvent {
    /// Check if this is a "before" event
    pub fn is_before(&self) -> bool {
        matches!(
            self,
            Self::Creating | Self::Updating | Self::Saving | Self::Deleting | Self::Restoring
        )
    }

    /// Check if this is an "after" event
    pub fn is_after(&self) -> bool {
        !self.is_before()
    }

    /// Get the event name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Creating => "creating",
            Self::Created => "created",
            Self::Updating => "updating",
            Self::Updated => "updated",
            Self::Saving => "saving",
            Self::Saved => "saved",
            Self::Deleting => "deleting",
            Self::Deleted => "deleted",
            Self::Restoring => "restoring",
            Self::Restored => "restored",
        }
    }
}

/// Trait for models with lifecycle events
#[async_trait]
pub trait ModelEvents: Send + Sync {
    /// Called before a model is created
    async fn creating(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after a model is created
    async fn created(&self) -> EventResult {
        Ok(())
    }

    /// Called before a model is updated
    async fn updating(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after a model is updated
    async fn updated(&self) -> EventResult {
        Ok(())
    }

    /// Called before a model is saved (created or updated)
    async fn saving(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after a model is saved
    async fn saved(&self) -> EventResult {
        Ok(())
    }

    /// Called before a model is deleted
    async fn deleting(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after a model is deleted
    async fn deleted(&self) -> EventResult {
        Ok(())
    }

    /// Called before a soft-deleted model is restored
    async fn restoring(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after a soft-deleted model is restored
    async fn restored(&self) -> EventResult {
        Ok(())
    }
}

/// Event listener function type
pub type EventListener = Arc<dyn Fn(&EventContext) -> EventResult + Send + Sync>;

/// Context passed to event listeners
#[derive(Debug, Clone)]
pub struct EventContext {
    pub event: ModelEvent,
    pub model_type: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl EventContext {
    /// Create a new event context
    pub fn new(event: ModelEvent, model_type: impl Into<String>) -> Self {
        Self {
            event,
            model_type: model_type.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Global event dispatcher
pub struct EventDispatcher {
    listeners: Arc<RwLock<HashMap<String, Vec<EventListener>>>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an event listener
    pub async fn listen<F>(&self, event: ModelEvent, model_type: &str, listener: F)
    where
        F: Fn(&EventContext) -> EventResult + Send + Sync + 'static,
    {
        let key = format!("{}::{}", model_type, event.name());
        let mut listeners = self.listeners.write().await;
        listeners
            .entry(key)
            .or_insert_with(Vec::new)
            .push(Arc::new(listener));
    }

    /// Dispatch an event to all registered listeners
    pub async fn dispatch(&self, context: &EventContext) -> EventResult {
        let key = format!("{}::{}", context.model_type, context.event.name());
        let listeners = self.listeners.read().await;

        if let Some(event_listeners) = listeners.get(&key) {
            for listener in event_listeners {
                listener(context)?;
            }
        }

        Ok(())
    }

    /// Remove all listeners for a specific event
    pub async fn forget(&self, event: ModelEvent, model_type: &str) {
        let key = format!("{}::{}", model_type, event.name());
        let mut listeners = self.listeners.write().await;
        listeners.remove(&key);
    }

    /// Clear all event listeners
    pub async fn clear(&self) {
        let mut listeners = self.listeners.write().await;
        listeners.clear();
    }

    /// Get the number of listeners for an event
    pub async fn listener_count(&self, event: ModelEvent, model_type: &str) -> usize {
        let key = format!("{}::{}", model_type, event.name());
        let listeners = self.listeners.read().await;
        listeners.get(&key).map(|l| l.len()).unwrap_or(0)
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Observer pattern for model events
pub struct EventObserver {
    dispatcher: Arc<EventDispatcher>,
}

impl EventObserver {
    /// Create a new event observer
    pub fn new() -> Self {
        Self {
            dispatcher: Arc::new(EventDispatcher::new()),
        }
    }

    /// Get the underlying dispatcher
    pub fn dispatcher(&self) -> Arc<EventDispatcher> {
        self.dispatcher.clone()
    }

    /// Register a creating event listener
    pub async fn creating<F>(&self, model_type: &str, listener: F)
    where
        F: Fn(&EventContext) -> EventResult + Send + Sync + 'static,
    {
        self.dispatcher
            .listen(ModelEvent::Creating, model_type, listener)
            .await;
    }

    /// Register a created event listener
    pub async fn created<F>(&self, model_type: &str, listener: F)
    where
        F: Fn(&EventContext) -> EventResult + Send + Sync + 'static,
    {
        self.dispatcher
            .listen(ModelEvent::Created, model_type, listener)
            .await;
    }

    /// Register an updating event listener
    pub async fn updating<F>(&self, model_type: &str, listener: F)
    where
        F: Fn(&EventContext) -> EventResult + Send + Sync + 'static,
    {
        self.dispatcher
            .listen(ModelEvent::Updating, model_type, listener)
            .await;
    }

    /// Register an updated event listener
    pub async fn updated<F>(&self, model_type: &str, listener: F)
    where
        F: Fn(&EventContext) -> EventResult + Send + Sync + 'static,
    {
        self.dispatcher
            .listen(ModelEvent::Updated, model_type, listener)
            .await;
    }

    /// Trigger an event
    pub async fn fire(&self, context: EventContext) -> EventResult {
        self.dispatcher.dispatch(&context).await
    }
}

impl Default for EventObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_event_is_before() {
        assert!(ModelEvent::Creating.is_before());
        assert!(ModelEvent::Updating.is_before());
        assert!(ModelEvent::Saving.is_before());
        assert!(ModelEvent::Deleting.is_before());
        assert!(!ModelEvent::Created.is_before());
        assert!(!ModelEvent::Updated.is_before());
    }

    #[test]
    fn test_model_event_is_after() {
        assert!(ModelEvent::Created.is_after());
        assert!(ModelEvent::Updated.is_after());
        assert!(ModelEvent::Saved.is_after());
        assert!(ModelEvent::Deleted.is_after());
        assert!(!ModelEvent::Creating.is_after());
        assert!(!ModelEvent::Updating.is_after());
    }

    #[test]
    fn test_model_event_name() {
        assert_eq!(ModelEvent::Creating.name(), "creating");
        assert_eq!(ModelEvent::Created.name(), "created");
        assert_eq!(ModelEvent::Updating.name(), "updating");
        assert_eq!(ModelEvent::Updated.name(), "updated");
    }

    #[test]
    fn test_event_context() {
        let context = EventContext::new(ModelEvent::Creating, "User")
            .with_metadata("id", "1")
            .with_metadata("action", "register");

        assert_eq!(context.event, ModelEvent::Creating);
        assert_eq!(context.model_type, "User");
        assert_eq!(context.get_metadata("id").unwrap(), "1");
        assert_eq!(context.get_metadata("action").unwrap(), "register");
    }

    #[tokio::test]
    async fn test_event_dispatcher() {
        let dispatcher = EventDispatcher::new();

        // Register a listener
        dispatcher
            .listen(ModelEvent::Creating, "User", |ctx| {
                assert_eq!(ctx.event, ModelEvent::Creating);
                Ok(())
            })
            .await;

        assert_eq!(
            dispatcher
                .listener_count(ModelEvent::Creating, "User")
                .await,
            1
        );

        // Dispatch event
        let context = EventContext::new(ModelEvent::Creating, "User");
        dispatcher.dispatch(&context).await.unwrap();

        // Forget listener
        dispatcher.forget(ModelEvent::Creating, "User").await;
        assert_eq!(
            dispatcher
                .listener_count(ModelEvent::Creating, "User")
                .await,
            0
        );
    }
}
