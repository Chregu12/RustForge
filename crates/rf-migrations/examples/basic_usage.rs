//! # rf-migrations — basic usage example
//!
//! Demonstrates how to define migrations and use `MigrationManager` to
//! run, check status, rollback, and refresh them.
//!
//! Run with:
//!   cargo run --example basic_usage -p rf-migrations

use async_trait::async_trait;
use rf_migrations::{Migration, MigrationManager};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

// ---------------------------------------------------------------------------
// Migration 1: create users table
// ---------------------------------------------------------------------------

pub struct CreateUsersTable;

#[async_trait]
impl Migration for CreateUsersTable {
    fn name(&self) -> &str {
        "2024_01_01_000001_create_users_table"
    }

    async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "CREATE TABLE IF NOT EXISTS users (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                email      TEXT NOT NULL UNIQUE,
                created_at TEXT DEFAULT (datetime('now'))
            )",
        ))
        .await?;
        println!("  ↑  users table created");
        Ok(())
    }

    async fn down(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "DROP TABLE IF EXISTS users",
        ))
        .await?;
        println!("  ↓  users table dropped");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Migration 2: create posts table
// ---------------------------------------------------------------------------

pub struct CreatePostsTable;

#[async_trait]
impl Migration for CreatePostsTable {
    fn name(&self) -> &str {
        "2024_01_15_000001_create_posts_table"
    }

    async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "CREATE TABLE IF NOT EXISTS posts (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id    INTEGER NOT NULL,
                title      TEXT NOT NULL,
                body       TEXT NOT NULL,
                published  INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            )",
        ))
        .await?;
        println!("  ↑  posts table created");
        Ok(())
    }

    async fn down(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "DROP TABLE IF EXISTS posts",
        ))
        .await?;
        println!("  ↓  posts table dropped");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Migration 3: add bio column to users
// ---------------------------------------------------------------------------

pub struct AddBioToUsers;

#[async_trait]
impl Migration for AddBioToUsers {
    fn name(&self) -> &str {
        "2024_02_01_000001_add_bio_to_users"
    }

    async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE users ADD COLUMN bio TEXT",
        ))
        .await?;
        println!("  ↑  bio column added to users");
        Ok(())
    }

    async fn down(&self, _db: &DatabaseConnection) -> anyhow::Result<()> {
        // SQLite does not support DROP COLUMN in older versions; we just log.
        println!("  ↓  bio column removal skipped (SQLite limitation)");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = Database::connect("sqlite::memory:").await?;

    let mut manager = MigrationManager::new(db);
    manager.register_many(vec![
        Box::new(CreateUsersTable),
        Box::new(CreatePostsTable),
        Box::new(AddBioToUsers),
    ]);

    // --- Run all pending migrations ----------------------------------------
    println!("\n=== run() ===");
    let result = manager.run().await?;
    println!("{}", result);

    // --- Status -----------------------------------------------------------
    println!("\n=== status() ===");
    for s in manager.status().await? {
        println!("  {}", s);
    }

    // --- Rollback last batch (AddBioToUsers) ------------------------------
    println!("\n=== rollback(1) ===");
    let rb = manager.rollback(1).await?;
    println!("{}", rb);
    for name in &rb.applied {
        println!("  rolled back: {}", name);
    }

    // --- Status after rollback -------------------------------------------
    println!("\n=== status() after rollback ===");
    for s in manager.status().await? {
        println!("  {}", s);
    }

    // --- Fresh (reset + run) ---------------------------------------------
    println!("\n=== fresh() ===");
    let fresh = manager.fresh().await?;
    println!("{}", fresh);

    // --- Final status ----------------------------------------------------
    println!("\n=== final status() ===");
    for s in manager.status().await? {
        println!("  {}", s);
    }

    Ok(())
}
