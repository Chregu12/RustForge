use crate::config::DatabaseConfig;
use crate::db;
use anyhow::Context;
use async_trait::async_trait;
use foundry_plugins::{CommandError, SeedPort, SeedRun};
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectionTrait, Statement, TransactionTrait, Value as SeaOrmValue};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const SEED_TABLE: &str = "foundry_seeds";

#[derive(Clone)]
pub struct SeaOrmSeedService {
    root: PathBuf,
}

impl SeaOrmSeedService {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    fn discover(&self) -> Result<Vec<SeedFile>, CommandError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut seeds = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|err| CommandError::Other(err.into()))? {
            let entry = entry.map_err(|err| CommandError::Other(err.into()))?;
            if !entry
                .file_type()
                .map_err(|err| CommandError::Other(err.into()))?
                .is_file()
            {
                continue;
            }

            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    CommandError::Message("Seed-Datei enthält ungültige UTF-8-Zeichen".into())
                })?
                .to_string();

            seeds.push(SeedFile { name, path });
        }

        seeds.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(seeds)
    }

    async fn ensure_meta_table(&self, conn: &DatabaseConnection) -> Result<(), CommandError> {
        let backend = conn.get_database_backend();
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {SEED_TABLE} (\
                name VARCHAR(255) PRIMARY KEY,\
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\
            )"
        );

        conn.execute(Statement::from_string(backend, create_sql))
            .await
            .map_err(|err| CommandError::Other(err.into()))?;
        Ok(())
    }

    async fn load_applied(&self, conn: &DatabaseConnection) -> Result<Vec<String>, CommandError> {
        let backend = conn.get_database_backend();
        let query = format!("SELECT name FROM {SEED_TABLE} ORDER BY applied_at ASC, name ASC");
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
        available: Vec<SeedFile>,
        applied: &HashSet<String>,
    ) -> (Vec<SeedFile>, Vec<String>) {
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
}

impl Default for SeaOrmSeedService {
    fn default() -> Self {
        Self {
            root: PathBuf::from("seeds"),
        }
    }
}

#[async_trait]
impl SeedPort for SeaOrmSeedService {
    async fn run(&self, config: &Value, dry_run: bool) -> Result<SeedRun, CommandError> {
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
            return Ok(SeedRun {
                pending: pending.iter().map(|seed| seed.name.clone()).collect(),
                skipped,
                ..Default::default()
            });
        }

        if pending.is_empty() {
            return Ok(SeedRun {
                skipped,
                ..Default::default()
            });
        }

        let backend = conn.get_database_backend();
        let txn = conn
            .begin()
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        let mut executed = Vec::new();
        for seed in pending {
            let sql = seed.read_sql()?;
            txn.execute(Statement::from_string(backend, sql))
                .await
                .map_err(|err| CommandError::Other(err.into()))?;

            txn.execute(Statement::from_sql_and_values(
                backend,
                format!("INSERT INTO {SEED_TABLE} (name) VALUES (?)"),
                [SeaOrmValue::String(Some(Box::new(seed.name.clone())))],
            ))
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

            executed.push(seed.name);
        }

        txn.commit()
            .await
            .map_err(|err| CommandError::Other(err.into()))?;

        Ok(SeedRun {
            executed,
            skipped,
            ..Default::default()
        })
    }
}

#[derive(Clone)]
struct SeedFile {
    name: String,
    path: PathBuf,
}

impl SeedFile {
    fn read_sql(&self) -> Result<String, CommandError> {
        if !self.path.exists() {
            return Err(CommandError::Message(format!(
                "Seed-Datei {} nicht gefunden",
                self.path.display()
            )));
        }

        fs::read_to_string(&self.path)
            .with_context(|| format!("Konnte Seed-Datei {} nicht lesen", self.path.display()))
            .map_err(CommandError::Other)
    }
}
