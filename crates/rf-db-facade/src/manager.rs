//! Global database manager

use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::RwLock;

/// Global database manager instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_DB: Lazy<RwLock<DBManager>> = Lazy::new(|| {
    RwLock::new(DBManager::new())
});

/// Database manager that holds connection state
#[derive(Debug)]
pub struct DBManager {
    /// Mock database records
    records: Vec<Value>,
    /// Connection name
    connection: String,
}

impl DBManager {
    /// Create a new database manager
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            connection: "default".to_string(),
        }
    }

    /// Execute a select query
    pub fn select(&self, _query: &str, _bindings: &[Value]) -> Result<Vec<Value>, String> {
        // Simplified implementation - returns mock data
        Ok(self.records.clone())
    }

    /// Execute an insert query
    pub fn insert(&mut self, _query: &str, bindings: &[Value]) -> Result<u64, String> {
        // Create a mock record
        let mut record = serde_json::json!({
            "id": self.records.len() as u64 + 1
        });

        // Add bindings as fields
        for (i, binding) in bindings.iter().enumerate() {
            record[format!("field_{}", i)] = binding.clone();
        }

        self.records.push(record);
        Ok(self.records.len() as u64)
    }

    /// Execute an update query
    pub fn update(&mut self, _query: &str, _bindings: &[Value]) -> Result<u64, String> {
        // Simplified - affects all records
        Ok(self.records.len() as u64)
    }

    /// Execute a delete query
    pub fn delete(&mut self, _query: &str, _bindings: &[Value]) -> Result<u64, String> {
        let count = self.records.len() as u64;
        self.records.clear();
        Ok(count)
    }

    /// Execute a statement
    pub fn statement(&mut self, _query: &str) -> Result<bool, String> {
        Ok(true)
    }

    /// Get the current connection name
    pub fn connection_name(&self) -> &str {
        &self.connection
    }

    /// Set the connection name
    pub fn set_connection(&mut self, connection: String) {
        self.connection = connection;
    }

    /// Begin a transaction (mock)
    pub fn begin_transaction(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Commit a transaction (mock)
    pub fn commit(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Rollback a transaction (mock)
    pub fn rollback(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Get all records (for testing)
    pub fn all_records(&self) -> &[Value] {
        &self.records
    }
}

impl Default for DBManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_manager_new() {
        let manager = DBManager::new();
        assert_eq!(manager.connection_name(), "default");
        assert_eq!(manager.all_records().len(), 0);
    }

    #[test]
    fn test_db_manager_insert() {
        let mut manager = DBManager::new();
        let bindings = vec![
            serde_json::json!("John"),
            serde_json::json!("john@example.com")
        ];

        let id = manager.insert("INSERT INTO users (name, email) VALUES (?, ?)", &bindings).unwrap();
        assert_eq!(id, 1);
        assert_eq!(manager.all_records().len(), 1);
    }

    #[test]
    fn test_db_manager_select() {
        let mut manager = DBManager::new();
        manager.insert("INSERT", &[serde_json::json!("test")]).unwrap();

        let results = manager.select("SELECT * FROM users", &[]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_db_manager_update() {
        let mut manager = DBManager::new();
        manager.insert("INSERT", &[serde_json::json!("test")]).unwrap();

        let affected = manager.update("UPDATE users SET name = ?", &[serde_json::json!("updated")]).unwrap();
        assert_eq!(affected, 1);
    }

    #[test]
    fn test_db_manager_delete() {
        let mut manager = DBManager::new();
        manager.insert("INSERT", &[serde_json::json!("test")]).unwrap();

        let deleted = manager.delete("DELETE FROM users", &[]).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(manager.all_records().len(), 0);
    }

    #[test]
    fn test_db_manager_connection() {
        let mut manager = DBManager::new();
        assert_eq!(manager.connection_name(), "default");

        manager.set_connection("mysql".to_string());
        assert_eq!(manager.connection_name(), "mysql");
    }

    #[test]
    fn test_db_manager_statement() {
        let mut manager = DBManager::new();
        let result = manager.statement("CREATE TABLE users (id INT)").unwrap();
        assert!(result);
    }

    #[test]
    fn test_db_manager_transaction() {
        let mut manager = DBManager::new();
        assert!(manager.begin_transaction().is_ok());
        assert!(manager.commit().is_ok());
        assert!(manager.rollback().is_ok());
    }
}
