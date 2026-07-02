//! Global database manager backing the synchronous `DB` facade.
//!
//! Executes real SQL against a SQLite database using the synchronous `rusqlite`
//! driver, which fits the facade's blocking, `.await`-free API without needing an
//! async runtime. The default connection is an in-memory database; call
//! [`DBManager::set_connection`] with a file path to persist to disk.

use once_cell::sync::Lazy;
use rusqlite::{params_from_iter, types::ValueRef, Connection};
use serde_json::{Map, Value};
use std::sync::Mutex;

/// Global database manager instance.
///
/// Uses a `Mutex` (not `RwLock`) for synchronous access: the underlying
/// `rusqlite::Connection` is `Send` but not `Sync`, and a `Mutex` grants the
/// exclusive access it needs while still being safe to share as a global.
pub static GLOBAL_DB: Lazy<Mutex<DBManager>> = Lazy::new(|| Mutex::new(DBManager::new()));

/// Database manager that owns a real (synchronous) SQLite connection.
pub struct DBManager {
    /// Live SQLite connection used to execute all queries.
    conn: Connection,
    /// Connection name / target (`"default"`/`":memory:"` or a file path).
    connection: String,
}

impl std::fmt::Debug for DBManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DBManager")
            .field("connection", &self.connection)
            .finish()
    }
}

/// Convert a JSON binding value into a SQLite value for parameter binding.
fn json_to_sql(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(*b as i64),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Sql::Integer(i)
            } else if let Some(u) = n.as_u64() {
                Sql::Integer(u as i64)
            } else {
                Sql::Real(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => Sql::Text(s.clone()),
        // Arrays/objects are stored as their JSON text representation.
        other => Sql::Text(other.to_string()),
    }
}

/// Convert a SQLite column value into a JSON value for result rows.
fn sql_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        ValueRef::Text(t) => Value::from(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::from(b.to_vec()),
    }
}

impl DBManager {
    /// Create a new database manager backed by a fresh in-memory SQLite database.
    pub fn new() -> Self {
        Self {
            conn: Connection::open_in_memory().expect("failed to open in-memory SQLite database"),
            connection: "default".to_string(),
        }
    }

    /// Execute a `SELECT` query and return the rows as JSON objects keyed by column name.
    pub fn select(&self, query: &str, bindings: &[Value]) -> Result<Vec<Value>, String> {
        let mut stmt = self.conn.prepare(query).map_err(|e| e.to_string())?;
        let column_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect();

        let params: Vec<rusqlite::types::Value> = bindings.iter().map(json_to_sql).collect();
        let mut rows = stmt
            .query(params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut obj = Map::new();
            for (i, name) in column_names.iter().enumerate() {
                let value = row.get_ref(i).map_err(|e| e.to_string())?;
                obj.insert(name.clone(), sql_to_json(value));
            }
            out.push(Value::Object(obj));
        }
        Ok(out)
    }

    /// Execute an `INSERT` query and return the row id of the inserted row.
    pub fn insert(&mut self, query: &str, bindings: &[Value]) -> Result<u64, String> {
        let params: Vec<rusqlite::types::Value> = bindings.iter().map(json_to_sql).collect();
        self.conn
            .execute(query, params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid() as u64)
    }

    /// Execute an `UPDATE` query and return the number of affected rows.
    pub fn update(&mut self, query: &str, bindings: &[Value]) -> Result<u64, String> {
        let params: Vec<rusqlite::types::Value> = bindings.iter().map(json_to_sql).collect();
        let affected = self
            .conn
            .execute(query, params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;
        Ok(affected as u64)
    }

    /// Execute a `DELETE` query and return the number of affected rows.
    pub fn delete(&mut self, query: &str, bindings: &[Value]) -> Result<u64, String> {
        let params: Vec<rusqlite::types::Value> = bindings.iter().map(json_to_sql).collect();
        let affected = self
            .conn
            .execute(query, params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;
        Ok(affected as u64)
    }

    /// Execute one or more statements (e.g. DDL) that return no rows.
    pub fn statement(&mut self, query: &str) -> Result<bool, String> {
        self.conn.execute_batch(query).map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Get the current connection name/target.
    pub fn connection_name(&self) -> &str {
        &self.connection
    }

    /// Point the manager at a different SQLite database.
    ///
    /// `"default"` or `":memory:"` open a fresh in-memory database; any other value
    /// is treated as a file path. If opening fails the previous connection is kept.
    pub fn set_connection(&mut self, connection: String) {
        let opened = if connection == "default" || connection == ":memory:" {
            Connection::open_in_memory()
        } else {
            Connection::open(&connection)
        };
        if let Ok(conn) = opened {
            self.conn = conn;
        }
        self.connection = connection;
    }

    /// Begin a transaction on the underlying connection.
    pub fn begin_transaction(&mut self) -> Result<(), String> {
        self.conn.execute_batch("BEGIN").map_err(|e| e.to_string())
    }

    /// Commit the current transaction.
    pub fn commit(&mut self) -> Result<(), String> {
        self.conn.execute_batch("COMMIT").map_err(|e| e.to_string())
    }

    /// Roll back the current transaction.
    pub fn rollback(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch("ROLLBACK")
            .map_err(|e| e.to_string())
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

    fn seeded() -> DBManager {
        let mut m = DBManager::new();
        m.statement("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, active INTEGER)")
            .unwrap();
        m
    }

    #[test]
    fn test_insert_returns_row_id_and_select_reads_it_back() {
        let mut m = seeded();
        let id = m
            .insert(
                "INSERT INTO users (name, active) VALUES (?, ?)",
                &[serde_json::json!("John"), serde_json::json!(true)],
            )
            .unwrap();
        assert_eq!(id, 1);

        let rows = m.select("SELECT id, name, active FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], serde_json::json!("John"));
        assert_eq!(rows[0]["id"], serde_json::json!(1));
    }

    #[test]
    fn test_select_honors_bindings() {
        let mut m = seeded();
        m.insert("INSERT INTO users (name) VALUES (?)", &[serde_json::json!("Alice")])
            .unwrap();
        m.insert("INSERT INTO users (name) VALUES (?)", &[serde_json::json!("Bob")])
            .unwrap();

        let rows = m
            .select(
                "SELECT name FROM users WHERE name = ?",
                &[serde_json::json!("Bob")],
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], serde_json::json!("Bob"));
    }

    #[test]
    fn test_update_and_delete_report_affected_rows() {
        let mut m = seeded();
        for name in ["a", "b", "c"] {
            m.insert(
                "INSERT INTO users (name, active) VALUES (?, 0)",
                &[serde_json::json!(name)],
            )
            .unwrap();
        }

        let updated = m
            .update("UPDATE users SET active = ?", &[serde_json::json!(1)])
            .unwrap();
        assert_eq!(updated, 3);

        let deleted = m
            .delete("DELETE FROM users WHERE name = ?", &[serde_json::json!("b")])
            .unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(m.select("SELECT id FROM users", &[]).unwrap().len(), 2);
    }

    #[test]
    fn test_transaction_rollback_discards_changes() {
        let mut m = seeded();
        m.begin_transaction().unwrap();
        m.insert("INSERT INTO users (name) VALUES (?)", &[serde_json::json!("temp")])
            .unwrap();
        m.rollback().unwrap();
        assert_eq!(m.select("SELECT id FROM users", &[]).unwrap().len(), 0);
    }

    #[test]
    fn test_select_on_missing_table_errors() {
        let m = DBManager::new();
        assert!(m.select("SELECT * FROM nope", &[]).is_err());
    }

    #[test]
    fn test_connection_name() {
        let mut m = DBManager::new();
        assert_eq!(m.connection_name(), "default");
        m.set_connection(":memory:".to_string());
        assert_eq!(m.connection_name(), ":memory:");
    }
}
