//! Notification entity for database storage

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub r#type: String,
    pub notifiable_id: i64,
    pub notifiable_type: String,
    pub data: Json,
    pub read_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Portable migration SQL for creating the notifications table.
///
/// Uses `TEXT` for the `data` column so the migration runs on both SQLite
/// (the framework default) and PostgreSQL without modification.  SQLite maps
/// any unknown type name such as `JSONB` to NUMERIC affinity, which is wrong
/// for JSON string storage and can corrupt bare-number or boolean JSON values.
/// `TEXT` gives the correct TEXT affinity on SQLite and is fully accepted by
/// PostgreSQL as well (JSON is serialised as a text string either way).
///
/// If you are targeting PostgreSQL exclusively and want the native binary-JSON
/// type with its indexing / operator benefits, use [`MIGRATION_SQL_POSTGRES`]
/// instead.
pub const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    type VARCHAR NOT NULL,
    notifiable_id BIGINT NOT NULL,
    notifiable_type VARCHAR NOT NULL,
    data TEXT NOT NULL,
    read_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_notifiable ON notifications(notifiable_id, notifiable_type);
CREATE INDEX idx_notifications_read_at ON notifications(read_at);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);
"#;

/// PostgreSQL-specific migration SQL that uses the native `JSONB` column type
/// for the `data` field.  Provides binary-JSON storage with GIN indexing
/// support and PostgreSQL JSON operators.
///
/// Do **not** run this on SQLite — `JSONB` is silently accepted there but
/// gives NUMERIC affinity instead of the required TEXT affinity.  Use the
/// portable [`MIGRATION_SQL`] on SQLite or when targeting multiple backends.
pub const MIGRATION_SQL_POSTGRES: &str = r#"
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    type VARCHAR NOT NULL,
    notifiable_id BIGINT NOT NULL,
    notifiable_type VARCHAR NOT NULL,
    data JSONB NOT NULL,
    read_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_notifiable ON notifications(notifiable_id, notifiable_type);
CREATE INDEX idx_notifications_read_at ON notifications(read_at);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_model() {
        let notification = Model {
            id: Uuid::new_v4(),
            r#type: "TestNotification".to_string(),
            notifiable_id: 1,
            notifiable_type: "User".to_string(),
            data: serde_json::json!({"key": "value"}),
            read_at: None,
            created_at: DateTimeUtc::from(chrono::Utc::now()),
        };

        assert_eq!(notification.notifiable_id, 1);
        assert_eq!(notification.notifiable_type, "User");
    }

    /// Verify MIGRATION_SQL works on SQLite and stores the JSON payload with
    /// correct TEXT affinity (JSONB is Postgres-only and maps to NUMERIC affinity
    /// on SQLite, which is wrong for JSON string storage).
    ///
    /// This test FAILS before the fix (col type = "JSONB") and PASSES after
    /// ("TEXT").  It also exercises a full JSON round-trip via raw SQL so that
    /// data-integrity is proven end-to-end on the framework's default DB backend.
    #[tokio::test]
    async fn test_migration_sql_sqlite_text_affinity_and_roundtrip() {
        use sea_orm::{ConnectionTrait, Database, DbBackend, Statement, Value};

        let db = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite must open");

        // Execute each statement individually — SQLite's C API is single-statement.
        for raw in MIGRATION_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            db.execute(Statement::from_string(DbBackend::Sqlite, raw.to_string()))
                .await
                .expect("every MIGRATION_SQL statement must succeed on SQLite");
        }

        // 1. Assert the `data` column's declared type via PRAGMA.
        //    Before fix: "JSONB" (NUMERIC affinity — wrong for JSON).
        //    After fix:  "TEXT" (TEXT affinity — correct, portable).
        let pragma_rows = db
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA table_info(notifications)".to_string(),
            ))
            .await
            .expect("PRAGMA table_info must succeed");

        let data_row = pragma_rows
            .iter()
            .find(|r| r.try_get::<String>("", "name").unwrap_or_default() == "data")
            .expect("PRAGMA must include a 'data' column");

        let col_type: String = data_row
            .try_get("", "type")
            .expect("PRAGMA 'type' column must be readable");

        assert_eq!(
            col_type, "TEXT",
            "MIGRATION_SQL data column must declare TEXT (not JSONB) so SQLite \
             gives it TEXT affinity; JSONB is Postgres-only and silently causes \
             NUMERIC affinity on SQLite. Got: {col_type}"
        );

        // 2. JSON payload round-trip: write a structured payload and read it
        //    back as the same string.  With correct TEXT affinity the value is
        //    stored and returned verbatim; with NUMERIC affinity a bare-number
        //    JSON value such as `42` would be coerced to an integer at rest.
        let payload = r#"{"title":"Invoice Paid","message":"Invoice #7 paid","data":{"amount":99.99}}"#;
        db.execute(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            r#"INSERT INTO notifications
                   (id, type, notifiable_id, notifiable_type, data, read_at, created_at)
               VALUES
                   (lower(hex(randomblob(16))), 'Test', 1, 'User', ?, NULL, CURRENT_TIMESTAMP)"#,
            [Value::String(Some(Box::new(payload.to_string())))],
        ))
        .await
        .expect("INSERT must succeed");

        let result_row = db
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT data FROM notifications WHERE notifiable_id = 1".to_string(),
            ))
            .await
            .expect("SELECT must succeed")
            .expect("one row must exist");

        let stored: String = result_row
            .try_get("", "data")
            .expect("data column must be readable as String");

        assert_eq!(
            stored, payload,
            "JSON payload must round-trip through SQLite unchanged"
        );
    }
}
