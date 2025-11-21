//! Comprehensive WebSocket broadcasting integration tests

use futures::{SinkExt, StreamExt};
use rf_broadcasting::{
    Broadcast, BroadcastDriver, Broadcaster, ClientMessage, RedisBroadcastDriver, ServerMessage,
    WebSocketServer,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to check if Redis is available
async fn redis_available() -> bool {
    tokio::net::TcpStream::connect("127.0.0.1:6379")
        .await
        .is_ok()
}

/// Helper to create Redis broadcast driver
async fn create_redis_driver() -> RedisBroadcastDriver {
    let config = deadpool_redis::Config {
        url: Some("redis://127.0.0.1:6379".to_string()),
        connection: None,
        pool: None,
    };

    let pool = config
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();

    RedisBroadcastDriver::new(pool)
}

/// Test event for broadcasting
#[derive(Debug, Clone)]
struct TestEvent {
    message: String,
    data: i32,
}

impl Broadcast for TestEvent {
    fn broadcast_on(&self) -> Vec<String> {
        vec!["test-channel".to_string()]
    }

    fn broadcast_with(&self) -> serde_json::Value {
        json!({
            "message": self.message,
            "data": self.data,
        })
    }
}

#[tokio::test]
async fn test_message_serialization() {
    // Test client messages
    let subscribe = ClientMessage::Subscribe {
        channel: "test".to_string(),
        auth: Some("token123".to_string()),
    };

    let json = serde_json::to_string(&subscribe).unwrap();
    let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();

    match deserialized {
        ClientMessage::Subscribe { channel, auth } => {
            assert_eq!(channel, "test");
            assert_eq!(auth, Some("token123".to_string()));
        }
        _ => panic!("Wrong message type"),
    }

    // Test server messages
    let event = ServerMessage::Event {
        channel: "orders".to_string(),
        event: "OrderCreated".to_string(),
        data: json!({"id": 1}),
    };

    let json = event.to_json().unwrap();
    let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();

    match deserialized {
        ServerMessage::Event {
            channel,
            event,
            data,
        } => {
            assert_eq!(channel, "orders");
            assert_eq!(event, "OrderCreated");
            assert_eq!(data["id"], 1);
        }
        _ => panic!("Wrong message type"),
    }
}

#[tokio::test]
async fn test_redis_broadcast_driver() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping Redis tests: Redis not available");
        eprintln!("   Start Redis with: docker run -p 6379:6379 redis");
        return;
    }

    let driver = create_redis_driver().await;

    // Subscribe to channel
    driver
        .subscribe(&["test-redis".to_string()])
        .await
        .unwrap();

    // Broadcast message
    let result = driver
        .broadcast(
            &["test-redis".to_string()],
            "TestEvent",
            json!({"test": "data"}),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_broadcaster_with_event() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping Redis tests: Redis not available");
        return;
    }

    let driver = create_redis_driver().await;
    let broadcaster = Broadcaster::new(Arc::new(driver));

    let event = TestEvent {
        message: "Test message".to_string(),
        data: 42,
    };

    let result = broadcaster.broadcast(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_broadcast_to_multiple_channels() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping Redis tests: Redis not available");
        return;
    }

    let driver = create_redis_driver().await;

    let channels = vec![
        "channel1".to_string(),
        "channel2".to_string(),
        "channel3".to_string(),
    ];

    let result = driver
        .broadcast(&channels, "MultiChannelEvent", json!({"value": 123}))
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_channel_registry() {
    use rf_broadcasting::websocket::ChannelRegistry;

    let registry = ChannelRegistry::new();

    // Create a channel and subscriber
    let tx = registry.get_or_create("test-cleanup").await;
    let _rx = tx.subscribe();

    // Drop receiver
    drop(_rx);

    // Cleanup should remove the channel
    registry.cleanup("test-cleanup").await;
}

#[tokio::test]
async fn test_client_message_types() {
    // Test Subscribe
    let subscribe = ClientMessage::Subscribe {
        channel: "test".to_string(),
        auth: None,
    };
    let json = serde_json::to_string(&subscribe).unwrap();
    assert!(json.contains("subscribe"));

    // Test Unsubscribe
    let unsubscribe = ClientMessage::Unsubscribe {
        channel: "test".to_string(),
    };
    let json = serde_json::to_string(&unsubscribe).unwrap();
    assert!(json.contains("unsubscribe"));

    // Test Ping
    let ping = ClientMessage::Ping;
    let json = serde_json::to_string(&ping).unwrap();
    assert!(json.contains("ping"));
}

#[tokio::test]
async fn test_server_message_types() {
    // Test Event
    let event = ServerMessage::Event {
        channel: "orders".to_string(),
        event: "OrderCreated".to_string(),
        data: json!({"id": 1}),
    };
    let json = event.to_json().unwrap();
    assert!(json.contains("event"));
    assert!(json.contains("OrderCreated"));

    // Test Subscribed
    let subscribed = ServerMessage::Subscribed {
        channel: "orders".to_string(),
    };
    let json = subscribed.to_json().unwrap();
    assert!(json.contains("subscribed"));

    // Test Unsubscribed
    let unsubscribed = ServerMessage::Unsubscribed {
        channel: "orders".to_string(),
    };
    let json = unsubscribed.to_json().unwrap();
    assert!(json.contains("unsubscribed"));

    // Test Pong
    let pong = ServerMessage::Pong;
    let json = pong.to_json().unwrap();
    assert!(json.contains("pong"));

    // Test Error
    let error = ServerMessage::Error {
        message: "Test error".to_string(),
    };
    let json = error.to_json().unwrap();
    assert!(json.contains("error"));
    assert!(json.contains("Test error"));
}

#[tokio::test]
async fn test_broadcast_trait() {
    let event = TestEvent {
        message: "Hello".to_string(),
        data: 42,
    };

    assert_eq!(event.broadcast_on(), vec!["test-channel"]);
    assert_eq!(event.broadcast_with()["message"], "Hello");
    assert_eq!(event.broadcast_with()["data"], 42);
    assert_eq!(event.broadcast_as(), None);
    assert!(!event.exclude_current());
}
