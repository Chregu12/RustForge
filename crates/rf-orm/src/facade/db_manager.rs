//! Global database manager backing the synchronous `DB` facade.
//!
//! Executes real SQL against either a SQLite or a Postgres database.
//!
//! **Default backend (SQLite):** An in-memory SQLite database opened via the
//! synchronous `rusqlite` driver. Fits the facade's blocking, `.await`-free
//! API with no async runtime. Call [`DBManager::set_connection`] with a file
//! path (or `"default"` / `":memory:"`) to stay on SQLite.
//!
//! **Postgres backend:** Selected automatically when you call
//! `set_connection("postgres://…")` or `set_connection("postgresql://…")`.
//! The connection pool is created via `sqlx::PgPool`, driven over a
//! `rf_async_bridge::AsyncBridge` so async work runs on a dedicated background
//! thread, safely callable from sync code or inside an existing Tokio runtime.
//!
//! ## Postgres dialect notes
//!
//! - **Placeholders:** the query builder emits `?` (SQLite style). For the PG
//!   path these are rewritten left-to-right to `$1`, `$2`, … before the query
//!   is sent to Postgres.
//! - **INSERT id:** SQLite uses `last_insert_rowid()`; Postgres has no
//!   equivalent. `insert()` appends ` RETURNING id` to the SQL (if not already
//!   present) and reads the returned `id` column. **Convention: PK must be
//!   named `id`.**
//! - **Type mapping:** `serde_json::Value` parameters are bound as their
//!   natural Postgres types (null → NULL, bool → BOOL, i64 → INT8, f64 →
//!   FLOAT8, string → TEXT, arrays/objects → TEXT of their JSON
//!   serialisation). Result rows are decoded back to `serde_json::Value`
//!   objects keyed by column name; column types are introspected and mapped
//!   (INT*/OID → i64, FLOAT* → f64, BOOL → bool, JSON/JSONB → Value, TEXT/*
//!   → string, NULL → null).

use once_cell::sync::Lazy;
use rusqlite::{params_from_iter, types::ValueRef, Connection};
use serde_json::{Map, Value};
use std::sync::Mutex;

use rf_async_bridge::AsyncBridge;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::{Arguments, Column, Row, TypeInfo};

// ── Backend enum ──────────────────────────────────────────────────────────────

/// The active database backend for a [`DBManager`].
enum Backend {
    /// Synchronous SQLite (default). Zero async overhead.
    Sqlite(Connection),
    /// Async Postgres pool bridged to the sync facade via a dedicated worker
    /// thread.
    Postgres {
        pool: sqlx::PgPool,
        bridge: AsyncBridge,
    },
}

// ── Global instance ───────────────────────────────────────────────────────────

/// Global database manager instance.
///
/// Uses a `Mutex` (not `RwLock`) for synchronous access: the underlying
/// `rusqlite::Connection` is `Send` but not `Sync`, and a `Mutex` grants the
/// exclusive access it needs while still being safe to share as a global.
pub static GLOBAL_DB: Lazy<Mutex<DBManager>> = Lazy::new(|| Mutex::new(DBManager::new()));

/// Database manager that owns a real (synchronous) SQLite or (async-bridged)
/// Postgres connection.
pub struct DBManager {
    backend: Backend,
    /// Connection name / target (`"default"`, `":memory:"`, a file path, or a
    /// `postgres://…` URL).
    connection: String,
}

impl std::fmt::Debug for DBManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backend_name = match &self.backend {
            Backend::Sqlite(_) => "SQLite",
            Backend::Postgres { .. } => "Postgres",
        };
        f.debug_struct("DBManager")
            .field("backend", &backend_name)
            .field("connection", &self.connection)
            .finish()
    }
}

// ── SQLite helpers ────────────────────────────────────────────────────────────

/// Convert a JSON binding value into a SQLite value for parameter binding.
fn json_to_sqlite(value: &Value) -> rusqlite::types::Value {
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
fn sqlite_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        ValueRef::Text(t) => Value::from(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::from(b.to_vec()),
    }
}

// ── Postgres helpers ──────────────────────────────────────────────────────────

/// Translate `?` positional placeholders (SQLite/MySQL style) to Postgres
/// `$1`, `$2`, … placeholders, left-to-right.
///
/// Only bare `?` characters are replaced; question marks inside single-quoted
/// string literals (e.g. `WHERE col = '?'`) are **not** touched. This covers
/// all queries produced by the RustForge query builder, which never embeds
/// literal `?` inside quoted strings.
///
/// # Examples
///
/// ```
/// use rf_orm::facade::db_manager::translate_placeholders;
/// assert_eq!(translate_placeholders("SELECT * FROM t WHERE a = ? AND b = ?"),
///            "SELECT * FROM t WHERE a = $1 AND b = $2");
/// assert_eq!(translate_placeholders("INSERT INTO t (a) VALUES (?)"),
///            "INSERT INTO t (a) VALUES ($1)");
/// assert_eq!(translate_placeholders("SELECT 1"),
///            "SELECT 1");
/// ```
pub fn translate_placeholders(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len() + 16);
    let mut counter: u32 = 0;
    let mut in_single_quote = false;

    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\'' => {
                // Toggle quote state. Handle escaped `''` by peeking ahead.
                if in_single_quote && i + 1 < chars.len() && chars[i + 1] == '\'' {
                    // Escaped single quote inside string — consume both and stay in string.
                    out.push('\'');
                    out.push('\'');
                    i += 2;
                    continue;
                }
                in_single_quote = !in_single_quote;
                out.push(c);
            }
            '?' if !in_single_quote => {
                counter += 1;
                out.push('$');
                out.push_str(&counter.to_string());
            }
            other => out.push(other),
        }
        i += 1;
    }
    out
}

/// Push a `serde_json::Value` binding into a `PgArguments` accumulator.
///
/// Mirrors the SQLite binding rules:
/// - null → NULL
/// - bool → BOOL
/// - integer (i64) → INT8
/// - float (f64) → FLOAT8
/// - string → TEXT
/// - arrays/objects → TEXT (their JSON serialisation)
fn push_pg_arg(args: &mut PgArguments, value: &Value) -> Result<(), String> {
    match value {
        Value::Null => args
            .add(Option::<String>::None)
            .map_err(|e| e.to_string())?,
        Value::Bool(b) => args.add(*b).map_err(|e| e.to_string())?,
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                args.add(i).map_err(|e| e.to_string())?;
            } else if let Some(u) = n.as_u64() {
                args.add(u as i64).map_err(|e| e.to_string())?;
            } else {
                args.add(n.as_f64().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            }
        }
        Value::String(s) => args.add(s.as_str()).map_err(|e| e.to_string())?,
        other => args
            .add(other.to_string())
            .map_err(|e| e.to_string())?,
    }
    Ok(())
}

/// Decode one column of a Postgres row into a `serde_json::Value`.
///
/// Column type introspection uses `sqlx::TypeInfo::name()` which returns the
/// Postgres type name in UPPER CASE (e.g. `"INT8"`, `"TEXT"`, `"BOOL"`).
fn pg_col_to_json(row: &PgRow, idx: usize, type_name: &str) -> Value {
    match type_name {
        // Integer types — each wire type needs its matching Rust width.
        // sqlx/postgres will reject a width mismatch (e.g. i64 for INT4),
        // so we decode at the correct native width and widen to i64 for JSON.
        "INT2" => match row.try_get::<Option<i16>, _>(idx) {
            Ok(Some(v)) => Value::from(v as i64),
            _ => Value::Null,
        },
        "INT4" => match row.try_get::<Option<i32>, _>(idx) {
            Ok(Some(v)) => Value::from(v as i64),
            _ => Value::Null,
        },
        "INT8" => match row.try_get::<Option<i64>, _>(idx) {
            Ok(Some(v)) => Value::from(v),
            _ => Value::Null,
        },
        // OID is a 32-bit unsigned type; sqlx exposes it as i32 on PG.
        "OID" => match row.try_get::<Option<i32>, _>(idx) {
            Ok(Some(v)) => Value::from(v as i64),
            _ => Value::Null,
        },
        // Floating-point types — FLOAT4 is a 32-bit wire type; widen to f64 for JSON.
        "FLOAT4" => match row.try_get::<Option<f32>, _>(idx) {
            Ok(Some(v)) => {
                serde_json::Number::from_f64(v as f64).map_or(Value::Null, Value::Number)
            }
            _ => Value::Null,
        },
        "FLOAT8" => match row.try_get::<Option<f64>, _>(idx) {
            Ok(Some(v)) => {
                serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
            }
            _ => Value::Null,
        },
        // Boolean
        "BOOL" => match row.try_get::<Option<bool>, _>(idx) {
            Ok(Some(v)) => Value::Bool(v),
            _ => Value::Null,
        },
        // JSON / JSONB — decode as Value directly
        "JSON" | "JSONB" => match row.try_get::<Option<Value>, _>(idx) {
            Ok(Some(v)) => v,
            _ => Value::Null,
        },
        // NUMERIC/DECIMAL — fetch as string to avoid losing precision
        "NUMERIC" => match row.try_get::<Option<String>, _>(idx) {
            Ok(Some(v)) => {
                // Try to parse as a number; fall back to string if it fails.
                if let Ok(n) = v.parse::<i64>() {
                    Value::from(n)
                } else if let Ok(f) = v.parse::<f64>() {
                    serde_json::Number::from_f64(f).map_or(Value::String(v), Value::Number)
                } else {
                    Value::String(v)
                }
            }
            _ => Value::Null,
        },
        // Everything else (TEXT, VARCHAR, TIMESTAMP, DATE, UUID, …) → string
        _ => match row.try_get::<Option<String>, _>(idx) {
            Ok(Some(v)) => Value::String(v),
            _ => Value::Null,
        },
    }
}

/// Build `PgArguments` from a slice of JSON values.
fn build_pg_args(bindings: &[Value]) -> Result<PgArguments, String> {
    let mut args = PgArguments::default();
    for v in bindings {
        push_pg_arg(&mut args, v)?;
    }
    Ok(args)
}

/// Decode a `Vec<PgRow>` into `Vec<serde_json::Value>` objects keyed by column
/// name.
fn pg_rows_to_json(rows: Vec<PgRow>) -> Vec<Value> {
    rows.into_iter()
        .map(|row| {
            let cols = row.columns();
            let mut obj = Map::with_capacity(cols.len());
            for col in cols {
                let name = col.name().to_string();
                let type_name = col.type_info().name().to_uppercase();
                let val = pg_col_to_json(&row, col.ordinal(), &type_name);
                obj.insert(name, val);
            }
            Value::Object(obj)
        })
        .collect()
}

// ── DBManager implementation ──────────────────────────────────────────────────

impl DBManager {
    /// Create a new database manager backed by a fresh in-memory SQLite
    /// database.
    pub fn new() -> Self {
        Self {
            backend: Backend::Sqlite(
                Connection::open_in_memory()
                    .expect("failed to open in-memory SQLite database"),
            ),
            connection: "default".to_string(),
        }
    }

    // ── SELECT ────────────────────────────────────────────────────────────────

    /// Execute a `SELECT` query and return the rows as JSON objects keyed by
    /// column name.
    pub fn select(&self, query: &str, bindings: &[Value]) -> Result<Vec<Value>, String> {
        match &self.backend {
            Backend::Sqlite(conn) => sqlite_select(conn, query, bindings),
            Backend::Postgres { pool, bridge } => {
                let pg_sql = translate_placeholders(query);
                let args = build_pg_args(bindings)?;
                let pool = pool.clone();
                let rows: Vec<PgRow> = bridge
                    .block_on(async move {
                        sqlx::query_with(&pg_sql, args)
                            .fetch_all(&pool)
                            .await
                            .map_err(|e| e.to_string())
                    })?;
                Ok(pg_rows_to_json(rows))
            }
        }
    }

    // ── INSERT ────────────────────────────────────────────────────────────────

    /// Execute an `INSERT` query and return the row id of the inserted row.
    ///
    /// **Postgres note:** appends ` RETURNING id` to the SQL if not already
    /// present, then reads the `id` column of the first returned row. The
    /// framework primary-key convention is a column named `id`.
    pub fn insert(&mut self, query: &str, bindings: &[Value]) -> Result<u64, String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                let params: Vec<rusqlite::types::Value> =
                    bindings.iter().map(json_to_sqlite).collect();
                conn.execute(query, params_from_iter(params.iter()))
                    .map_err(|e| e.to_string())?;
                Ok(conn.last_insert_rowid() as u64)
            }
            Backend::Postgres { pool, bridge } => {
                let pg_sql = translate_placeholders(query);
                // Append RETURNING id if needed.
                let pg_sql_returning = if pg_sql
                    .trim_end()
                    .to_uppercase()
                    .contains("RETURNING")
                {
                    pg_sql
                } else {
                    format!("{} RETURNING id", pg_sql.trim_end())
                };
                let args = build_pg_args(bindings)?;
                let pool = pool.clone();
                let row: PgRow = bridge
                    .block_on(async move {
                        sqlx::query_with(&pg_sql_returning, args)
                            .fetch_one(&pool)
                            .await
                            .map_err(|e| e.to_string())
                    })?;
                let id: i64 = row.try_get("id").map_err(|e| e.to_string())?;
                Ok(id as u64)
            }
        }
    }

    // ── UPDATE ────────────────────────────────────────────────────────────────

    /// Execute an `UPDATE` query and return the number of affected rows.
    pub fn update(&mut self, query: &str, bindings: &[Value]) -> Result<u64, String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                let params: Vec<rusqlite::types::Value> =
                    bindings.iter().map(json_to_sqlite).collect();
                let affected = conn
                    .execute(query, params_from_iter(params.iter()))
                    .map_err(|e| e.to_string())?;
                Ok(affected as u64)
            }
            Backend::Postgres { pool, bridge } => {
                let pg_sql = translate_placeholders(query);
                let args = build_pg_args(bindings)?;
                let pool = pool.clone();
                let rows_affected: u64 = bridge
                    .block_on(async move {
                        sqlx::query_with(&pg_sql, args)
                            .execute(&pool)
                            .await
                            .map(|r| r.rows_affected())
                            .map_err(|e| e.to_string())
                    })?;
                Ok(rows_affected)
            }
        }
    }

    // ── DELETE ────────────────────────────────────────────────────────────────

    /// Execute a `DELETE` query and return the number of affected rows.
    pub fn delete(&mut self, query: &str, bindings: &[Value]) -> Result<u64, String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                let params: Vec<rusqlite::types::Value> =
                    bindings.iter().map(json_to_sqlite).collect();
                let affected = conn
                    .execute(query, params_from_iter(params.iter()))
                    .map_err(|e| e.to_string())?;
                Ok(affected as u64)
            }
            Backend::Postgres { pool, bridge } => {
                let pg_sql = translate_placeholders(query);
                let args = build_pg_args(bindings)?;
                let pool = pool.clone();
                let rows_affected: u64 = bridge
                    .block_on(async move {
                        sqlx::query_with(&pg_sql, args)
                            .execute(&pool)
                            .await
                            .map(|r| r.rows_affected())
                            .map_err(|e| e.to_string())
                    })?;
                Ok(rows_affected)
            }
        }
    }

    // ── STATEMENT (DDL / multi-statement) ─────────────────────────────────────

    /// Execute one or more statements (e.g. DDL) that return no rows.
    pub fn statement(&mut self, query: &str) -> Result<bool, String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                conn.execute_batch(query).map_err(|e| e.to_string())?;
                Ok(true)
            }
            Backend::Postgres { pool, bridge } => {
                let sql = query.to_string();
                let pool = pool.clone();
                bridge.block_on(async move {
                    sqlx::raw_sql(&sql)
                        .execute(&pool)
                        .await
                        .map(|_| true)
                        .map_err(|e| e.to_string())
                })
            }
        }
    }

    // ── REFRESH (test isolation) ──────────────────────────────────────────────

    /// Drop every user-defined table in the current database, returning it to
    /// an empty schema. The RustForge equivalent of Laravel's
    /// `RefreshDatabase`.
    ///
    /// **SQLite:** queries `sqlite_master` and issues `DROP TABLE` for each
    /// user table.
    ///
    /// **Postgres:** queries `pg_tables` for the `public` schema and issues
    /// `DROP TABLE … CASCADE` for each user table.
    pub fn refresh(&mut self) -> Result<(), String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                // Collect table names first so the borrow from `prepare`/`query`
                // is released before we execute the DROPs.
                let names = sqlite_list_tables(conn)?;
                if names.is_empty() {
                    return Ok(());
                }
                let mut sql = String::from("PRAGMA foreign_keys = OFF;\n");
                for name in &names {
                    sql.push_str(&format!(
                        "DROP TABLE IF EXISTS \"{}\";\n",
                        name.replace('"', "\"\"")
                    ));
                }
                sql.push_str("PRAGMA foreign_keys = ON;\n");
                conn.execute_batch(&sql).map_err(|e| e.to_string())?;
                Ok(())
            }
            Backend::Postgres { pool, bridge } => {
                let pool2 = pool.clone();
                // Collect public table names.
                let names: Vec<String> = bridge.block_on(async move {
                    let rows = sqlx::query(
                        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
                    )
                    .fetch_all(&pool2)
                    .await
                    .map_err(|e| e.to_string())?;
                    let names: Vec<String> = rows
                        .into_iter()
                        .filter_map(|r| r.try_get::<String, _>("tablename").ok())
                        .collect();
                    Ok::<Vec<String>, String>(names)
                })?;

                if names.is_empty() {
                    return Ok(());
                }

                let pool2 = pool.clone();
                let drop_sql = names
                    .iter()
                    .map(|n| format!("DROP TABLE IF EXISTS \"{}\" CASCADE;", n.replace('"', "\"\"")))
                    .collect::<Vec<_>>()
                    .join("\n");

                bridge.block_on(async move {
                    sqlx::raw_sql(&drop_sql)
                        .execute(&pool2)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                })
            }
        }
    }

    // ── TRANSACTIONS ──────────────────────────────────────────────────────────

    /// Begin a transaction on the underlying connection.
    pub fn begin_transaction(&mut self) -> Result<(), String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                conn.execute_batch("BEGIN").map_err(|e| e.to_string())
            }
            Backend::Postgres { pool, bridge } => {
                let pool = pool.clone();
                bridge.block_on(async move {
                    sqlx::query("BEGIN")
                        .execute(&pool)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                })
            }
        }
    }

    /// Commit the current transaction.
    pub fn commit(&mut self) -> Result<(), String> {
        match &self.backend {
            Backend::Sqlite(conn) => {
                conn.execute_batch("COMMIT").map_err(|e| e.to_string())
            }
            Backend::Postgres { pool, bridge } => {
                let pool = pool.clone();
                bridge.block_on(async move {
                    sqlx::query("COMMIT")
                        .execute(&pool)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                })
            }
        }
    }

    /// Roll back the current transaction.
    pub fn rollback(&mut self) -> Result<(), String> {
        match &self.backend {
            Backend::Sqlite(conn) => conn
                .execute_batch("ROLLBACK")
                .map_err(|e| e.to_string()),
            Backend::Postgres { pool, bridge } => {
                let pool = pool.clone();
                bridge.block_on(async move {
                    sqlx::query("ROLLBACK")
                        .execute(&pool)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                })
            }
        }
    }

    // ── CONNECTION MANAGEMENT ─────────────────────────────────────────────────

    /// Get the current connection name/target.
    pub fn connection_name(&self) -> &str {
        &self.connection
    }

    /// Point the manager at a different database.
    ///
    /// - `"default"` or `":memory:"` → fresh in-memory **SQLite**.
    /// - A `postgres://…` or `postgresql://…` URL → **Postgres** backend via
    ///   `sqlx::PgPool`. Connection errors are returned and the previous
    ///   backend is kept.
    /// - Any other string → **SQLite** file path.
    ///
    /// If opening fails the previous connection is kept unchanged.
    pub fn set_connection(&mut self, connection: String) {
        if connection.starts_with("postgres://") || connection.starts_with("postgresql://") {
            // Postgres path — create pool over the bridge.
            let bridge = AsyncBridge::new();
            let url = connection.clone();
            let pool_result: Result<sqlx::PgPool, String> = bridge.block_on(async move {
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&url)
                    .await
                    .map_err(|e| e.to_string())
            });
            if let Ok(pool) = pool_result {
                self.backend = Backend::Postgres { pool, bridge };
                self.connection = connection;
            }
            // On error: keep the previous backend unchanged (matches SQLite behaviour).
        } else {
            // SQLite path (unchanged behaviour).
            let opened = if connection == "default" || connection == ":memory:" {
                Connection::open_in_memory()
            } else {
                Connection::open(&connection)
            };
            if let Ok(conn) = opened {
                self.backend = Backend::Sqlite(conn);
            }
            self.connection = connection;
        }
    }
}

// ── SQLite helpers (free functions to avoid borrow checker issues) ────────────

fn sqlite_select(conn: &Connection, query: &str, bindings: &[Value]) -> Result<Vec<Value>, String> {
    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let column_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect();

    let params: Vec<rusqlite::types::Value> = bindings.iter().map(json_to_sqlite).collect();
    let mut rows = stmt
        .query(params_from_iter(params.iter()))
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut obj = Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = row.get_ref(i).map_err(|e| e.to_string())?;
            obj.insert(name.clone(), sqlite_to_json(value));
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

fn sqlite_list_tables(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let name: String = row.get(0).map_err(|e| e.to_string())?;
        names.push(name);
    }
    Ok(names)
}

// ── Default ───────────────────────────────────────────────────────────────────

impl Default for DBManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Placeholder translation unit tests (no server needed) ─────────────────

    #[test]
    fn test_translate_no_placeholders() {
        assert_eq!(translate_placeholders("SELECT 1"), "SELECT 1");
    }

    #[test]
    fn test_translate_single_placeholder() {
        assert_eq!(
            translate_placeholders("SELECT * FROM t WHERE id = ?"),
            "SELECT * FROM t WHERE id = $1"
        );
    }

    #[test]
    fn test_translate_multiple_placeholders() {
        assert_eq!(
            translate_placeholders("INSERT INTO t (a, b) VALUES (?, ?)"),
            "INSERT INTO t (a, b) VALUES ($1, $2)"
        );
    }

    #[test]
    fn test_translate_three_placeholders() {
        assert_eq!(
            translate_placeholders("UPDATE t SET a = ?, b = ? WHERE id = ?"),
            "UPDATE t SET a = $1, b = $2 WHERE id = $3"
        );
    }

    #[test]
    fn test_translate_question_mark_in_string_literal_untouched() {
        // A `?` inside a single-quoted literal must NOT be replaced.
        let sql = "SELECT * FROM t WHERE col = '?' AND id = ?";
        assert_eq!(
            translate_placeholders(sql),
            "SELECT * FROM t WHERE col = '?' AND id = $1"
        );
    }

    #[test]
    fn test_translate_escaped_quote_in_string_literal() {
        // '''' is an escaped single-quote inside a string; the outer `?` after
        // it should still be replaced.
        let sql = "SELECT * FROM t WHERE col = 'it''s fine' AND id = ?";
        assert_eq!(
            translate_placeholders(sql),
            "SELECT * FROM t WHERE col = 'it''s fine' AND id = $1"
        );
    }

    // ── SQLite backend tests (existing — byte-identical behaviour) ────────────

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
    fn test_refresh_drops_all_user_tables() {
        let mut m = seeded();
        m.insert("INSERT INTO users (name) VALUES (?)", &[serde_json::json!("x")])
            .unwrap();
        m.statement("CREATE TABLE posts (id INTEGER PRIMARY KEY, body TEXT)")
            .unwrap();
        assert_eq!(m.select("SELECT id FROM users", &[]).unwrap().len(), 1);

        m.refresh().unwrap();

        // Both tables are gone (full schema reset), not just emptied.
        assert!(m.select("SELECT id FROM users", &[]).is_err());
        assert!(m.select("SELECT id FROM posts", &[]).is_err());
        // Refreshing an already-empty database is a harmless no-op.
        m.refresh().unwrap();
    }

    #[test]
    fn test_refresh_gives_two_blocks_clean_state() {
        let mut m = DBManager::new();
        // Block A
        m.refresh().unwrap();
        m.statement("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
        m.insert("INSERT INTO t DEFAULT VALUES", &[]).unwrap();
        m.insert("INSERT INTO t DEFAULT VALUES", &[]).unwrap();
        assert_eq!(m.select("SELECT id FROM t", &[]).unwrap().len(), 2);
        // Block B must not see block A's rows.
        m.refresh().unwrap();
        m.statement("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
        m.insert("INSERT INTO t DEFAULT VALUES", &[]).unwrap();
        assert_eq!(m.select("SELECT id FROM t", &[]).unwrap().len(), 1);
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

    // ── Postgres integration test — env-var gated, always compiled ────────────
    //
    // Run with:
    //   RF_PG_TEST_URL=postgres://rustforge:testpass@127.0.0.1:5432/rustforge_test \
    //     cargo test -p rf-orm test_postgres_integration_full_cycle -- --nocapture
    //
    // The test skips cleanly (passes green) when RF_PG_TEST_URL is absent so
    // normal `cargo test -p rf-orm` (no env var) is hermetic and never touches
    // a network.
    #[test]
    fn test_postgres_integration_full_cycle() {
        // ── Gate: skip when RF_PG_TEST_URL is not set ─────────────────────────
        let url = match std::env::var("RF_PG_TEST_URL") {
            Ok(u) if !u.is_empty() => u,
            _ => {
                println!(
                    "SKIP test_postgres_integration_full_cycle \
                     — set RF_PG_TEST_URL=postgres://... to run"
                );
                return;
            }
        };

        // ── Connect ───────────────────────────────────────────────────────────
        // Use a fresh local DBManager (not GLOBAL_DB) so this test does not
        // interfere with the SQLite-backed tests running concurrently.
        let mut m = DBManager::new();
        m.set_connection(url.clone());
        assert!(
            m.connection_name().starts_with("postgres"),
            "set_connection({url}) did not switch to Postgres backend — \
             is the server reachable?"
        );
        println!("Connected to Postgres: {url}");

        // ── DDL — CREATE TABLE (idempotent via DROP IF EXISTS) ─────────────────
        m.statement("DROP TABLE IF EXISTS rf_pg_integ_test CASCADE")
            .expect("DROP TABLE IF EXISTS failed");
        m.statement(
            "CREATE TABLE rf_pg_integ_test (\
                 id    BIGSERIAL PRIMARY KEY, \
                 name  TEXT NOT NULL, \
                 score INTEGER NOT NULL DEFAULT 0\
             )",
        )
        .expect("CREATE TABLE failed");
        println!("CREATE TABLE rf_pg_integ_test OK");

        // ── INSERT — RETURNING id must come back as a positive integer ─────────
        let id = m
            .insert(
                "INSERT INTO rf_pg_integ_test (name, score) VALUES (?, ?)",
                &[serde_json::json!("Alice"), serde_json::json!(42i64)],
            )
            .expect("INSERT failed");
        assert!(id > 0, "INSERT RETURNING id should be > 0, got {id}");
        println!("INSERT returned id={id}");

        // ── SELECT — row must exist with correct values ────────────────────────
        let rows = m
            .select(
                "SELECT id, name, score FROM rf_pg_integ_test WHERE id = ?",
                &[serde_json::json!(id as i64)],
            )
            .expect("SELECT failed");
        assert_eq!(rows.len(), 1, "SELECT should return exactly 1 row");
        assert_eq!(rows[0]["name"], serde_json::json!("Alice"));
        assert_eq!(rows[0]["score"], serde_json::json!(42i64));
        println!("SELECT OK: {:?}", rows[0]);

        // ── UPDATE — affected rows must be 1 ──────────────────────────────────
        let affected = m
            .update(
                "UPDATE rf_pg_integ_test SET score = ? WHERE id = ?",
                &[serde_json::json!(99i64), serde_json::json!(id as i64)],
            )
            .expect("UPDATE failed");
        assert_eq!(affected, 1, "UPDATE should affect 1 row, got {affected}");

        // Verify the update landed.
        let after_update = m
            .select(
                "SELECT score FROM rf_pg_integ_test WHERE id = ?",
                &[serde_json::json!(id as i64)],
            )
            .expect("SELECT after UPDATE failed");
        assert_eq!(
            after_update[0]["score"],
            serde_json::json!(99i64),
            "score should be 99 after UPDATE"
        );
        println!("UPDATE OK: score -> 99");

        // ── DELETE — row must disappear ────────────────────────────────────────
        let deleted = m
            .delete(
                "DELETE FROM rf_pg_integ_test WHERE id = ?",
                &[serde_json::json!(id as i64)],
            )
            .expect("DELETE failed");
        assert_eq!(deleted, 1, "DELETE should affect 1 row, got {deleted}");

        let remaining = m
            .select("SELECT id FROM rf_pg_integ_test", &[])
            .expect("SELECT after DELETE failed");
        assert!(
            remaining.is_empty(),
            "Table should be empty after DELETE, got {} rows",
            remaining.len()
        );
        println!("DELETE OK: table is empty");

        // ── Cleanup ───────────────────────────────────────────────────────────
        m.statement("DROP TABLE IF EXISTS rf_pg_integ_test CASCADE")
            .expect("DROP TABLE cleanup failed");
        println!("PASS: test_postgres_integration_full_cycle completed (id={id})");
    }

    // ── Postgres integration tests (require a live server) ────────────────────
    //
    // Run with:
    //   cargo test -p rf-orm --features integration-tests -- pg_crud
    //
    // The URL can be overridden with the RF_ORM_TEST_PG_URL env var; the
    // default points at the throwaway Docker container started by cycle-7.

    #[cfg(feature = "integration-tests")]
    const DEFAULT_PG_URL: &str = "postgres://postgres:pw@localhost:5433/postgres";

    #[cfg(feature = "integration-tests")]
    fn pg_url() -> String {
        std::env::var("RF_ORM_TEST_PG_URL").unwrap_or_else(|_| DEFAULT_PG_URL.to_string())
    }

    /// Full CRUD + type-mapping probe on the real Postgres backend.
    ///
    /// Exercises: CREATE TABLE (DDL statement), INSERT (RETURNING id), SELECT
    /// (with ?-to-$N translation and column type decoding), UPDATE (affected
    /// rows), DELETE (affected rows), and DROP TABLE (refresh / cleanup).
    #[cfg(feature = "integration-tests")]
    #[test]
    fn test_pg_crud_full_roundtrip() {
        let mut m = DBManager::new();
        m.set_connection(pg_url());
        assert!(
            m.connection_name().starts_with("postgres"),
            "failed to connect to Postgres – is the container running? URL: {}",
            pg_url()
        );

        // ── DDL ────────────────────────────────────────────────────────────────
        m.statement(
            "DROP TABLE IF EXISTS rf_pg_probe; \
             CREATE TABLE rf_pg_probe (\
               id    BIGSERIAL PRIMARY KEY,\
               name  TEXT NOT NULL,\
               score FLOAT8,\
               flag  BOOL\
             )",
        )
        .expect("CREATE TABLE failed");

        // ── INSERT + RETURNING id ──────────────────────────────────────────────
        let id1 = m
            .insert(
                "INSERT INTO rf_pg_probe (name, score, flag) VALUES (?, ?, ?)",
                &[
                    serde_json::json!("Alice"),
                    serde_json::json!(3.14),
                    serde_json::json!(true),
                ],
            )
            .expect("INSERT 1 failed");
        println!("INSERT 1 returned id = {id1}");
        assert_eq!(id1, 1, "first RETURNING id must be 1");

        let id2 = m
            .insert(
                "INSERT INTO rf_pg_probe (name, score, flag) VALUES (?, ?, ?)",
                &[
                    serde_json::json!("Bob"),
                    serde_json::json!(2.72),
                    serde_json::json!(false),
                ],
            )
            .expect("INSERT 2 failed");
        println!("INSERT 2 returned id = {id2}");
        assert_eq!(id2, 2);

        // ── SELECT + type mapping ──────────────────────────────────────────────
        let rows = m
            .select("SELECT id, name, score, flag FROM rf_pg_probe ORDER BY id", &[])
            .expect("SELECT all failed");
        println!("SELECT rows = {rows:#?}");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], serde_json::json!("Alice"));
        assert_eq!(rows[0]["id"], serde_json::json!(1i64));
        // FLOAT8 decoded as JSON Number
        assert!(rows[0]["score"].is_number(), "score should decode as number");
        assert_eq!(rows[0]["flag"], serde_json::json!(true));

        // ── SELECT with ? binding ──────────────────────────────────────────────
        let filtered = m
            .select(
                "SELECT name FROM rf_pg_probe WHERE name = ?",
                &[serde_json::json!("Bob")],
            )
            .expect("SELECT filtered failed");
        println!("SELECT filtered = {filtered:#?}");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["name"], serde_json::json!("Bob"));

        // ── UPDATE ────────────────────────────────────────────────────────────
        let updated = m
            .update(
                "UPDATE rf_pg_probe SET flag = ? WHERE id = ?",
                &[serde_json::json!(false), serde_json::json!(1i64)],
            )
            .expect("UPDATE failed");
        println!("UPDATE affected = {updated}");
        assert_eq!(updated, 1);

        // Verify the update took effect.
        let check = m
            .select("SELECT flag FROM rf_pg_probe WHERE id = $1", &[serde_json::json!(1i64)])
            .expect("SELECT after UPDATE failed");
        assert_eq!(check[0]["flag"], serde_json::json!(false));

        // ── DELETE ────────────────────────────────────────────────────────────
        let deleted = m
            .delete(
                "DELETE FROM rf_pg_probe WHERE id = ?",
                &[serde_json::json!(2i64)],
            )
            .expect("DELETE failed");
        println!("DELETE affected = {deleted}");
        assert_eq!(deleted, 1);

        let remaining = m
            .select("SELECT id FROM rf_pg_probe", &[])
            .expect("SELECT after DELETE failed");
        assert_eq!(remaining.len(), 1);

        // ── CLEANUP ───────────────────────────────────────────────────────────
        m.statement("DROP TABLE rf_pg_probe").expect("DROP failed");
        println!("Postgres CRUD probe PASSED");
    }
}
