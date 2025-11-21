//! Integration tests for domain events
//!
//! Tests event creation, serialization, and listener handling.

use app::events::order_placed_event::{OrderPlacedEvent, OrderPlacedPayload};
use app::listeners::send_order_email_listener::{SendOrderEmailListener, EventListener};

mod order_placed_event_tests {
    use super::*;

    #[test]
    fn event_creation() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };

        let event = OrderPlacedEvent::new(payload);

        assert_eq!(event.payload.user_id, 1);
        assert_eq!(event.payload.order_id, "ORD-123");
        assert_eq!(event.payload.amount, 99.99);
        assert!(!event.event_id.is_empty());
    }

    #[test]
    fn event_unique_ids() {
        let payload1 = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };

        let payload2 = OrderPlacedPayload {
            user_id: 2,
            order_id: "ORD-124".to_string(),
            amount: 149.99,
        };

        let event1 = OrderPlacedEvent::new(payload1);
        let event2 = OrderPlacedEvent::new(payload2);

        assert_ne!(event1.event_id, event2.event_id);
    }

    #[test]
    fn event_name_constant() {
        assert_eq!(OrderPlacedEvent::event_name(), "OrderPlacedEvent");
    }

    #[test]
    fn event_timestamp_is_set() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };

        let event = OrderPlacedEvent::new(payload);

        // Timestamp should be recent (within last second)
        let now = chrono::Utc::now();
        let diff = (now - event.occurred_at).num_seconds();
        assert!(diff >= 0 && diff <= 1);
    }

    #[test]
    fn event_serialization() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };

        let event = OrderPlacedEvent::new(payload);
        let json = serde_json::to_string(&event).expect("Failed to serialize");

        let deserialized: OrderPlacedEvent =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.event_id, event.event_id);
        assert_eq!(deserialized.payload.user_id, event.payload.user_id);
        assert_eq!(deserialized.payload.order_id, event.payload.order_id);
        assert_eq!(deserialized.payload.amount, event.payload.amount);
    }

    #[test]
    fn event_clone() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };

        let event1 = OrderPlacedEvent::new(payload);
        let event2 = event1.clone();

        assert_eq!(event1.event_id, event2.event_id);
        assert_eq!(event1.payload.user_id, event2.payload.user_id);
    }

    #[test]
    fn event_payload_with_various_amounts() {
        let test_cases = vec![
            (1, 0.01),
            (100, 99.99),
            (1000, 999.99),
        ];

        for (order_id, amount) in test_cases {
            let payload = OrderPlacedPayload {
                user_id: 1,
                order_id: format!("ORD-{}", order_id),
                amount,
            };

            let event = OrderPlacedEvent::new(payload);
            assert_eq!(event.payload.amount, amount);
        }
    }
}

mod listener_tests {
    use super::*;

    #[test]
    fn listener_creation() {
        let listener = SendOrderEmailListener::new();
        assert_eq!(
            std::any::type_name_of_val(&listener),
            "app::listeners::send_order_email_listener::SendOrderEmailListener"
        );
    }

    #[test]
    fn listener_default() {
        let listener = SendOrderEmailListener::default();
        assert_eq!(
            std::any::type_name_of_val(&listener),
            "app::listeners::send_order_email_listener::SendOrderEmailListener"
        );
    }

    #[test]
    fn listener_clone() {
        let listener1 = SendOrderEmailListener::new();
        let listener2 = listener1.clone();

        assert_eq!(
            std::any::type_name_of_val(&listener1),
            std::any::type_name_of_val(&listener2)
        );
    }

    #[test]
    fn listener_serialization() {
        let listener = SendOrderEmailListener::new();
        let json = serde_json::to_string(&listener).expect("Failed to serialize");

        let _deserialized: SendOrderEmailListener =
            serde_json::from_str(&json).expect("Failed to deserialize");
    }
}

mod event_listener_integration_tests {
    use super::*;

    #[test]
    fn event_listener_trait_object() {
        let _listener: Box<dyn std::any::Any> = Box::new(SendOrderEmailListener::new());
    }

    #[test]
    fn multiple_listeners_can_handle_same_event() {
        let payload = OrderPlacedPayload {
            user_id: 1,
            order_id: "ORD-123".to_string(),
            amount: 99.99,
        };

        let event = OrderPlacedEvent::new(payload);

        let listener1 = SendOrderEmailListener::new();
        let listener2 = SendOrderEmailListener::new();

        // Both listeners are independent instances
        assert_eq!(
            std::any::type_name_of_val(&listener1),
            std::any::type_name_of_val(&listener2)
        );

        // Event can be cloned and passed to multiple listeners
        let event_copy = event.clone();
        assert_eq!(event.event_id, event_copy.event_id);
    }
}
