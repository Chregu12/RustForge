//! Request watcher for HTTP request monitoring

use crate::{Entry, EntryType, Storage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTP request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub ip_address: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

impl RequestInfo {
    /// Create a new request info
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        ip_address: impl Into<String>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            status: 200,
            duration_ms: 0,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            ip_address: ip_address.into(),
            user_id: None,
            session_id: None,
            started_at: Utc::now(),
            completed_at: Utc::now(),
        }
    }

    /// Set HTTP status
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self.completed_at = self.started_at + chrono::Duration::milliseconds(duration_ms as i64);
        self
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Add a query parameter
    pub fn with_query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Request watcher for monitoring HTTP requests
#[derive(Clone)]
pub struct RequestWatcher {
    storage: Storage,
    active_requests: Arc<RwLock<HashMap<String, RequestInfo>>>,
}

impl RequestWatcher {
    /// Create a new request watcher
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            active_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a request
    pub async fn start_request(&self, request_id: String, info: RequestInfo) {
        let mut requests = self.active_requests.write().await;
        requests.insert(request_id, info);
    }

    /// Complete a request and store it
    pub async fn complete_request(&self, request_id: String, status: u16, duration_ms: u64) {
        let mut requests = self.active_requests.write().await;

        if let Some(mut info) = requests.remove(&request_id) {
            info.status = status;
            info.duration_ms = duration_ms;
            info.completed_at =
                info.started_at + chrono::Duration::milliseconds(duration_ms as i64);

            let entry = Entry::new(
                EntryType::Request,
                json!({
                    "method": info.method,
                    "path": info.path,
                    "status": info.status,
                    "duration_ms": info.duration_ms,
                    "headers": info.headers,
                    "query_params": info.query_params,
                    "ip_address": info.ip_address,
                    "user_id": info.user_id,
                    "session_id": info.session_id,
                    "started_at": info.started_at,
                    "completed_at": info.completed_at,
                }),
            )
            .with_tag(format!("status:{}", info.status))
            .with_tag(format!("method:{}", info.method));

            self.storage.store(entry).await;
        }
    }

    /// Record a complete request (for simple use cases)
    pub async fn record(&self, info: RequestInfo) {
        let entry = Entry::new(
            EntryType::Request,
            json!({
                "method": info.method,
                "path": info.path,
                "status": info.status,
                "duration_ms": info.duration_ms,
                "headers": info.headers,
                "query_params": info.query_params,
                "ip_address": info.ip_address,
                "user_id": info.user_id,
                "session_id": info.session_id,
                "started_at": info.started_at,
                "completed_at": info.completed_at,
            }),
        )
        .with_tag(format!("status:{}", info.status))
        .with_tag(format!("method:{}", info.method));

        self.storage.store(entry).await;
    }

    /// Get all recorded requests
    pub async fn all(&self) -> Vec<Entry> {
        self.storage.by_type(EntryType::Request).await
    }

    /// Get requests with specific status code
    pub async fn by_status(&self, status: u16) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| {
                entry
                    .content
                    .get("status")
                    .and_then(|s| s.as_u64())
                    .map(|s| s as u16 == status)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get slow requests (above threshold in milliseconds)
    pub async fn slow_requests(&self, threshold_ms: u64) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| {
                entry
                    .content
                    .get("duration_ms")
                    .and_then(|d| d.as_u64())
                    .map(|d| d > threshold_ms)
                    .unwrap_or(false)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_info_creation() {
        let info = RequestInfo::new("GET", "/api/users", "127.0.0.1")
            .with_status(200)
            .with_duration(150)
            .with_header("User-Agent", "TestClient/1.0");

        assert_eq!(info.method, "GET");
        assert_eq!(info.path, "/api/users");
        assert_eq!(info.status, 200);
        assert_eq!(info.duration_ms, 150);
        assert_eq!(info.headers.get("User-Agent").unwrap(), "TestClient/1.0");
    }

    #[tokio::test]
    async fn test_request_watcher_record() {
        let storage = Storage::new();
        let watcher = RequestWatcher::new(storage.clone());

        let info = RequestInfo::new("POST", "/api/login", "192.168.1.1")
            .with_status(201)
            .with_duration(250);

        watcher.record(info).await;

        let requests = watcher.all().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].content["method"], "POST");
        assert_eq!(requests[0].content["path"], "/api/login");
    }

    #[tokio::test]
    async fn test_request_watcher_by_status() {
        let storage = Storage::new();
        let watcher = RequestWatcher::new(storage);

        watcher
            .record(RequestInfo::new("GET", "/", "127.0.0.1").with_status(200))
            .await;
        watcher
            .record(RequestInfo::new("GET", "/404", "127.0.0.1").with_status(404))
            .await;
        watcher
            .record(RequestInfo::new("GET", "/error", "127.0.0.1").with_status(500))
            .await;

        let not_found = watcher.by_status(404).await;
        assert_eq!(not_found.len(), 1);
    }

    #[tokio::test]
    async fn test_request_watcher_slow_requests() {
        let storage = Storage::new();
        let watcher = RequestWatcher::new(storage);

        watcher
            .record(RequestInfo::new("GET", "/fast", "127.0.0.1").with_duration(50))
            .await;
        watcher
            .record(RequestInfo::new("GET", "/slow", "127.0.0.1").with_duration(1500))
            .await;
        watcher
            .record(RequestInfo::new("GET", "/very-slow", "127.0.0.1").with_duration(3000))
            .await;

        let slow = watcher.slow_requests(1000).await;
        assert_eq!(slow.len(), 2);
    }

    #[tokio::test]
    async fn test_request_with_user_and_session() {
        let storage = Storage::new();
        let watcher = RequestWatcher::new(storage);

        let info = RequestInfo::new("GET", "/dashboard", "127.0.0.1")
            .with_user("user-123")
            .with_session("session-456");

        watcher.record(info).await;

        let requests = watcher.all().await;
        assert_eq!(requests[0].content["user_id"], "user-123");
        assert_eq!(requests[0].content["session_id"], "session-456");
    }
}
