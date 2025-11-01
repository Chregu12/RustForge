use foundry_infra::{
    config::{DatabaseConfig, DatabaseDriver},
    db::connect,
    SeaOrmMigrationService, SeaOrmSeedService,
};
use foundry_plugins::{MigrationPort, SeedPort};
use sea_orm::{ConnectionTrait, DatabaseBackend};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::sleep;

const TEST_DB_NAME: &str = "foundry_test_db";
const TEST_USER: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_PORT: &str = "5433"; // Use a different port to avoid conflicts

async fn setup_postgres_container() -> String {
    // Ensure no old container is running
    Command::new("docker")
        .args(&["stop", "foundry-test-postgres"])
        .output()
        .ok();
    Command::new("docker")
        .args(&["rm", "foundry-test-postgres"])
        .output()
        .ok();

    // Run PostgreSQL container
    let output = Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name",
            "foundry-test-postgres",
            "-e",
            &format!("POSTGRES_USER={}", TEST_USER),
            "-e",
            &format!("POSTGRES_PASSWORD={}", TEST_PASSWORD),
            "-e",
            &format!("POSTGRES_DB={}", TEST_DB_NAME),
            "-p",
            &format!("{}:5432", TEST_PORT),
            "-d",
            "postgres:16",
        ])
        .output()
        .expect("Failed to start postgres container");

    if !output.status.success() {
        panic!("Docker command failed: {:?}", output);
    }

    let database_url = format!(
        "postgres://{}:{}@localhost:{}/{}",
        TEST_USER, TEST_PASSWORD, TEST_PORT, TEST_DB_NAME
    );

    // Wait for PostgreSQL to be ready
    for _ in 0..30 {
        // Try for 30 seconds
        if let Ok(conn) = connect(&DatabaseConfig {
            driver: DatabaseDriver::Postgres,
            url: database_url.clone(),
        })
        .await
        {
            conn.close().await.ok();
            println!("PostgreSQL container is ready.");
            return database_url;
        }
        sleep(Duration::from_secs(1)).await;
    }

    panic!("PostgreSQL container did not become ready in time.");
}

async fn teardown_postgres_container() {
    Command::new("docker")
        .args(&["stop", "foundry-test-postgres"])
        .output()
        .expect("Failed to stop postgres container");
}

#[tokio::test]
async fn test_postgres_init_migrate_seed() {
    let database_url = setup_postgres_container().await;

    let temp_dir = tempdir().expect("Failed to create temp dir");
    let migrations_path = temp_dir.path().join("migrations");
    let seeds_path = temp_dir.path().join("seeds");

    fs::create_dir(&migrations_path).expect("Failed to create migrations dir");
    fs::create_dir(&seeds_path).expect("Failed to create seeds dir");

    // Create a dummy migration file
    let migration_dir = migrations_path.join("20251027000000_test_migration");
    fs::create_dir(&migration_dir).expect("Failed to create migration dir");
    fs::write(
        migration_dir.join("up.sql"),
        "CREATE TABLE test_table (id SERIAL PRIMARY KEY);",
    )
    .expect("Failed to write up.sql");
    fs::write(migration_dir.join("down.sql"), "DROP TABLE test_table;")
        .expect("Failed to write down.sql");

    // Create a dummy seed file
    fs::write(
        seeds_path.join("20251027000000_test_seed.sql"),
        "INSERT INTO test_table (id) VALUES (1);",
    )
    .expect("Failed to write seed.sql");

    let config = Value::Object(
        serde_json::from_str(&format!(
            r#"{{"DB_CONNECTION": "postgres", "DATABASE_URL": "{}"}}"#,
            database_url
        ))
        .unwrap(),
    );

    let migrations = SeaOrmMigrationService::new(migrations_path);
    let seeds = SeaOrmSeedService::new(seeds_path);

    // Run migrations
    let migration_run = migrations
        .apply(&config, false)
        .await
        .expect("Migrations failed");
    assert!(!migration_run.applied.is_empty());

    // Run seeds
    let seed_run = seeds.run(&config, false).await.expect("Seeds failed");
    assert!(!seed_run.executed.is_empty());

    // Verify by connecting and checking tables
    let conn = connect(&DatabaseConfig {
        driver: DatabaseDriver::Postgres,
        url: database_url.clone(),
    })
    .await
    .expect("Failed to connect to DB for verification");
    let rows = conn
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT name FROM foundry_migrations",
        ))
        .await
        .expect("Failed to query migrations table");
    assert!(!rows.is_empty());

    let rows = conn
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT name FROM foundry_seeds",
        ))
        .await
        .expect("Failed to query seeds table");
    assert!(!rows.is_empty());

    // Verify the test_table and its content
    let rows = conn
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id FROM test_table",
        ))
        .await
        .expect("Failed to query test_table");
    assert_eq!(rows.len(), 1);
    let id: i32 = rows[0]
        .try_get("", "id")
        .expect("Failed to get id from test_table");
    assert_eq!(id, 1);

    teardown_postgres_container().await;
}
