//! WebSocket Integration Tests

use foundry_api::websocket::{
    Channel, ChannelManager, ConnectionId, WebSocketManager, WebSocketMessage,
    BroadcastOptions,
};

#[tokio::test]
async fn test_websocket_manager_basic() {
    let manager = WebSocketManager::new();
    assert_eq!(manager.connection_count().await, 0);
}

#[tokio::test]
async fn test_channel_creation_and_subscription() {
    let manager = ChannelManager::new();

    // Erstelle einen Channel
    let channel = Channel::new("test-channel").with_description("Test Channel");
    manager.create_channel(channel).await;

    // Prüfe ob Channel existiert
    assert!(manager.channel_exists("test-channel").await);

    // Subscribe eine Connection
    let conn_id = ConnectionId::new();
    manager.subscribe("test-channel", conn_id).await;

    // Prüfe Subscriber-Count
    assert_eq!(manager.subscriber_count("test-channel").await, 1);

    // Unsubscribe
    manager.unsubscribe("test-channel", conn_id).await;
    assert_eq!(manager.subscriber_count("test-channel").await, 0);
}

#[tokio::test]
async fn test_multiple_channel_subscriptions() {
    let manager = ChannelManager::new();

    // Erstelle mehrere Channels
    manager.create_channel(Channel::new("channel-1")).await;
    manager.create_channel(Channel::new("channel-2")).await;
    manager.create_channel(Channel::new("channel-3")).await;

    // Eine Connection abonniert mehrere Channels
    let conn_id = ConnectionId::new();
    manager.subscribe("channel-1", conn_id).await;
    manager.subscribe("channel-2", conn_id).await;
    manager.subscribe("channel-3", conn_id).await;

    // Prüfe subscribed channels
    let channels = manager.get_subscribed_channels(conn_id).await;
    assert_eq!(channels.len(), 3);

    // Unsubscribe all
    manager.unsubscribe_all(conn_id).await;
    let channels = manager.get_subscribed_channels(conn_id).await;
    assert_eq!(channels.len(), 0);
}

#[tokio::test]
async fn test_websocket_message_creation() {
    // Text message
    let text_msg = WebSocketMessage::text("Hello World");
    assert!(text_msg.payload.is_some());

    // JSON message
    let json_data = serde_json::json!({"key": "value"});
    let json_msg = WebSocketMessage::json(&json_data).unwrap();
    assert!(json_msg.payload.is_some());

    // Event message
    let event_data = serde_json::json!({"userId": 42});
    let event_msg = WebSocketMessage::event("user.joined", event_data);
    assert!(event_msg.payload.is_some());

    // System message
    let system_msg = WebSocketMessage::system("System notification");
    assert!(system_msg.payload.is_some());

    // Ping/Pong
    let ping = WebSocketMessage::ping();
    let pong = WebSocketMessage::pong();
    assert!(ping.payload.is_none());
    assert!(pong.payload.is_none());
}

#[tokio::test]
async fn test_websocket_message_serialization() {
    let msg = WebSocketMessage::text("Test Message");
    let json_str = msg.to_json_string().unwrap();
    assert!(!json_str.is_empty());

    let deserialized = WebSocketMessage::from_json_string(&json_str).unwrap();
    assert_eq!(deserialized.payload, msg.payload);
}

#[tokio::test]
async fn test_broadcast_options() {
    let id1 = ConnectionId::new();
    let id2 = ConnectionId::new();
    let id3 = ConnectionId::new();

    // Test exclude
    let opts = BroadcastOptions::new().exclude(vec![id1, id2]);
    assert!(!opts.should_include(&id1));
    assert!(!opts.should_include(&id2));
    assert!(opts.should_include(&id3));

    // Test only_to
    let opts = BroadcastOptions::new().only_to(vec![id1]);
    assert!(opts.should_include(&id1));
    assert!(!opts.should_include(&id2));
    assert!(!opts.should_include(&id3));

    // Test channel
    let opts = BroadcastOptions::new().to_channel("test-channel");
    assert_eq!(opts.channel, Some("test-channel".to_string()));
}

#[tokio::test]
async fn test_channel_metadata() {
    let manager = ChannelManager::new();

    let channel = Channel::new("metadata-test")
        .with_description("Test description")
        .make_private();

    manager.create_channel(channel).await;

    let metadata = manager.get_channel_metadata("metadata-test").await;
    assert!(metadata.is_some());

    let meta = metadata.unwrap();
    assert_eq!(meta.name, "metadata-test");
    assert_eq!(meta.description, Some("Test description".to_string()));
    assert!(meta.is_private);
}

#[tokio::test]
async fn test_channel_deletion() {
    let manager = ChannelManager::new();

    manager.create_channel(Channel::new("delete-test")).await;
    assert!(manager.channel_exists("delete-test").await);

    manager.delete_channel("delete-test").await;
    assert!(!manager.channel_exists("delete-test").await);
}

#[tokio::test]
async fn test_list_channels() {
    let manager = ChannelManager::new();

    manager.create_channel(Channel::new("channel-a")).await;
    manager.create_channel(Channel::new("channel-b")).await;
    manager.create_channel(Channel::new("channel-c")).await;

    let channels = manager.list_channels().await;
    assert_eq!(channels.len(), 3);
    assert!(channels.contains(&"channel-a".to_string()));
    assert!(channels.contains(&"channel-b".to_string()));
    assert!(channels.contains(&"channel-c".to_string()));
}

#[tokio::test]
async fn test_connection_id_uniqueness() {
    let id1 = ConnectionId::new();
    let id2 = ConnectionId::new();
    let id3 = ConnectionId::new();

    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
}

#[tokio::test]
async fn test_connection_id_display() {
    let id = ConnectionId::new();
    let display = format!("{}", id);
    assert!(!display.is_empty());
    assert!(display.contains('-')); // UUID format
}
