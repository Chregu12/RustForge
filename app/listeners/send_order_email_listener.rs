use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// SendOrderEmail Listener
///
/// Handles events and executes specific actions in response.
/// Implement the `handle()` method to define event processing logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOrderEmailListener;

/// Generic trait for event listeners
#[async_trait]
pub trait EventListener<E> {
    /// Handle an incoming event
    async fn handle(&self, event: &E) -> Result<(), ListenerError>;
}

/// Errors that can occur during listener execution
#[derive(Debug, thiserror::Error)]
pub enum ListenerError {
    #[error("Listener execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Event processing error: {0}")]
    ProcessingError(String),
}

impl SendOrderEmailListener {
    /// Creates a new SendOrderEmail listener instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendOrderEmailListener {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_listener_instance() {
        let listener = SendOrderEmailListener::new();
        assert_eq!(std::any::type_name_of_val(&listener), "app::listeners::send_order_email_listener::SendOrderEmailListener");
    }

    #[test]
    fn default_creates_listener() {
        let listener = SendOrderEmailListener::default();
        assert_eq!(std::any::type_name_of_val(&listener), "app::listeners::send_order_email_listener::SendOrderEmailListener");
    }

    #[test]
    fn listener_is_cloneable() {
        let listener1 = SendOrderEmailListener::new();
        let listener2 = listener1.clone();
        // Both listeners are independent instances
        assert_eq!(std::any::type_name_of_val(&listener1), std::any::type_name_of_val(&listener2));
    }
}
