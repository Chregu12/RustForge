use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Event: OrderPlaced
///
/// Dieses Event wird ausgelöst, wenn ein bestimmtes Ereignis im System eintritt.
/// Es kann von mehreren Listenern verarbeitet werden, um verschiedene
/// Nebeneffekte (z.B. E-Mail versenden, Logging, Benachrichtigungen) auszulösen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedEvent {
    /// Eindeutige Event-ID
    pub event_id: String,
    
    /// Zeitpunkt des Event-Auftretens
    pub occurred_at: DateTime<Utc>,
    
    /// Event-spezifische Payload-Daten
    pub payload: OrderPlacedPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedPayload {
    // TODO: Füge hier die Event-spezifischen Felder hinzu
    // Beispiel:
    // pub user_id: i64,
    // pub action: String,
    // pub metadata: serde_json::Value,
}

impl OrderPlacedEvent {
    /// Erstellt eine neue Event-Instanz
    pub fn new(payload: OrderPlacedPayload) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            payload,
        }
    }
    
    /// Gibt den Event-Namen zurück (für Event-Bus-Routing)
    pub fn event_name() -> &'static str {
        "OrderPlacedEvent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_event_with_unique_id() {
        let payload = OrderPlacedPayload {};
        let event1 = OrderPlacedEvent::new(payload.clone());
        let event2 = OrderPlacedEvent::new(payload);
        
        assert_ne!(event1.event_id, event2.event_id);
    }
    
    #[test]
    fn event_name_is_correct() {
        assert_eq!(OrderPlacedEvent::event_name(), "OrderPlacedEvent");
    }
}
