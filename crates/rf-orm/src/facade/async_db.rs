//! Shared async execution for models, the DB facade and injected services.
//!
//! A scoped SeaORM connection is the application's existing pool. Legacy
//! synchronous DB configuration remains available; its I/O is offloaded as one
//! blocking operation, never polled on a Tokio request worker.

use super::db_manager;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, FromQueryResult,
    Statement, TransactionTrait,
};
use serde_json::Value;
use std::{future::Future, sync::Arc};

#[derive(Clone)]
enum Executor {
    Pool(Arc<DatabaseConnection>),
    Transaction(Arc<DatabaseTransaction>),
}

tokio::task_local! {
    static DATABASE: Executor;
}

pub(super) fn ensure_sync_context() -> Result<(), String> {
    if DATABASE.try_with(|_| ()).is_ok() {
        Err("Synchronous DB calls cannot use an async database scope; use the corresponding _async method".into())
    } else {
        Ok(())
    }
}

/// Run model/facade operations against an application's owned connection pool.
/// Nested scopes restore the previous connection. Spawned tasks must establish
/// their own scope explicitly; there is no process-global reconfiguration.
pub async fn with_database<F: Future>(
    connection: impl Into<Arc<DatabaseConnection>>,
    future: F,
) -> F::Output {
    DATABASE
        .scope(Executor::Pool(connection.into()), future)
        .await
}

/// Run an async operation atomically on the scoped pool. Nested calls use
/// savepoints. Dropping the future drops the transaction and queues rollback.
pub async fn transaction<F, T>(future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let current = DATABASE.try_with(Clone::clone).map_err(|_| {
        "DB::transaction requires DatabaseManager::scope or rf::App::database".to_string()
    })?;
    let tx = match current {
        Executor::Pool(conn) => conn.begin().await,
        Executor::Transaction(tx) => tx.begin().await,
    }
    .map_err(|e| e.to_string())?;
    let tx = Arc::new(tx);
    let result = DATABASE
        .scope(Executor::Transaction(tx.clone()), future)
        .await;
    let tx = Arc::try_unwrap(tx).map_err(|_| "Transaction is still in use".to_string())?;
    match result {
        Ok(value) => {
            tx.commit().await.map_err(|e| e.to_string())?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback()
                .await
                .map_err(|e| format!("{error}; rollback failed: {e}"))?;
            Err(error)
        }
    }
}

fn statement(backend: DbBackend, sql: &str, bindings: &[Value]) -> Result<Statement, String> {
    let sql = if backend == DbBackend::Postgres {
        db_manager::translate_placeholders(sql)
    } else {
        sql.to_string()
    };
    let values = bindings
        .iter()
        .map(|value| {
            Ok(match value {
                Value::Null => sea_orm::Value::String(None),
                Value::Bool(v) => (*v).into(),
                Value::Number(v) => {
                    if let Some(n) = v.as_i64() {
                        n.into()
                    } else if let Some(n) = v.as_u64() {
                        i64::try_from(n)
                            .map_err(|_| "Integer exceeds signed database range".to_string())?
                            .into()
                    } else {
                        v.as_f64().ok_or("Invalid numeric value")?.into()
                    }
                }
                Value::String(v) => v.clone().into(),
                value => sea_orm::Value::Json(Some(Box::new(value.clone()))),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Statement::from_sql_and_values(backend, sql, values))
}

async fn select_on<C: ConnectionTrait>(
    conn: &C,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<Value>, String> {
    Value::find_by_statement(statement(conn.get_database_backend(), sql, bindings)?)
        .all(conn)
        .await
        .map_err(|e| e.to_string())
}

async fn execute_on<C: ConnectionTrait>(
    conn: &C,
    sql: &str,
    bindings: &[Value],
    insert: bool,
) -> Result<u64, String> {
    let backend = conn.get_database_backend();
    if insert && backend == DbBackend::Postgres {
        let sql = format!("{} RETURNING id", sql.trim_end().trim_end_matches(';'));
        let row = conn
            .query_one(statement(backend, &sql, bindings)?)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("INSERT returned no id")?;
        let id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        return u64::try_from(id).map_err(|e| e.to_string());
    }
    let result = conn
        .execute(statement(backend, sql, bindings)?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(if insert {
        result.last_insert_id()
    } else {
        result.rows_affected()
    })
}

type BlockingQuery<T> = fn(&str, &[Value]) -> Result<T, String>;

async fn legacy<T: Send + 'static>(
    sql: &str,
    bindings: &[Value],
    operation: BlockingQuery<T>,
) -> Result<T, String> {
    if db_manager::has_thread_transaction() {
        return Err("Async queries cannot use a thread-local sync transaction; use DB::transaction inside a database scope".into());
    }
    let sql = sql.to_string();
    let bindings = bindings.to_vec();
    tokio::task::spawn_blocking(move || operation(&sql, &bindings))
        .await
        .map_err(|e| format!("Database worker failed: {e}"))?
}

pub async fn select(sql: &str, bindings: &[Value]) -> Result<Vec<Value>, String> {
    match DATABASE.try_with(Clone::clone).ok() {
        Some(Executor::Pool(conn)) => select_on(conn.as_ref(), sql, bindings).await,
        Some(Executor::Transaction(tx)) => select_on(tx.as_ref(), sql, bindings).await,
        None => legacy(sql, bindings, db_manager::global_select).await,
    }
}

pub async fn insert(sql: &str, bindings: &[Value]) -> Result<u64, String> {
    execute(sql, bindings, true).await
}

pub async fn execute(sql: &str, bindings: &[Value], insert: bool) -> Result<u64, String> {
    match DATABASE.try_with(Clone::clone).ok() {
        Some(Executor::Pool(conn)) => execute_on(conn.as_ref(), sql, bindings, insert).await,
        Some(Executor::Transaction(tx)) => execute_on(tx.as_ref(), sql, bindings, insert).await,
        None => {
            legacy(
                sql,
                bindings,
                if insert {
                    db_manager::global_insert
                } else {
                    db_manager::global_update
                },
            )
            .await
        }
    }
}

pub async fn update(sql: &str, bindings: &[Value]) -> Result<u64, String> {
    execute(sql, bindings, false).await
}
pub async fn delete(sql: &str, bindings: &[Value]) -> Result<u64, String> {
    execute(sql, bindings, false).await
}

pub async fn execute_statement(sql: &str) -> Result<(), String> {
    if DATABASE.try_with(|_| ()).is_ok() {
        execute(sql, &[], false).await.map(|_| ())
    } else {
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || db_manager::global_statement(&sql))
            .await
            .map_err(|e| format!("Database worker failed: {e}"))?
            .map(|_| ())
    }
}
