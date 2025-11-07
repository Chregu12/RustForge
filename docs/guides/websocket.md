# WebSocket Real-Time Features

RustForge bietet vollständige WebSocket-Unterstützung für Echtzeit-Kommunikation in deinen Anwendungen.

## Features

- **Connection Management**: Automatisches Verwalten von WebSocket-Verbindungen mit Cleanup
- **Broadcasting**: Sende Nachrichten an alle verbundenen Clients
- **Channels**: Organisiere Clients in Gruppen/Räumen
- **Message Types**: JSON, Text, System, Events
- **Heartbeat**: Automatisches Ping/Pong für Connection-Health
- **Integration**: Nahtlose Integration mit Axum Router

## Quick Start

### 1. WebSocket-Routen hinzufügen

```rust
use foundry_api::websocket;
use foundry_api::HttpServer;
use axum::Router;

let server = HttpServer::new(invoker)
    .merge_router(websocket::websocket_routes());

server.serve("0.0.0.0:3000".parse()?).await?;
```

### 2. WebSocket Endpoints

Nach dem Start sind folgende Endpoints verfügbar:

- `ws://localhost:3000/ws` - Haupt-WebSocket-Endpoint
- `ws://localhost:3000/ws/:channel` - Channel-spezifischer Endpoint

### 3. Client-Verbindung (JavaScript)

```javascript
// Hauptverbindung
const ws = new WebSocket('ws://localhost:3000/ws');

// Channel-spezifisch
const chatWs = new WebSocket('ws://localhost:3000/ws/chat:room1');

ws.onopen = () => {
    console.log('Connected!');
};

ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    console.log('Received:', message);
};
```

## Nachrichten senden

### Server-seitig

```rust
use foundry_api::websocket::{WebSocketManager, WebSocketMessage};

// Manager abrufen
let manager = websocket::handler::get_websocket_manager().await.unwrap();

// Broadcast an alle
let msg = WebSocketMessage::text("Hello everyone!");
manager.broadcast(msg, None).await;

// An spezifischen Channel
let msg = WebSocketMessage::json(&data)?;
manager.send_to_channel("chat:room1", msg).await;

// An spezifische Connection
let msg = WebSocketMessage::event("user.joined", json!({"userId": 42}));
manager.send_to_connection(connection_id, msg).await?;
```

### Client-seitig (System Commands)

Der Client kann System-Commands senden:

```javascript
// Channel abonnieren
ws.send(JSON.stringify({
    type: "system",
    payload: {
        command: "subscribe",
        channel: "chat:room1"
    }
}));

// Channel deabonnieren
ws.send(JSON.stringify({
    type: "system",
    payload: {
        command: "unsubscribe",
        channel: "chat:room1"
    }
}));

// Channels auflisten
ws.send(JSON.stringify({
    type: "system",
    payload: {
        command: "list_channels"
    }
}));
```

## Message Types

```rust
use foundry_api::websocket::WebSocketMessage;

// Text-Nachricht
let msg = WebSocketMessage::text("Hello");

// JSON-Nachricht
let data = json!({"key": "value"});
let msg = WebSocketMessage::json(&data)?;

// Event-Nachricht
let msg = WebSocketMessage::event("user.joined", json!({"userId": 42}));

// System-Nachricht
let msg = WebSocketMessage::system("Server maintenance in 5 minutes");

// Ping/Pong
let ping = WebSocketMessage::ping();
let pong = WebSocketMessage::pong();

// Mit Metadaten
let msg = WebSocketMessage::text("Hello")
    .with_metadata(json!({"priority": "high"}));
```

## Channels

Channels erlauben es, Clients in Gruppen zu organisieren:

```rust
use foundry_api::websocket::{Channel, ChannelManager};

let manager = WebSocketManager::new();
let channel_mgr = manager.channel_manager();

// Channel erstellen
let channel = Channel::new("chat:general")
    .with_description("General Chat Room");
channel_mgr.create_channel(channel).await;

// Client abonniert Channel
channel_mgr.subscribe("chat:general", connection_id).await;

// Nachricht an Channel senden
let msg = WebSocketMessage::text("Welcome to general chat!");
manager.send_to_channel("chat:general", msg).await;

// Subscriber zählen
let count = channel_mgr.subscriber_count("chat:general").await;

// Channel löschen
channel_mgr.delete_channel("chat:general").await;
```

## Broadcasting mit Optionen

```rust
use foundry_api::websocket::BroadcastOptions;

// Nur an spezifische Connections
let opts = BroadcastOptions::new()
    .only_to(vec![conn_id1, conn_id2]);
manager.broadcast(msg, Some(opts)).await;

// Spezifische Connections ausschließen
let opts = BroadcastOptions::new()
    .exclude(vec![conn_id1]);
manager.broadcast(msg, Some(opts)).await;

// An Channel (alternativ zu send_to_channel)
let opts = BroadcastOptions::new()
    .to_channel("chat:vip");
manager.broadcast(msg, Some(opts)).await;
```

## Beispiele

### 1. Real-Time Chat

```rust
use foundry_api::websocket::examples::chat::{ChatService, ChatMessage};

let chat = ChatService::new(manager);

// Raum erstellen
chat.create_room("general", "General Discussion").await;

// Nachricht senden
let msg = ChatMessage::new("Alice", "Hello everyone!");
chat.send_message("general", msg).await?;

// User-Benachrichtigungen
chat.notify_user_joined("general", "Bob").await;
chat.notify_user_left("general", "Alice").await;

// Raum-Statistiken
let count = chat.room_user_count("general").await;
let rooms = chat.list_rooms().await;
```

### 2. Live Updates / Dashboard

```rust
use foundry_api::websocket::examples::live_updates::{
    LiveUpdateService, LiveUpdate, UpdateAction, DashboardService, DashboardMetrics
};

let live = LiveUpdateService::new(manager);

// Entity-Updates
live.notify_created("user", "123", json!({
    "name": "Alice",
    "email": "alice@example.com"
})).await?;

live.notify_updated("post", "456", json!({
    "title": "Updated Title"
})).await?;

live.notify_deleted("comment", "789").await?;

// Dashboard-Metriken
let dashboard = DashboardService::new(manager);
let metrics = DashboardMetrics {
    active_users: 42,
    requests_per_second: 123.45,
    memory_usage_mb: 256.0,
    cpu_usage_percent: 45.5,
};
dashboard.send_metrics(metrics).await?;
```

## CLI Commands

```bash
# WebSocket-Info anzeigen
rustforge websocket:info

# WebSocket-Statistiken (wenn Server läuft)
rustforge websocket:stats
```

## Connection Lifecycle

1. **Connect**: Client verbindet sich über WebSocket
2. **Register**: Connection wird im Manager registriert
3. **Welcome**: Server sendet Welcome-Nachricht
4. **Communication**: Bidirektionale Nachrichtenübertragung
5. **Heartbeat**: Automatische Ping/Pong-Messages
6. **Disconnect**: Client trennt Verbindung
7. **Cleanup**: Automatisches Deregistrieren und Channel-Cleanup

## Best Practices

### 1. Connection Management

```rust
// Verbindungen zählen
let count = manager.connection_count().await;

// Alle Connection-IDs abrufen
let ids = manager.get_connection_ids().await;

// Connection-Metadaten
let metadata = manager.get_connection_metadata(conn_id).await;
```

### 2. Error Handling

```rust
use anyhow::Result;

async fn send_update(manager: &WebSocketManager) -> Result<()> {
    let msg = WebSocketMessage::json(&data)?;

    if let Err(e) = manager.send_to_connection(conn_id, msg).await {
        tracing::error!("Failed to send message: {}", e);
        // Connection möglicherweise nicht mehr aktiv
    }

    Ok(())
}
```

### 3. Channel-Namenskonventionen

```rust
// Feature:Raum-Muster verwenden
"chat:general"
"chat:support"
"notifications:user-123"
"dashboard:admin"
"live:feed"
```

### 4. Message-Serialisierung

```rust
// Immer mit Result arbeiten
let msg = WebSocketMessage::json(&data)?;

// JSON-String für Debugging
let json_str = msg.to_json_string()?;
tracing::debug!("Sending: {}", json_str);
```

## Integration mit Events

```rust
use foundry_plugins::{DomainEvent, EventPort};

// WebSocket-Updates von Domain-Events
async fn on_domain_event(event: DomainEvent, manager: &WebSocketManager) {
    let ws_msg = WebSocketMessage::event(&event.name, event.payload);
    manager.broadcast(ws_msg, None).await;
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast() {
        let manager = WebSocketManager::new();

        // Keine Connections = 0 Empfänger
        let msg = WebSocketMessage::text("Test");
        let count = manager.broadcast(msg, None).await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_channel_subscription() {
        let manager = ChannelManager::new();
        let channel = Channel::new("test");
        manager.create_channel(channel).await;

        let conn_id = ConnectionId::new();
        manager.subscribe("test", conn_id).await;

        assert_eq!(manager.subscriber_count("test").await, 1);
    }
}
```

## Produktions-Deployment

### 1. Reverse Proxy (nginx)

```nginx
location /ws {
    proxy_pass http://localhost:3000;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_read_timeout 86400;
}
```

### 2. SSL/TLS

```rust
// Verwende wss:// statt ws://
// TLS wird vom Reverse Proxy gehandelt
```

### 3. Monitoring

```rust
// Periodisch Heartbeats senden
tokio::spawn(async move {
    loop {
        manager.ping_all().await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
});

// Connection-Statistiken loggen
tokio::spawn(async move {
    loop {
        let count = manager.connection_count().await;
        tracing::info!("Active WebSocket connections: {}", count);
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
});
```

## Troubleshooting

### Problem: Verbindung wird sofort geschlossen

- Prüfe, ob der WebSocket-Endpoint korrekt ist
- Stelle sicher, dass der HTTP-Server läuft
- Prüfe die Browser-Console auf Fehler

### Problem: Nachrichten kommen nicht an

- Prüfe, ob die Connection registriert ist
- Stelle sicher, dass der Channel existiert
- Prüfe die Logs auf Fehler

### Problem: Connection-Leaks

- Der Manager bereinigt Connections automatisch
- Bei Disconnect wird automatisch deregistriert
- Channel-Subscriptions werden automatisch aufgeräumt

## Weitere Ressourcen

- [Axum WebSocket Guide](https://docs.rs/axum/latest/axum/extract/ws/index.html)
- [WebSocket Protocol Spec](https://datatracker.ietf.org/doc/html/rfc6455)
- [JavaScript WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)

---

**Version**: 0.1.0
**Letztes Update**: 2025-11-01
