//! Execute project migration and seed scripts against the application's database.

use anyhow::{bail, ensure, Context, Result};
use rf_orm::{
    sea_orm::{DbBackend, Statement, TransactionTrait},
    BatchResult, ConnectionTrait, DatabaseConfig, DatabaseManager, MigrationError, Migrator,
};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};

use super::ensure_forge_project;

const MIGRATION_DIRS: &[&str] = &["database/migrations", "src/migrations", "migrations"];

pub(super) fn migration_directory() -> &'static str {
    MIGRATION_DIRS
        .iter()
        .copied()
        .find(|dir| Path::new(dir).is_dir())
        .unwrap_or("database/migrations")
}

async fn database(force: bool, mutates: bool) -> Result<DatabaseManager> {
    ensure_forge_project()?;
    match dotenvy::from_path(".env") {
        Ok(()) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Load project .env"),
    }
    if mutates && env::var("APP_ENV").is_ok_and(|env| env.eq_ignore_ascii_case("production")) {
        ensure!(force, "Use --force to change a production database");
    }
    let url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://database/database.sqlite?mode=rwc".into());
    if url == "sqlite://database/database.sqlite?mode=rwc" {
        fs::create_dir_all("database")?;
    }
    DatabaseManager::connect(DatabaseConfig {
        url,
        ..Default::default()
    })
    .await
    .context("Connect to the project database")
}

fn load_migrations(db: &DatabaseManager) -> Result<Migrator> {
    let mut files = BTreeMap::new();
    for root in MIGRATION_DIRS {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| format!("Read {root}")),
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir()
                || !["up.sql", "down.sql", "up.postgres.sql", "down.postgres.sql"]
                    .iter()
                    .any(|file| path.join(file).is_file())
            {
                continue;
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Invalid migration name in {root}"))?;
            ensure!(
                files.insert(name.clone(), path).is_none(),
                "Duplicate migration '{name}'"
            );
        }
    }
    let mut migrator = Migrator::from_manager(db);
    let postgres = db.connection().get_database_backend() == DbBackend::Postgres;
    for (name, path) in files {
        let read = |direction: &str| {
            let postgres_file = path.join(format!("{direction}.postgres.sql"));
            let file = if postgres && postgres_file.is_file() {
                postgres_file
            } else {
                path.join(format!("{direction}.sql"))
            };
            fs::read_to_string(&file).with_context(|| format!("Read {}", file.display()))
        };
        migrator.add_sql_migration(name, read("up")?, read("down")?);
    }
    Ok(migrator)
}

fn report(result: BatchResult, action: &str) -> Result<()> {
    for name in &result.successful {
        if result.batch == 0 {
            println!("{action} {name}");
        } else {
            println!("{action} {name} (batch {})", result.batch);
        }
    }
    if let Some((name, error)) = result.failed.first() {
        bail!("{name}: {error}");
    }
    if result.migrations_run == 0 {
        println!("Nothing to migrate.");
    }
    Ok(())
}

pub async fn run(force: bool, step: bool) -> Result<()> {
    let db = database(force, true).await?;
    let migrator = load_migrations(&db)?;
    if step {
        let result = migrator.run_stepwise().await?;
        for (offset, name) in result.successful.iter().enumerate() {
            println!("Migrated {name} (batch {})", result.batch + offset);
        }
        report(
            BatchResult {
                successful: Vec::new(),
                ..result
            },
            "Migrated",
        )
    } else {
        report(migrator.run().await?, "Migrated")
    }
}

pub async fn rollback(steps: Option<usize>, force: bool) -> Result<()> {
    ensure!(steps != Some(0), "--step must be greater than zero");
    let db = database(force, true).await?;
    let migrator = load_migrations(&db)?;
    let result = if let Some(steps) = steps {
        migrator.rollback_latest(steps).await
    } else {
        migrator.rollback(None).await
    };
    match result {
        Ok(result) => report(result, "Rolled back"),
        Err(MigrationError::NoMigrationsToRollback) => {
            println!("Nothing to roll back.");
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

pub async fn rollback_batch(batch: usize, force: bool) -> Result<()> {
    let db = database(force, true).await?;
    report(
        load_migrations(&db)?.rollback_batch(batch).await?,
        "Rolled back",
    )
}

pub async fn reset(force: bool) -> Result<()> {
    let db = database(force, true).await?;
    report(load_migrations(&db)?.reset().await?, "Rolled back")
}

pub async fn refresh(step: Option<usize>, with_seed: bool, force: bool) -> Result<()> {
    ensure!(step != Some(0), "--step must be greater than zero");
    let db = database(force, true).await?;
    let migrator = load_migrations(&db)?;
    let seeds = if with_seed {
        load_seeds(None)?
    } else {
        Vec::new()
    };
    let rollback = match step {
        Some(step) => migrator.rollback_latest(step).await?,
        None => migrator.reset().await?,
    };
    report(rollback, "Rolled back")?;
    report(migrator.run().await?, "Migrated")?;
    if with_seed {
        run_seeds(&db, seeds).await?;
    }
    Ok(())
}

pub async fn status() -> Result<()> {
    let db = database(false, false).await?;
    let statuses = load_migrations(&db)?.status().await?;
    println!("{:<58} {:<8} Batch", "Migration", "Status");
    for migration in statuses {
        let status = if migration.executed { "Ran" } else { "Pending" };
        let batch = migration
            .batch
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into());
        println!("{:<58} {status:<8} {batch}", migration.name);
    }
    Ok(())
}

pub async fn fresh(with_seed: bool, force: bool) -> Result<()> {
    ensure!(
        force,
        "migrate:fresh drops all tables in the configured database. Pass --force to proceed"
    );
    let db = database(force, true).await?;
    let migrator = load_migrations(&db)?;
    let seeds = if with_seed {
        load_seeds(None)?
    } else {
        Vec::new()
    };
    let tx = db.connection().begin().await?;
    let backend = tx.get_database_backend();
    let tables_sql = match backend {
        DbBackend::Sqlite => {
            tx.execute_unprepared("PRAGMA defer_foreign_keys = ON")
                .await?;
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT GLOB 'sqlite_*'"
        }
        DbBackend::Postgres => {
            "SELECT tablename AS name FROM pg_tables WHERE schemaname = current_schema()"
        }
        DbBackend::MySql => bail!("migrate:fresh supports SQLite and PostgreSQL"),
    };
    let names = tx
        .query_all(Statement::from_string(backend, tables_sql))
        .await?
        .into_iter()
        .map(|row| row.try_get::<String>("", "name"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let quoted: Vec<_> = names
        .into_iter()
        .map(|name| format!("\"{}\"", name.replace('"', "\"\"")))
        .collect();
    match backend {
        DbBackend::Postgres if !quoted.is_empty() => {
            tx.execute_unprepared(&format!("DROP TABLE {}", quoted.join(", ")))
                .await?;
        }
        _ => {
            for table in quoted {
                tx.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                    .await?;
            }
        }
    }
    tx.commit().await?;
    report(migrator.run().await?, "Migrated")?;
    if with_seed {
        run_seeds(&db, seeds).await?;
    }
    Ok(())
}

fn load_seeds(class: Option<&str>) -> Result<Vec<(PathBuf, String)>> {
    let root = Path::new("database/seeders");
    let mut paths = Vec::new();
    if let Some(name) = class {
        use inflector::Inflector;
        ensure!(
            name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') && !name.is_empty(),
            "Invalid seeder name"
        );
        let name = name.to_snake_case();
        paths.push(root.join(format!("{name}.sql")));
    } else {
        for entry in fs::read_dir(root).with_context(|| format!("Read {}", root.display()))? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "sql") && path.is_file() {
                paths.push(path);
            }
        }
        paths.sort();
    }
    ensure!(
        !paths.is_empty(),
        "No SQL seeders found in {}",
        root.display()
    );
    paths
        .into_iter()
        .map(|path| {
            let sql =
                fs::read_to_string(&path).with_context(|| format!("Read {}", path.display()))?;
            Ok((path, sql))
        })
        .collect()
}

async fn run_seeds(db: &DatabaseManager, seeds: Vec<(PathBuf, String)>) -> Result<()> {
    let tx = db.connection().begin().await?;
    for (path, sql) in &seeds {
        tx.execute_unprepared(sql)
            .await
            .with_context(|| format!("Execute {}", path.display()))?;
    }
    tx.commit().await?;
    for (path, _) in seeds {
        println!("Seeded {}", path.display());
    }
    Ok(())
}

pub async fn seed(class: Option<&str>, force: bool) -> Result<()> {
    let db = database(force, true).await?;
    run_seeds(&db, load_seeds(class)?).await
}
