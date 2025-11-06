//! Live Updates Example - Real-Time Data Updates
//!
//! Beispiel für Live-Daten-Updates über WebSockets.

use crate::websocket::{
    manager::WebSocketManager,
    message::WebSocketMessage,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Ein Live-Update Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveUpdate {
    /// Event-Typ
    pub event_type: String,
    /// Entity-Typ (z.B. "user", "post", "order")
    pub entity_type: String,
    /// Entity-ID
    pub entity_id: String,
    /// Aktion (created, updated, deleted)
    pub action: UpdateAction,
    /// Daten
    pub data: serde_json::Value,
    /// Zeitstempel
    pub timestamp: i64,
}

/// Update-Aktionen
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateAction {
    Created,
    Updated,
    Deleted,
}

impl LiveUpdate {
    /// Erstellt ein neues Live-Update
    pub fn new(
        event_type: impl Into<String>,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        action: UpdateAction,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            action,
            data,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Konvertiert zu WebSocket-Nachricht
    pub fn to_websocket_message(&self) -> serde_json::Result<WebSocketMessage> {
        Ok(WebSocketMessage::event(&self.event_type, json!(self)))
    }
}

/// Service für Live-Updates
pub struct LiveUpdateService {
    manager: WebSocketManager,
}

impl LiveUpdateService {
    /// Erstellt einen neuen LiveUpdateService
    pub fn new(manager: WebSocketManager) -> Self {
        Self { manager }
    }

    /// Sendet ein Live-Update an alle verbundenen Clients
    ///
    /// # Beispiel
    ///
    /// ```no_run
    /// use foundry_api::websocket::examples::live_updates::{LiveUpdateService, LiveUpdate, UpdateAction};
    /// use foundry_api::websocket::WebSocketManager;
    /// use serde_json::json;
    ///
    /// # async fn example() {
    /// let manager = WebSocketManager::new();
    /// let live = LiveUpdateService::new(manager);
    ///
    /// let update = LiveUpdate::new(
    ///     "user.created",
    ///     "user",
    ///     "123",
    ///     UpdateAction::Created,
    ///     json!({"name": "Alice", "email": "alice@example.com"})
    /// );
    ///
    /// live.broadcast_update(update).await.unwrap();
    /// # }
    /// ```
    pub async fn broadcast_update(&self, update: LiveUpdate) -> anyhow::Result<usize> {
        let message = update.to_websocket_message()?;
        let count = self.manager.broadcast(message, None).await;
        Ok(count)
    }

    /// Sendet ein Update an einen spezifischen Channel
    pub async fn send_update_to_channel(
        &self,
        channel: &str,
        update: LiveUpdate,
    ) -> anyhow::Result<usize> {
        let message = update.to_websocket_message()?;
        let count = self.manager.send_to_channel(channel, message).await;
        Ok(count)
    }

    /// Hilfsmethode: Benachrichtigt über eine Entity-Erstellung
    pub async fn notify_created(
        &self,
        entity_type: &str,
        entity_id: &str,
        data: serde_json::Value,
    ) -> anyhow::Result<usize> {
        let event_type = format!("{}.created", entity_type);
        let update = LiveUpdate::new(
            event_type,
            entity_type,
            entity_id,
            UpdateAction::Created,
            data,
        );
        self.broadcast_update(update).await
    }

    /// Hilfsmethode: Benachrichtigt über eine Entity-Änderung
    pub async fn notify_updated(
        &self,
        entity_type: &str,
        entity_id: &str,
        data: serde_json::Value,
    ) -> anyhow::Result<usize> {
        let event_type = format!("{}.updated", entity_type);
        let update = LiveUpdate::new(
            event_type,
            entity_type,
            entity_id,
            UpdateAction::Updated,
            data,
        );
        self.broadcast_update(update).await
    }

    /// Hilfsmethode: Benachrichtigt über eine Entity-Löschung
    pub async fn notify_deleted(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> anyhow::Result<usize> {
        let event_type = format!("{}.deleted", entity_type);
        let update = LiveUpdate::new(
            event_type,
            entity_type,
            entity_id,
            UpdateAction::Deleted,
            json!({}),
        );
        self.broadcast_update(update).await
    }

    /// Sendet ein Heartbeat-Update an alle Clients
    pub async fn send_heartbeat(&self) -> usize {
        let message = WebSocketMessage::event(
            "heartbeat",
            json!({
                "timestamp": chrono::Utc::now().timestamp(),
                "server": "rustforge"
            }),
        );
        self.manager.broadcast(message, None).await
    }
}

/// Praktisches Beispiel: Echtzeit-Dashboard-Updates
pub struct DashboardMetrics {
    pub active_users: usize,
    pub requests_per_second: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

impl DashboardMetrics {
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "active_users": self.active_users,
            "requests_per_second": self.requests_per_second,
            "memory_usage_mb": self.memory_usage_mb,
            "cpu_usage_percent": self.cpu_usage_percent,
            "timestamp": chrono::Utc::now().timestamp()
        })
    }
}

/// Service für Dashboard-Metriken
pub struct DashboardService {
    live_updates: LiveUpdateService,
}

impl DashboardService {
    pub fn new(manager: WebSocketManager) -> Self {
        Self {
            live_updates: LiveUpdateService::new(manager),
        }
    }

    /// Sendet Dashboard-Metriken an Clients
    pub async fn send_metrics(&self, metrics: DashboardMetrics) -> anyhow::Result<usize> {
        let update = LiveUpdate::new(
            "dashboard.metrics",
            "metrics",
            "system",
            UpdateAction::Updated,
            metrics.to_json(),
        );
        self.live_updates.send_update_to_channel("dashboard", update).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_update_creation() {
        let update = LiveUpdate::new(
            "test.event",
            "test",
            "123",
            UpdateAction::Created,
            json!({"key": "value"}),
        );
        assert_eq!(update.event_type, "test.event");
        assert_eq!(update.entity_type, "test");
    }

    #[tokio::test]
    async fn test_live_update_service() {
        let manager = WebSocketManager::new();
        let service = LiveUpdateService::new(manager);

        // Diese sendet an 0 Clients (keine verbunden)
        let result = service.notify_created("user", "1", json!({"name": "Test"})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_dashboard_metrics() {
        let metrics = DashboardMetrics {
            active_users: 42,
            requests_per_second: 123.45,
            memory_usage_mb: 256.0,
            cpu_usage_percent: 45.5,
        };
        let json = metrics.to_json();
        assert_eq!(json["active_users"], 42);
    }
}
