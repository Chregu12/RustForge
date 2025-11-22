// Database Demo - Complete CRUD Example with rf-orm
//
// Demonstrates:
// - Database connection and setup
// - Entity definition with SeaORM
// - CRUD operations (Create, Read, Update, Delete)
// - Query filtering and ordering
// - Soft delete functionality
// - Transaction support

mod entities;

use chrono::Utc;
use entities::user::{self, Entity as User};
use rf_orm::prelude::*;
use sea_orm::{ActiveValue, ConnectionTrait, DbBackend, Schema, Statement};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Database Demo - rf-orm with SeaORM");
    info!("================================================\n");

    // Step 1: Connect to database (SQLite in-memory for demo)
    info!("📦 Step 1: Connecting to database...");
    let config = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
        min_connections: 1,
        ..Default::default()
    };

    let db = DatabaseManager::connect(config).await?;
    info!("✅ Connected successfully\n");

    // Step 2: Create table
    info!("🔨 Step 2: Creating users table...");
    create_table(db.connection()).await?;
    info!("✅ Table created\n");

    // Step 3: Insert users
    info!("➕ Step 3: Inserting users...");
    let alice = create_user(db.connection(), "alice@example.com", "Alice Smith").await?;
    info!("   Created user: {} (id: {})", alice.display(), alice.id);

    let bob = create_user(db.connection(), "bob@example.com", "Bob Jones").await?;
    info!("   Created user: {} (id: {})", bob.display(), bob.id);

    let charlie = create_user(db.connection(), "charlie@example.com", "Charlie Brown").await?;
    info!(
        "   Created user: {} (id: {})\n",
        charlie.display(),
        charlie.id
    );

    // Step 4: Query all users
    info!("🔍 Step 4: Querying all users...");
    let users = User::find().all(db.connection()).await?;
    info!("   Found {} users:", users.len());
    for user in &users {
        info!("   - {}", user.display());
    }
    info!("");

    // Step 5: Find by ID
    info!("🔎 Step 5: Finding user by ID...");
    let found = User::find_by_id(alice.id)
        .one(db.connection())
        .await?
        .expect("User not found");
    info!("   Found: {}\n", found.display());

    // Step 6: Update user
    info!("✏️  Step 6: Updating user name...");
    let mut alice_active: user::ActiveModel = alice.clone().into();
    alice_active.name = Set("Alice Johnson".to_string());
    alice_active.updated_at = Set(Utc::now());
    let alice = alice_active.update(db.connection()).await?;
    info!("   Updated to: {}\n", alice.display());

    // Step 7: Query with filter
    info!("🔍 Step 7: Querying users with email filter...");
    let filtered = User::find()
        .filter(user::Column::Email.contains("alice"))
        .all(db.connection())
        .await?;
    info!("   Found {} user(s) matching filter:", filtered.len());
    for user in &filtered {
        info!("   - {}", user.display());
    }
    info!("");

    // Step 8: Soft delete
    info!("🗑️  Step 8: Soft deleting user (Bob)...");
    let mut bob_active: user::ActiveModel = bob.clone().into();
    bob_active.soft_delete();
    let bob = bob_active.update(db.connection()).await?;
    info!(
        "   Soft deleted: {} (deleted_at: {:?})\n",
        bob.display(),
        bob.deleted_at
    );

    // Step 9: Query excluding soft-deleted
    info!("🔍 Step 9: Querying active users (excluding soft-deleted)...");
    let active_users = User::find()
        .filter(user::Column::DeletedAt.is_null())
        .all(db.connection())
        .await?;
    info!("   Found {} active user(s):", active_users.len());
    for user in &active_users {
        info!("   - {}", user.display());
    }
    info!("");

    // Step 10: Query only soft-deleted
    info!("🔍 Step 10: Querying soft-deleted users...");
    let deleted_users = User::find()
        .filter(user::Column::DeletedAt.is_not_null())
        .all(db.connection())
        .await?;
    info!("   Found {} soft-deleted user(s):", deleted_users.len());
    for user in &deleted_users {
        info!(
            "   - {} (deleted_at: {:?})",
            user.display(),
            user.deleted_at
        );
    }
    info!("");

    // Step 11: Restore soft-deleted user
    info!("♻️  Step 11: Restoring soft-deleted user...");
    let mut bob_active: user::ActiveModel = bob.into();
    bob_active.restore();
    let bob = bob_active.update(db.connection()).await?;
    info!(
        "   Restored: {} (deleted_at: {:?})\n",
        bob.display(),
        bob.deleted_at
    );

    // Step 12: Order by created_at
    info!("🔍 Step 12: Querying users ordered by creation date...");
    let ordered = User::find()
        .order_by_desc(user::Column::CreatedAt)
        .all(db.connection())
        .await?;
    info!("   Users ordered by created_at (newest first):");
    for user in &ordered {
        info!("   - {}", user.display());
    }
    info!("");

    // Step 13: Count users
    info!("🔢 Step 13: Counting total users...");
    let count = User::find().count(db.connection()).await?;
    info!("   Total users: {}\n", count);

    // Step 14: Hard delete
    info!("🗑️  Step 14: Hard deleting user (Charlie)...");
    let result = User::delete_by_id(charlie.id).exec(db.connection()).await?;
    info!("   Deleted {} row(s)\n", result.rows_affected);

    // Step 15: Final count
    info!("🔢 Step 15: Final user count...");
    let final_count = User::find().count(db.connection()).await?;
    info!("   Remaining users: {}\n", final_count);

    // Step 16: List remaining users
    info!("📋 Step 16: Final user list...");
    let final_users = User::find().all(db.connection()).await?;
    for user in &final_users {
        info!("   - {}", user.display());
    }
    info!("");

    info!("✅ Demo completed successfully!");
    info!("================================================");

    // Close database connection
    db.close().await?;

    Ok(())
}

/// Create users table
async fn create_table(db: &DatabaseConnection) -> anyhow::Result<()> {
    let schema = Schema::new(DbBackend::Sqlite);
    let stmt = schema.create_table_from_entity(User);

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        stmt.to_string(sea_orm::sea_query::SqliteQueryBuilder),
    ))
    .await?;

    Ok(())
}

/// Create a new user
async fn create_user(
    db: &DatabaseConnection,
    email: &str,
    name: &str,
) -> anyhow::Result<user::Model> {
    let now = Utc::now();

    let user = user::ActiveModel {
        id: ActiveValue::NotSet,
        email: Set(email.to_string()),
        name: Set(name.to_string()),
        password_hash: Set("$2b$12$dummy_hash_for_demo".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    let result = User::insert(user).exec(db).await?;

    let inserted = User::find_by_id(result.last_insert_id)
        .one(db)
        .await?
        .expect("Failed to find inserted user");

    Ok(inserted)
}
