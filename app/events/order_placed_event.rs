use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// OrderPlaced Event
///
/// Fired when an order is successfully placed in the system.
/// Multiple listeners can handle this event to trigger various side effects
/// such as sending emails, logging, or triggering notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedEvent {
    /// Unique event ID
    pub event_id: String,

    /// Timestamp when the event occurred
    pub occurred_at: DateTime<Utc>,

    /// Event-specific payload data
    pub payload: OrderPlacedPayload,
}

/// OrderPlaced Event Payload
///
/// Contains the data associated with an order placement event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedPayload {
    /// User ID who placed the order
    pub user_id: i64,
    /// Order ID
    pub order_id: String,
    /// Order total amount
    pub amount: f64,
}

impl OrderPlacedEvent {
    /// Creates a new OrderPlaced event instance
    pub fn new(payload: OrderPlacedPayload) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            payload,
        }
    }

    /// Returns the event name (for event bus routing)
    pub fn event_name() -> &'static str {
        "OrderPlacedEvent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_event_with_unique_id() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };
        let event1 = OrderPlacedEvent::new(payload.clone());
        let event2 = OrderPlacedEvent::new(payload);

        assert_ne!(event1.event_id, event2.event_id);
    }

    #[test]
    fn event_name_is_correct() {
        assert_eq!(OrderPlacedEvent::event_name(), "OrderPlacedEvent");
    }

    #[test]
    fn payload_is_correctly_stored() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };
        let event = OrderPlacedEvent::new(payload);

        assert_eq!(event.payload.user_id, 1);
        assert_eq!(event.payload.order_id, "ORD-123");
        assert_eq!(event.payload.amount, 99.99);
    }
}
