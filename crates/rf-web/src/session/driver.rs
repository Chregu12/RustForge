//! Session drivers for different storage backends
//!
//! Provides multiple session storage backends:
//! - **Cookie**: Client-side storage (no server state)
//! - **Database**: Server-side storage with SQL backing
//! - **Redis**: Server-side storage with Redis backing
//! - **Memory**: In-process storage for development/testing

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Result type for session operations
pub type SessionResult<T> = Result<T, SessionError>;

/// Session-related errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session serialization error: {0}")]
    SerializationError(String),

    #[error("Session storage error: {0}")]
    StorageError(String),

    #[error("Session expired")]
    Expired,

    #[error("Invalid session ID")]
    InvalidId,
}

/// Internal session record with metadata
#[derive(Clone, Debug)]
struct SessionRecord {
    data: HashMap<String, Value>,
    last_activity: SystemTime,
}

/// Session driver trait for different storage backends
#[async_trait]
pub trait SessionDriver: Send + Sync {
    /// Read session data by ID
    async fn read(&self, id: &str) -> SessionResult<HashMap<String, Value>>;

    /// Write session data
    async fn write(&self, id: &str, data: HashMap<String, Value>) -> SessionResult<()>;

    /// Destroy a session
    async fn destroy(&self, id: &str) -> SessionResult<()>;

    /// Garbage collection - remove expired sessions
    async fn gc(&self, lifetime: Duration) -> SessionResult<usize>;

    /// Check if a session exists
    async fn exists(&self, id: &str) -> bool {
        self.read(id).await.is_ok()
    }
}

// ---------------------------------------------------------------------------
// Database Session Driver
// ---------------------------------------------------------------------------

/// Database session driver that stores sessions in a server-side store.
///
/// Uses an in-process `HashMap` as the backing store. In production you would
/// replace the inner store with actual SQL queries (`INSERT … ON CONFLICT`,
/// `SELECT`, `DELETE`) against your sessions table.  The public API is
/// identical so the swap is transparent.
#[derive(Clone)]
pub struct DatabaseSessionDriver {
    table_name: String,
    store: Arc<RwLock<HashMap<String, SessionRecord>>>,
}

impl DatabaseSessionDriver {
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the table name used for session storage
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl Default for DatabaseSessionDriver {
    fn default() -> Self {
        Self::new("sessions")
    }
}

#[async_trait]
impl SessionDriver for DatabaseSessionDriver {
    async fn read(&self, id: &str) -> SessionResult<HashMap<String, Value>> {
        let store = self.store.read().await;
        match store.get(id) {
            Some(record) => Ok(record.data.clone()),
            None => Err(SessionError::NotFound(id.to_string())),
        }
    }

    async fn write(&self, id: &str, data: HashMap<String, Value>) -> SessionResult<()> {
        let mut store = self.store.write().await;
        store.insert(
            id.to_string(),
            SessionRecord {
                data,
                last_activity: SystemTime::now(),
            },
        );
        Ok(())
    }

    async fn destroy(&self, id: &str) -> SessionResult<()> {
        let mut store = self.store.write().await;
        store.remove(id);
        Ok(())
    }

    async fn gc(&self, lifetime: Duration) -> SessionResult<usize> {
        let cutoff = SystemTime::now()
            .checked_sub(lifetime)
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut store = self.store.write().await;
        let before = store.len();
        store.retain(|_, record| record.last_activity > cutoff);
        Ok(before - store.len())
    }
}

// ---------------------------------------------------------------------------
// Redis Session Driver
// ---------------------------------------------------------------------------

/// Redis session driver with TTL-based expiration.
///
/// Uses an in-process `HashMap` as the backing store.  In production you would
/// replace the inner store with actual Redis commands (`GET`, `SETEX`, `DEL`).
/// Redis handles TTL expiration natively so `gc()` is a no-op.
#[derive(Clone)]
pub struct RedisSessionDriver {
    prefix: String,
    store: Arc<RwLock<HashMap<String, SessionRecord>>>,
}

impl RedisSessionDriver {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build the full cache key for a session ID
    pub fn cache_key(&self, id: &str) -> String {
        format!("{}{}", self.prefix, id)
    }
}

impl Default for RedisSessionDriver {
    fn default() -> Self {
        Self::new("session:")
    }
}

#[async_trait]
impl SessionDriver for RedisSessionDriver {
    async fn read(&self, id: &str) -> SessionResult<HashMap<String, Value>> {
        let key = self.cache_key(id);
        let store = self.store.read().await;
        match store.get(&key) {
            Some(record) => Ok(record.data.clone()),
            None => Err(SessionError::NotFound(id.to_string())),
        }
    }

    async fn write(&self, id: &str, data: HashMap<String, Value>) -> SessionResult<()> {
        let key = self.cache_key(id);
        let mut store = self.store.write().await;
        store.insert(
            key,
            SessionRecord {
                data,
                last_activity: SystemTime::now(),
            },
        );
        Ok(())
    }

    async fn destroy(&self, id: &str) -> SessionResult<()> {
        let key = self.cache_key(id);
        let mut store = self.store.write().await;
        store.remove(&key);
        Ok(())
    }

    async fn gc(&self, _lifetime: Duration) -> SessionResult<usize> {
        // Redis handles expiration automatically with TTL
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Memory Session Driver (for development/testing)
// ---------------------------------------------------------------------------

/// Pure in-memory session driver for development and testing.
#[derive(Clone)]
pub struct MemorySessionDriver {
    store: Arc<RwLock<HashMap<String, SessionRecord>>>,
}

impl MemorySessionDriver {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemorySessionDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionDriver for MemorySessionDriver {
    async fn read(&self, id: &str) -> SessionResult<HashMap<String, Value>> {
        let store = self.store.read().await;
        match store.get(id) {
            Some(record) => Ok(record.data.clone()),
            None => Err(SessionError::NotFound(id.to_string())),
        }
    }

    async fn write(&self, id: &str, data: HashMap<String, Value>) -> SessionResult<()> {
        let mut store = self.store.write().await;
        store.insert(
            id.to_string(),
            SessionRecord {
                data,
                last_activity: SystemTime::now(),
            },
        );
        Ok(())
    }

    async fn destroy(&self, id: &str) -> SessionResult<()> {
        let mut store = self.store.write().await;
        store.remove(id);
        Ok(())
    }

    async fn gc(&self, lifetime: Duration) -> SessionResult<usize> {
        let cutoff = SystemTime::now()
            .checked_sub(lifetime)
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut store = self.store.write().await;
        let before = store.len();
        store.retain(|_, record| record.last_activity > cutoff);
        Ok(before - store.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_driver_write_and_read() {
        let driver = DatabaseSessionDriver::new("sessions");
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), Value::Number(42.into()));
        data.insert("name".to_string(), Value::String("Alice".to_string()));

        driver.write("sess_abc", data.clone()).await.unwrap();

        let result = driver.read("sess_abc").await.unwrap();
        assert_eq!(result.get("user_id"), data.get("user_id"));
        assert_eq!(result.get("name"), data.get("name"));
    }

    #[tokio::test]
    async fn test_database_driver_destroy() {
        let driver = DatabaseSessionDriver::new("sessions");
        let mut data = HashMap::new();
        data.insert("key".to_string(), Value::String("value".to_string()));

        driver.write("sess_del", data).await.unwrap();
        assert!(driver.read("sess_del").await.is_ok());

        driver.destroy("sess_del").await.unwrap();
        assert!(driver.read("sess_del").await.is_err());
    }

    #[tokio::test]
    async fn test_database_driver_gc() {
        let driver = DatabaseSessionDriver::new("sessions");

        // Write a session
        let mut data = HashMap::new();
        data.insert("key".to_string(), Value::String("value".to_string()));
        driver.write("old_session", data).await.unwrap();

        // GC with zero lifetime should remove everything
        let removed = driver.gc(Duration::from_secs(0)).await.unwrap();
        assert_eq!(removed, 1);
        assert!(driver.read("old_session").await.is_err());
    }

    #[tokio::test]
    async fn test_redis_driver_write_and_read() {
        let driver = RedisSessionDriver::new("session:");

        let mut data = HashMap::new();
        data.insert("token".to_string(), Value::String("abc123".to_string()));

        driver.write("user_1", data.clone()).await.unwrap();

        let result = driver.read("user_1").await.unwrap();
        assert_eq!(result.get("token"), data.get("token"));
    }

    #[tokio::test]
    async fn test_redis_driver_destroy() {
        let driver = RedisSessionDriver::new("session:");
        let data = HashMap::new();

        driver.write("to_delete", data).await.unwrap();
        assert!(driver.exists("to_delete").await);

        driver.destroy("to_delete").await.unwrap();
        assert!(!driver.exists("to_delete").await);
    }

    #[tokio::test]
    async fn test_redis_driver_cache_key() {
        let driver = RedisSessionDriver::new("myapp:session:");
        assert_eq!(driver.cache_key("abc"), "myapp:session:abc");
    }

    #[tokio::test]
    async fn test_memory_driver_full_lifecycle() {
        let driver = MemorySessionDriver::new();

        // Not found initially
        assert!(driver.read("test").await.is_err());
        assert!(!driver.exists("test").await);

        // Write
        let mut data = HashMap::new();
        data.insert("role".to_string(), Value::String("admin".to_string()));
        driver.write("test", data).await.unwrap();

        // Read back
        assert!(driver.exists("test").await);
        let result = driver.read("test").await.unwrap();
        assert_eq!(
            result.get("role"),
            Some(&Value::String("admin".to_string()))
        );

        // Destroy
        driver.destroy("test").await.unwrap();
        assert!(!driver.exists("test").await);
    }

    #[tokio::test]
    async fn test_memory_driver_gc_removes_expired() {
        let driver = MemorySessionDriver::new();

        let data = HashMap::new();
        driver.write("s1", data.clone()).await.unwrap();
        driver.write("s2", data).await.unwrap();

        // GC with zero lifetime removes all
        let removed = driver.gc(Duration::from_secs(0)).await.unwrap();
        assert_eq!(removed, 2);
    }
}
