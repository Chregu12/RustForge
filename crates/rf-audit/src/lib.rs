//! Audit Logging System for RustForge
//!
//! This crate provides comprehensive audit trail functionality for compliance.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Audit errors
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type AuditResult<T> = Result<T, AuditError>;

/// Audit action types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    Created,
    Updated,
    Deleted,
    Viewed,
    Custom(String),
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub user_id: Option<i64>,
    pub model_type: String,
    pub model_id: String,
    pub action: AuditAction,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl AuditEntry {
    pub fn new(model_type: impl Into<String>, model_id: impl Into<String>, action: AuditAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: None,
            model_type: model_type.into(),
            model_id: model_id.into(),
            action,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn user_id(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn old_values(mut self, values: serde_json::Value) -> Self {
        self.old_values = Some(values);
        self
    }

    pub fn new_values(mut self, values: serde_json::Value) -> Self {
        self.new_values = Some(values);
        self
    }

    pub fn ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Audit storage trait
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// Store an audit entry
    async fn store(&self, entry: AuditEntry) -> AuditResult<()>;

    /// Query audit entries
    async fn query(&self, query: AuditQuery) -> AuditResult<Vec<AuditEntry>>;

    /// Delete old entries (for retention policies)
    async fn delete_before(&self, date: DateTime<Utc>) -> AuditResult<usize>;
}

/// Audit query builder
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub model_type: Option<String>,
    pub model_id: Option<String>,
    pub user_id: Option<i64>,
    pub action: Option<AuditAction>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl AuditQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model_type(mut self, model_type: impl Into<String>) -> Self {
        self.model_type = Some(model_type.into());
        self
    }

    pub fn model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn user_id(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_date = Some(start);
        self.end_date = Some(end);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// In-memory audit storage
pub struct MemoryAuditStorage {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
}

impl MemoryAuditStorage {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn count(&self) -> usize {
        self.entries.read().await.len()
    }
}

impl Default for MemoryAuditStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditStorage for MemoryAuditStorage {
    async fn store(&self, entry: AuditEntry) -> AuditResult<()> {
        let mut entries = self.entries.write().await;
        entries.push(entry);
        Ok(())
    }

    async fn query(&self, query: AuditQuery) -> AuditResult<Vec<AuditEntry>> {
        let entries = self.entries.read().await;

        let mut results: Vec<AuditEntry> = entries
            .iter()
            .filter(|entry| {
                if let Some(ref model_type) = query.model_type {
                    if &entry.model_type != model_type {
                        return false;
                    }
                }

                if let Some(ref model_id) = query.model_id {
                    if &entry.model_id != model_id {
                        return false;
                    }
                }

                if let Some(user_id) = query.user_id {
                    if entry.user_id != Some(user_id) {
                        return false;
                    }
                }

                if let Some(ref action) = query.action {
                    if &entry.action != action {
                        return false;
                    }
                }

                if let Some(start) = query.start_date {
                    if entry.created_at < start {
                        return false;
                    }
                }

                if let Some(end) = query.end_date {
                    if entry.created_at > end {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply offset and limit
        if let Some(offset) = query.offset {
            results = results.into_iter().skip(offset).collect();
        }

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn delete_before(&self, date: DateTime<Utc>) -> AuditResult<usize> {
        let mut entries = self.entries.write().await;
        let before_count = entries.len();
        entries.retain(|entry| entry.created_at >= date);
        let deleted = before_count - entries.len();
        Ok(deleted)
    }
}

/// Audit logger
pub struct AuditLogger {
    storage: Arc<dyn AuditStorage>,
}

impl AuditLogger {
    /// Create a new audit logger with memory storage
    pub fn new() -> Self {
        Self {
            storage: Arc::new(MemoryAuditStorage::new()),
        }
    }

    /// Create an audit logger with custom storage
    pub fn with_storage(storage: Arc<dyn AuditStorage>) -> Self {
        Self { storage }
    }

    /// Log an audit entry
    pub async fn log(&self, entry: AuditEntry) -> AuditResult<()> {
        self.storage.store(entry).await
    }

    /// Log a creation
    pub async fn log_created(
        &self,
        model_type: impl Into<String>,
        model_id: impl Into<String>,
        new_values: serde_json::Value,
        user_id: Option<i64>,
    ) -> AuditResult<()> {
        let entry = AuditEntry::new(model_type, model_id, AuditAction::Created)
            .new_values(new_values);

        let entry = if let Some(uid) = user_id {
            entry.user_id(uid)
        } else {
            entry
        };

        self.log(entry).await
    }

    /// Log an update
    pub async fn log_updated(
        &self,
        model_type: impl Into<String>,
        model_id: impl Into<String>,
        old_values: serde_json::Value,
        new_values: serde_json::Value,
        user_id: Option<i64>,
    ) -> AuditResult<()> {
        let entry = AuditEntry::new(model_type, model_id, AuditAction::Updated)
            .old_values(old_values)
            .new_values(new_values);

        let entry = if let Some(uid) = user_id {
            entry.user_id(uid)
        } else {
            entry
        };

        self.log(entry).await
    }

    /// Log a deletion
    pub async fn log_deleted(
        &self,
        model_type: impl Into<String>,
        model_id: impl Into<String>,
        old_values: serde_json::Value,
        user_id: Option<i64>,
    ) -> AuditResult<()> {
        let entry = AuditEntry::new(model_type, model_id, AuditAction::Deleted)
            .old_values(old_values);

        let entry = if let Some(uid) = user_id {
            entry.user_id(uid)
        } else {
            entry
        };

        self.log(entry).await
    }

    /// Query audit logs
    pub async fn query(&self, query: AuditQuery) -> AuditResult<Vec<AuditEntry>> {
        self.storage.query(query).await
    }

    /// Get logs for a specific model
    pub async fn for_model(
        &self,
        model_type: impl Into<String>,
        model_id: impl Into<String>,
    ) -> AuditResult<Vec<AuditEntry>> {
        self.query(
            AuditQuery::new()
                .model_type(model_type)
                .model_id(model_id)
        ).await
    }

    /// Get logs by user
    pub async fn by_user(&self, user_id: i64) -> AuditResult<Vec<AuditEntry>> {
        self.query(AuditQuery::new().user_id(user_id)).await
    }

    /// Clean old entries (retention policy)
    pub async fn clean_before(&self, date: DateTime<Utc>) -> AuditResult<usize> {
        self.storage.delete_before(date).await
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Auditable trait
#[async_trait]
pub trait Auditable {
    /// Get model type name
    fn model_type() -> &'static str;

    /// Get model ID
    fn model_id(&self) -> String;

    /// Serialize to JSON value
    fn to_audit_value(&self) -> AuditResult<serde_json::Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Serialize)]
    struct TestModel {
        id: i64,
        name: String,
        email: String,
    }

    impl Auditable for TestModel {
        fn model_type() -> &'static str {
            "TestModel"
        }

        fn model_id(&self) -> String {
            self.id.to_string()
        }

        fn to_audit_value(&self) -> AuditResult<serde_json::Value> {
            serde_json::to_value(self)
                .map_err(|e| AuditError::SerializationError(e.to_string()))
        }
    }

    #[tokio::test]
    async fn test_audit_entry_builder() {
        let entry = AuditEntry::new("User", "123", AuditAction::Created)
            .user_id(1)
            .ip_address("127.0.0.1")
            .user_agent("Mozilla/5.0")
            .metadata("action", "signup");

        assert_eq!(entry.model_type, "User");
        assert_eq!(entry.model_id, "123");
        assert_eq!(entry.user_id, Some(1));
        assert_eq!(entry.ip_address, Some("127.0.0.1".to_string()));
        assert_eq!(entry.metadata.get("action"), Some(&"signup".to_string()));
    }

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryAuditStorage::new();

        let entry = AuditEntry::new("User", "1", AuditAction::Created);
        storage.store(entry).await.unwrap();

        assert_eq!(storage.count().await, 1);
    }

    #[tokio::test]
    async fn test_audit_logger_created() {
        let logger = AuditLogger::new();

        let model = TestModel {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };

        logger
            .log_created(
                TestModel::model_type(),
                model.model_id(),
                model.to_audit_value().unwrap(),
                Some(100),
            )
            .await
            .unwrap();

        let logs = logger.for_model("TestModel", "1").await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, AuditAction::Created);
        assert_eq!(logs[0].user_id, Some(100));
    }

    #[tokio::test]
    async fn test_audit_logger_updated() {
        let logger = AuditLogger::new();

        let old = TestModel {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };

        let new = TestModel {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };

        logger
            .log_updated(
                TestModel::model_type(),
                old.model_id(),
                old.to_audit_value().unwrap(),
                new.to_audit_value().unwrap(),
                Some(100),
            )
            .await
            .unwrap();

        let logs = logger.for_model("TestModel", "1").await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, AuditAction::Updated);
        assert!(logs[0].old_values.is_some());
        assert!(logs[0].new_values.is_some());
    }

    #[tokio::test]
    async fn test_audit_logger_deleted() {
        let logger = AuditLogger::new();

        let model = TestModel {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };

        logger
            .log_deleted(
                TestModel::model_type(),
                model.model_id(),
                model.to_audit_value().unwrap(),
                Some(100),
            )
            .await
            .unwrap();

        let logs = logger.for_model("TestModel", "1").await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, AuditAction::Deleted);
    }

    #[tokio::test]
    async fn test_query_by_user() {
        let logger = AuditLogger::new();

        // User 1 creates record
        logger
            .log_created("User", "1", serde_json::json!({"name": "Alice"}), Some(1))
            .await
            .unwrap();

        // User 2 creates record
        logger
            .log_created("User", "2", serde_json::json!({"name": "Bob"}), Some(2))
            .await
            .unwrap();

        let logs = logger.by_user(1).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].user_id, Some(1));
    }

    #[tokio::test]
    async fn test_query_by_action() {
        let logger = AuditLogger::new();

        logger
            .log_created("User", "1", serde_json::json!({}), None)
            .await
            .unwrap();

        logger
            .log_updated("User", "1", serde_json::json!({}), serde_json::json!({}), None)
            .await
            .unwrap();

        let logs = logger
            .query(AuditQuery::new().action(AuditAction::Created))
            .await
            .unwrap();

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, AuditAction::Created);
    }

    #[tokio::test]
    async fn test_query_with_date_range() {
        let logger = AuditLogger::new();

        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let one_hour_later = now + chrono::Duration::hours(1);

        logger
            .log_created("User", "1", serde_json::json!({}), None)
            .await
            .unwrap();

        let logs = logger
            .query(AuditQuery::new().between(one_hour_ago, one_hour_later))
            .await
            .unwrap();

        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let logger = AuditLogger::new();

        for i in 1..=10 {
            logger
                .log_created("User", &i.to_string(), serde_json::json!({}), None)
                .await
                .unwrap();
        }

        let logs = logger
            .query(AuditQuery::new().limit(5))
            .await
            .unwrap();

        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_offset() {
        let logger = AuditLogger::new();

        for i in 1..=10 {
            logger
                .log_created("User", &i.to_string(), serde_json::json!({}), None)
                .await
                .unwrap();
        }

        let logs = logger
            .query(AuditQuery::new().offset(5).limit(5))
            .await
            .unwrap();

        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_clean_old_entries() {
        let logger = AuditLogger::new();

        logger
            .log_created("User", "1", serde_json::json!({}), None)
            .await
            .unwrap();

        let future = Utc::now() + chrono::Duration::hours(1);
        let deleted = logger.clean_before(future).await.unwrap();

        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn test_multiple_actions_same_model() {
        let logger = AuditLogger::new();

        logger
            .log_created("User", "1", serde_json::json!({"name": "Alice"}), Some(1))
            .await
            .unwrap();

        logger
            .log_updated(
                "User",
                "1",
                serde_json::json!({"name": "Alice"}),
                serde_json::json!({"name": "Alice Smith"}),
                Some(1),
            )
            .await
            .unwrap();

        logger
            .log_deleted("User", "1", serde_json::json!({"name": "Alice Smith"}), Some(2))
            .await
            .unwrap();

        let logs = logger.for_model("User", "1").await.unwrap();
        assert_eq!(logs.len(), 3);

        // Should be sorted by created_at descending
        assert_eq!(logs[0].action, AuditAction::Deleted);
        assert_eq!(logs[1].action, AuditAction::Updated);
        assert_eq!(logs[2].action, AuditAction::Created);
    }
}
