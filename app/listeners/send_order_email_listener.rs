use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Listener: SendOrderEmail
///
/// Dieser Listener reagiert auf Events und führt spezifische Aktionen aus.
/// Implementiere die `handle()` Methode, um die Event-Verarbeitungslogik zu definieren.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOrderEmailListener;

#[async_trait]
pub trait EventListener<E> {
    async fn handle(&self, event: &E) -> Result<(), ListenerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ListenerError {
    #[error("Listener execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Event processing error: {0}")]
    ProcessingError(String),
}

impl SendOrderEmailListener {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendOrderEmailListener {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implementiere EventListener für dein spezifisches Event
// Beispiel:
// #[async_trait]
// impl EventListener<OrderPlacedEvent> for SendOrderEmailListener {
//     async fn handle(&self, event: &OrderPlacedEvent) -> Result<(), ListenerError> {
//         // Verarbeite das Event
//         println!("Handling event: {:?}", event);
//         
//         // Führe Aktionen aus (z.B. E-Mail versenden)
//         // self.send_email(&event.payload).await?;
//         
//         Ok(())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_listener_instance() {
        let listener = SendOrderEmailListener::new();
        assert!(true); // Placeholder test
    }
    
    // TODO: Füge hier weitere Tests hinzu
    // #[tokio::test]
    // async fn handles_event_successfully() {
    //     let listener = SendOrderEmailListener::new();
    //     let event = OrderPlacedEvent::new(...);
    //     let result = listener.handle(&event).await;
    //     assert!(result.is_ok());
    // }
}
