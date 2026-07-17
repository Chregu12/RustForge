//! WebSocket support for real-time dashboard updates

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::routes::AppState;

/// WebSocket update message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsUpdate {
    /// Queue metrics update
    #[serde(rename = "metrics")]
    Metrics { data: MetricsUpdate },
    /// Worker status update
    #[serde(rename = "workers")]
    Workers { data: WorkersUpdate },
    /// Recent jobs update
    #[serde(rename = "recent_jobs")]
    RecentJobs { data: RecentJobsUpdate },
    /// System stats update
    #[serde(rename = "stats")]
    Stats { data: StatsUpdate },
    /// Heartbeat
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsUpdate {
    pub queues: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkersUpdate {
    pub workers: Vec<serde_json::Value>,
    pub total: usize,
    pub active: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentJobsUpdate {
    pub jobs: Vec<serde_json::Value>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsUpdate {
    pub total_jobs: u64,
    pub jobs_pending: u64,
    pub jobs_processing: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub jobs_per_minute: f64,
}

/// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Create update interval (1 second)
    let mut update_interval = interval(Duration::from_secs(1));

    // Heartbeat interval (30 seconds)
    let mut heartbeat_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = update_interval.tick() => {
                // Send metrics update
                if let Ok(update) = collect_metrics_update(&state).await {
                    let msg = serde_json::to_string(&update).unwrap_or_default();
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
            }
            _ = heartbeat_interval.tick() => {
                // Send heartbeat
                let heartbeat = WsUpdate::Heartbeat {
                    timestamp: chrono::Utc::now().timestamp(),
                };
                let msg = serde_json::to_string(&heartbeat).unwrap_or_default();
                if sender.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(ping))) => {
                        if sender.send(Message::Pong(ping)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
        }
    }
}

/// Collect current metrics for update
async fn collect_metrics_update(state: &Arc<AppState>) -> anyhow::Result<WsUpdate> {
    let horizon_state = state.horizon.state().await;

    // Calculate stats
    let mut total_jobs = 0u64;
    let mut jobs_pending = 0u64;
    let mut jobs_completed = 0u64;
    let mut jobs_failed = 0u64;
    let mut jobs_per_minute = 0.0;

    for metrics in horizon_state.metrics.values() {
        total_jobs += metrics.jobs_processed + metrics.jobs_failed;
        jobs_pending += metrics.jobs_pending;
        jobs_completed += metrics.jobs_processed;
        jobs_failed += metrics.jobs_failed;
        jobs_per_minute += metrics.throughput_per_minute;
    }

    Ok(WsUpdate::Stats {
        data: StatsUpdate {
            total_jobs,
            jobs_pending,
            jobs_processing: 0, // TODO: Get from worker registry
            jobs_completed,
            jobs_failed,
            jobs_per_minute,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_update_serialization() {
        let update = WsUpdate::Heartbeat {
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("\"type\":\"heartbeat\""));
        assert!(json.contains("\"timestamp\":1234567890"));
    }

    #[test]
    fn test_stats_update_serialization() {
        let update = WsUpdate::Stats {
            data: StatsUpdate {
                total_jobs: 100,
                jobs_pending: 10,
                jobs_processing: 5,
                jobs_completed: 80,
                jobs_failed: 5,
                jobs_per_minute: 10.5,
            },
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("\"type\":\"stats\""));
        assert!(json.contains("\"total_jobs\":100"));
    }
}
