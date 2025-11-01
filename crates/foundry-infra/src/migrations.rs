use crate::config::DatabaseConfig;
use crate::db;
use anyhow::Context;
use async_trait::async_trait;
use foundry_plugins::{CommandError, MigrationPort, MigrationRun};
use sea_orm::sea_query::{Alias, ColumnDef, Expr, PostgresQueryBuilder, SqliteQueryBuilder, Table};
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectionTrait, Statement, TransactionTrait, Value as SeaOrmValue};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MIGRATION_TABLE: &str = "foundry_migrations";
const MIGRATION_TABLE_NAME_COLUMN: &str = "name";
const MIGRATION_TABLE_APPLIED_AT_COLUMN: &str = "applied_at";

#[derive(Clone)]
pub struct SeaOrmMigrationService {
    root: PathBuf,
}

impl SeaOrmMigrationService {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    fn discover(&self) -> Result<Vec<MigrationFile>, CommandError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut migrations = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|err| CommandError::Other(err.into()))? {
            let entry = entry.map_err(|err| CommandError::Other(err.into()))?;
            if !entry
                .file_type()
                .map_err(|err| CommandError::Other(err.into()))?
                .is_dir()
            {
                continue;
            }

            let name = entry.file_name().into_string().map_err(|_| {
                CommandError::Message("Migrationspfad enthält ungültige UTF-8-Zeichen".into())
            })?;

            let dir = entry.path();
            let up = dir.join("up.sql");
            let down = dir.join("down.sql");

            migrations.push(MigrationFile { name, up, down });
        }

        migrations.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(migrations)
    }

    async fn ensure_meta_table(&self, conn: &DatabaseConnection) -> Result<(), CommandError> {
        let backend = conn.get_database_backend();
        let mut create_table_stmt = Table::create();
        create_table_stmt
            .table(Alias::new(MIGRATION_TABLE))
            .if_not_exists()
            .col(
                ColumnDef::new(Alias::new(MIGRATION_TABLE_NAME_COLUMN))
                    .string()
                    .not_null()
                    .primary_key(),
            );

        match backend {
            sea_orm::DatabaseBackend::Postgres => {
                create_table_stmt.col(
                    ColumnDef::new(Alias::new(MIGRATION_TABLE_APPLIED_AT_COLUMN))
                        .timestamp_with_time_zone()
                        .default(Expr::current_timestamp())
                        .not_null(),
                );
            }
            _ => {
                create_table_stmt.col(
                    ColumnDef::new(Alias::new(MIGRATION_TABLE_APPLIED_AT_COLUMN))
                        .timestamp()
                        .default(Expr::current_timestamp())
                        .not_null(),
                );
            }
        }

        let sql = match backend {
            sea_orm::DatabaseBackend::Postgres => create_table_stmt.build(PostgresQueryBuilder),
            sea_orm::DatabaseBackend::Sqlite => create_table_stmt.build(SqliteQueryBuilder),
            _ => {
                return Err(CommandError::Message(
                    "Unsupported database backend for schema building".into(),
                ))
            }
        };

        conn.execute(Statement::from_string(backend, sql))
            .await
            .map_err(|err| CommandError::Other(err.into()))?;
        Ok(())
    }

    async fn load_applied(&self, conn: &DatabaseConnection) -> Result<Vec<String>, CommandError> {
        let backend = conn.get_database_backend();
        let query = format!("SELECT name FROM {MIGRATION_TABLE} ORDER BY applied_at ASC, name ASC");
        let rows = conn
            .query_all(Statement::from_string(backend, query))
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        let mut applied = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row
                .try_get("", "name")
                .map_err(|err| CommandError::Other(err.into()))?;
            applied.push(name);
        }
        Ok(applied)
    }

    fn pending_from(
        &self,
        available: Vec<MigrationFile>,
        applied: &HashSet<String>,
    ) -> (Vec<MigrationFile>, Vec<String>) {
        let mut pending = Vec::new();
        let mut skipped = Vec::new();

        for file in available {
            if applied.contains(&file.name) {
                skipped.push(file.name.clone());
            } else {
                pending.push(file);
            }
        }

        (pending, skipped)
    }

    fn find_migration(
        &self,
        available: &[MigrationFile],
        name: &str,
    ) -> Result<MigrationFile, CommandError> {
        available
            .iter()
            .find(|migration| migration.name == name)
            .cloned()
            .ok_or_else(|| {
                CommandError::Message(format!(
                    "Migration `{name}` konnte im Verzeichnis {} nicht gefunden werden",
                    self.root.display()
                ))
            })
    }
}

impl Default for SeaOrmMigrationService {
    fn default() -> Self {
        Self {
            root: PathBuf::from("migrations"),
        }
    }
}

#[async_trait]
impl MigrationPort for SeaOrmMigrationService {
    async fn apply(&self, config: &Value, dry_run: bool) -> Result<MigrationRun, CommandError> {
        let db_config = DatabaseConfig::try_from(config.clone())
            .map_err(|e| CommandError::Message(e.to_string()))?;
        let conn = db::connect(&db_config)
            .await
            .map_err(|e| CommandError::Message(e.to_string()))?;
        self.ensure_meta_table(&conn).await?;

        let available = self.discover()?;
        let applied = self.load_applied(&conn).await?;
        let applied_set: HashSet<_> = applied.iter().cloned().collect();

        let (pending, skipped) = self.pending_from(available, &applied_set);
        if dry_run {
            return Ok(MigrationRun {
                pending: pending.iter().map(|m| m.name.clone()).collect(),
                skipped,
                ..Default::default()
            });
        }

        if pending.is_empty() {
            return Ok(MigrationRun {
                skipped,
                ..Default::default()
            });
        }

        let backend = conn.get_database_backend();
        let mut applied_names = Vec::new();
        let txn = conn
            .begin()
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        for migration in pending {
            let sql = migration.read_up()?;
            txn.execute(Statement::from_string(backend, sql))
                .await
                .map_err(|err| CommandError::Other(err.into()))?;

            txn.execute(Statement::from_sql_and_values(
                backend,
                format!("INSERT INTO {MIGRATION_TABLE} (name) VALUES (?)"),
                [SeaOrmValue::String(Some(Box::new(migration.name.clone())))],
            ))
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

            applied_names.push(migration.name);
        }

        txn.commit()
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        Ok(MigrationRun {
            applied: applied_names,
            skipped,
            ..Default::default()
        })
    }

    async fn rollback(&self, config: &Value, dry_run: bool) -> Result<MigrationRun, CommandError> {
        let db_config = DatabaseConfig::try_from(config.clone())
            .map_err(|e| CommandError::Message(e.to_string()))?;
        let conn = db::connect(&db_config)
            .await
            .map_err(|e| CommandError::Message(e.to_string()))?;
        self.ensure_meta_table(&conn).await?;

        let available = self.discover()?;
        let applied = self.load_applied(&conn).await?;
        let Some(target) = applied.last().cloned() else {
            return Ok(MigrationRun {
                skipped: applied,
                ..Default::default()
            });
        };

        let migration = self.find_migration(&available, &target)?;

        if dry_run {
            return Ok(MigrationRun {
                pending: vec![migration.name.clone()],
                skipped: applied,
                ..Default::default()
            });
        }

        let backend = conn.get_database_backend();
        let txn = conn
            .begin()
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        let sql = migration.read_down()?;
        txn.execute(Statement::from_string(backend, sql))
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        txn.execute(Statement::from_sql_and_values(
            backend,
            format!("DELETE FROM {MIGRATION_TABLE} WHERE name = ?"),
            [SeaOrmValue::String(Some(Box::new(migration.name.clone())))],
        ))
        .await
        .map_err(|err| CommandError::Other(err.into()))?;

        txn.commit()
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        Ok(MigrationRun {
            rolled_back: vec![migration.name],
            skipped: applied.into_iter().filter(|name| name != &target).collect(),
            ..Default::default()
        })
    }
}

#[derive(Clone)]
pub struct MigrationFile {
    name: String,
    up: PathBuf,
    down: PathBuf,
}

impl MigrationFile {
    pub fn read_up(&self) -> Result<String, CommandError> {
        Self::read_sql(&self.up)
    }

    pub fn read_down(&self) -> Result<String, CommandError> {
        Self::read_sql(&self.down)
    }

    fn read_sql(path: &Path) -> Result<String, CommandError> {
        if !path.exists() {
            return Err(CommandError::Message(format!(
                "Migration-Datei {} nicht gefunden",
                path.display()
            )));
        }

        fs::read_to_string(path)
            .with_context(|| format!("Konnte SQL-Datei {} nicht lesen", path.display()))
            .map_err(CommandError::Other)
    }
}
