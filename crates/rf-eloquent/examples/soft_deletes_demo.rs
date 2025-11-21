//! # Soft Deletes Demo
//!
//! Demonstrates Laravel-style soft delete functionality:
//! - Soft delete models
//! - Restore soft-deleted models
//! - Query with/without trashed records
//! - Force delete (permanent)

use rf_eloquent::soft_deletes::*;
use chrono::Utc;
use sea_orm::{entity::prelude::*, ActiveValue, Set};

// ============================================================================
// Example Models with Soft Deletes
// ============================================================================

/// User model with soft deletes
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub deleted_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// Implement SoftDeletes trait
impl SoftDeletes for ActiveModel {
    fn soft_delete(&mut self) {
        self.deleted_at = set_deleted_at();
    }

    fn restore(&mut self) {
        self.deleted_at = clear_deleted_at();
    }

    fn is_trashed(&self) -> bool {
        matches!(
            &self.deleted_at,
            ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_))
        )
    }

    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match &self.deleted_at {
            ActiveValue::Set(dt) | ActiveValue::Unchanged(dt) => *dt,
            _ => None,
        }
    }
}

// Implement SoftDeleteEntity for query-level operations
impl_soft_delete_entity!(Entity, Column::DeletedAt);

// ============================================================================
// Example 1: Basic Soft Delete
// ============================================================================

fn example_basic_soft_delete() {
    println!("====================================");
    println!("Example 1: Basic Soft Delete");
    println!("====================================");
    println!();

    let mut user = ActiveModel {
        id: Set(1),
        name: Set("John Doe".to_string()),
        email: Set("john@example.com".to_string()),
        deleted_at: Set(None),
    };

    println!("1. Initial state:");
    println!("   Is deleted? {}", user.is_trashed());
    println!();

    // Soft delete the user
    user.soft_delete();

    println!("2. After soft delete:");
    println!("   Is deleted? {}", user.is_trashed());
    if let Some(deleted_at) = user.deleted_at() {
        println!("   Deleted at: {}", deleted_at);
    }
    println!();

    // In a real app, you'd save to database:
    // user.update(&db).await?;
}

// ============================================================================
// Example 2: Restore Soft-Deleted Record
// ============================================================================

fn example_restore() {
    println!("====================================");
    println!("Example 2: Restore Soft-Deleted Record");
    println!("====================================");
    println!();

    let mut user = ActiveModel {
        id: Set(2),
        name: Set("Jane Smith".to_string()),
        email: Set("jane@example.com".to_string()),
        deleted_at: Set(Some(Utc::now())),
    };

    println!("1. Initial state (soft-deleted):");
    println!("   Is deleted? {}", user.is_trashed());
    println!();

    // Restore the user
    user.restore();

    println!("2. After restore:");
    println!("   Is deleted? {}", user.is_trashed());
    println!("   Deleted at: {:?}", user.deleted_at());
    println!();

    // In a real app:
    // user.update(&db).await?;
}

// ============================================================================
// Example 3: Query Scopes
// ============================================================================

async fn example_query_scopes() {
    println!("====================================");
    println!("Example 3: Query Scopes");
    println!("====================================");
    println!();

    // Default: Exclude soft-deleted records
    println!("1. Default query (excludes deleted):");
    println!("   let users = Entity::find().all(&db).await?;");
    println!("   // Returns only non-deleted users");
    println!();

    // Include soft-deleted records
    println!("2. Include soft-deleted:");
    println!("   let users = Entity::with_trashed().all(&db).await?;");
    println!("   // Returns all users (including soft-deleted)");
    println!();

    // Only soft-deleted records
    println!("3. Only soft-deleted:");
    println!("   let users = Entity::only_trashed().all(&db).await?;");
    println!("   // Returns only soft-deleted users");
    println!();

    // Using SoftDeleteScope
    println!("4. Using SoftDeleteScope:");
    let scope = SoftDeleteScope::<Entity>::new()
        .with_trashed();

    println!("   Include trashed? {}", scope.include_trashed);
    println!("   Only trashed? {}", scope.only_trashed);
    println!();

    let scope2 = SoftDeleteScope::<Entity>::new()
        .only_trashed();

    println!("   Only trashed scope:");
    println!("   Include trashed? {}", scope2.include_trashed);
    println!("   Only trashed? {}", scope2.only_trashed);
    println!();
}

// ============================================================================
// Example 4: Force Delete (Permanent)
// ============================================================================

async fn example_force_delete() {
    println!("====================================");
    println!("Example 4: Force Delete (Permanent)");
    println!("====================================");
    println!();

    let user = ActiveModel {
        id: Set(3),
        name: Set("Bob Wilson".to_string()),
        email: Set("bob@example.com".to_string()),
        deleted_at: Set(Some(Utc::now())),
    };

    println!("1. User is soft-deleted: {}", user.is_trashed());
    println!();

    // Force delete (permanent)
    println!("2. Force deleting (permanent removal)...");
    println!("   user.force_delete(&db).await?;");
    println!("   // User is permanently removed from database");
    println!();

    // In a real app:
    // user.force_delete(&db).await?;
    // The record is now permanently deleted
}

// ============================================================================
// Example 5: Workflow - Complete Use Case
// ============================================================================

fn example_complete_workflow() {
    println!("====================================");
    println!("Example 5: Complete Workflow");
    println!("====================================");
    println!();

    let mut user = ActiveModel {
        id: Set(4),
        name: Set("Alice Johnson".to_string()),
        email: Set("alice@example.com".to_string()),
        deleted_at: Set(None),
    };

    println!("Step 1: Create user");
    println!("   Status: Active");
    println!("   Is deleted? {}", user.is_trashed());
    println!();

    println!("Step 2: User requests account deletion");
    user.soft_delete();
    println!("   Status: Soft-deleted");
    println!("   Is deleted? {}", user.is_trashed());
    if let Some(deleted_at) = user.deleted_at() {
        println!("   Deleted at: {}", deleted_at);
    }
    println!();

    println!("Step 3: User changes mind and wants to restore account");
    user.restore();
    println!("   Status: Restored");
    println!("   Is deleted? {}", user.is_trashed());
    println!("   Deleted at: {:?}", user.deleted_at());
    println!();

    println!("Step 4: User wants permanent deletion");
    user.soft_delete();
    println!("   Status: Soft-deleted again");
    println!("   Is deleted? {}", user.is_trashed());
    println!();

    println!("Step 5: After 30 days, permanent deletion (force delete)");
    println!("   user.force_delete(&db).await?;");
    println!("   Status: Permanently deleted from database");
    println!();
}

// ============================================================================
// Example 6: Batch Operations
// ============================================================================

fn example_batch_operations() {
    println!("====================================");
    println!("Example 6: Batch Operations");
    println!("====================================");
    println!();

    // Create multiple users
    let mut users: Vec<ActiveModel> = (1..=5)
        .map(|i| ActiveModel {
            id: Set(i),
            name: Set(format!("User {}", i)),
            email: Set(format!("user{}@example.com", i)),
            deleted_at: Set(None),
        })
        .collect();

    println!("1. Created {} users", users.len());
    println!();

    // Soft delete even numbered users
    println!("2. Soft deleting even-numbered users...");
    for user in users.iter_mut().filter(|u| {
        if let ActiveValue::Set(id) = u.id {
            id % 2 == 0
        } else {
            false
        }
    }) {
        user.soft_delete();
    }

    let trashed_count = users.iter().filter(|u| u.is_trashed()).count();
    let active_count = users.iter().filter(|u| !u.is_trashed()).count();

    println!("   Active users: {}", active_count);
    println!("   Soft-deleted users: {}", trashed_count);
    println!();

    // Restore all
    println!("3. Restoring all users...");
    for user in &mut users {
        user.restore();
    }

    let trashed_after_restore = users.iter().filter(|u| u.is_trashed()).count();
    println!("   Active users: {}", users.len());
    println!("   Soft-deleted users: {}", trashed_after_restore);
    println!();
}

// ============================================================================
// Example 7: Helper Functions
// ============================================================================

fn example_helper_functions() {
    println!("====================================");
    println!("Example 7: Helper Functions");
    println!("====================================");
    println!();

    println!("1. set_deleted_at() - Get current timestamp:");
    let deleted_at = set_deleted_at();
    println!("   Type: ActiveValue<Option<DateTime<Utc>>>");
    println!("   Value: Set(Some(current_timestamp))");
    println!();

    println!("2. clear_deleted_at() - Clear timestamp:");
    let cleared = clear_deleted_at();
    println!("   Type: ActiveValue<Option<DateTime<Utc>>>");
    println!("   Value: Set(None)");
    println!();

    // Manual usage
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Manual User".to_string()),
        email: Set("manual@example.com".to_string()),
        deleted_at: Set(None),
    };

    println!("3. Manual usage:");
    println!("   Before: Is deleted? {}", user.is_trashed());

    user.deleted_at = set_deleted_at();
    println!("   After set_deleted_at(): Is deleted? {}", user.is_trashed());

    user.deleted_at = clear_deleted_at();
    println!("   After clear_deleted_at(): Is deleted? {}", user.is_trashed());
    println!();
}

// ============================================================================
// Main Demo
// ============================================================================

#[tokio::main]
async fn main() {
    println!();
    println!("╔════════════════════════════════════╗");
    println!("║   Soft Deletes Demo - RustForge   ║");
    println!("╚════════════════════════════════════╝");
    println!();

    example_basic_soft_delete();
    example_restore();
    example_query_scopes().await;
    example_force_delete().await;
    example_complete_workflow();
    example_batch_operations();
    example_helper_functions();

    println!("====================================");
    println!("Demo Complete!");
    println!("====================================");
    println!();
    println!("To use soft deletes in your models:");
    println!("1. Add `deleted_at: Option<DateTimeUtc>` field");
    println!("2. Implement SoftDeletes trait");
    println!("3. Use soft_delete(), restore(), is_trashed()");
    println!("4. Query with Entity::with_trashed() or ::only_trashed()");
    println!();
}
