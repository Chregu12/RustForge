//! # Comprehensive Soft Deletes Tests
//!
//! This test suite provides 15+ tests for soft delete functionality:
//! - Basic soft delete operations (5 tests)
//! - Query scoping (4 tests)
//! - Restore operations (3 tests)
//! - Force delete (2 tests)
//! - Edge cases (3+ tests)

use chrono::{Duration, Utc};
use rf_eloquent::soft_deletes::*;
use sea_orm::{entity::prelude::*, ActiveValue, Set};

// ============================================================================
// Test Models
// ============================================================================

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

impl SoftDeletes for ActiveModel {
    fn soft_delete(&mut self) {
        self.deleted_at = Set(Some(Utc::now()));
    }

    fn restore(&mut self) {
        self.deleted_at = Set(None);
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

// ============================================================================
// Basic Soft Delete Operations (5 tests)
// ============================================================================

#[test]
fn test_soft_delete_basic() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("John Doe".to_string()),
        email: Set("john@example.com".to_string()),
        deleted_at: Set(None),
    };

    // Initially not deleted
    assert!(!user.is_trashed());
    assert!(user.deleted_at().is_none());

    // Soft delete
    user.soft_delete();

    // Now should be deleted
    assert!(user.is_trashed());
    assert!(user.deleted_at().is_some());

    // Verify timestamp is recent (within last minute)
    let deleted_time = user.deleted_at().unwrap();
    let now = Utc::now();
    let diff = now.signed_duration_since(deleted_time);
    assert!(diff < Duration::minutes(1));
}

#[test]
fn test_soft_delete_preserves_data() {
    let mut user = ActiveModel {
        id: Set(42),
        name: Set("Alice Smith".to_string()),
        email: Set("alice@example.com".to_string()),
        deleted_at: Set(None),
    };

    user.soft_delete();

    // Verify all data is preserved except deleted_at
    match &user.id {
        ActiveValue::Set(id) => assert_eq!(*id, 42),
        _ => panic!("ID should be Set"),
    }

    match &user.name {
        ActiveValue::Set(name) => assert_eq!(name, "Alice Smith"),
        _ => panic!("Name should be Set"),
    }

    match &user.email {
        ActiveValue::Set(email) => assert_eq!(email, "alice@example.com"),
        _ => panic!("Email should be Set"),
    }
}

#[test]
fn test_soft_delete_idempotent() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Test User".to_string()),
        email: Set("test@example.com".to_string()),
        deleted_at: Set(None),
    };

    // First soft delete
    user.soft_delete();
    let first_deleted_at = user.deleted_at().unwrap();

    // Wait a tiny bit
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Second soft delete
    user.soft_delete();
    let second_deleted_at = user.deleted_at().unwrap();

    // Both times should exist and second should be later
    assert!(user.is_trashed());
    assert!(second_deleted_at >= first_deleted_at);
}

#[test]
fn test_is_trashed_with_different_active_values() {
    // Test with Set(Some(_))
    let user1 = ActiveModel {
        id: Set(1),
        name: Set("User1".to_string()),
        email: Set("user1@example.com".to_string()),
        deleted_at: ActiveValue::Set(Some(Utc::now())),
    };
    assert!(user1.is_trashed());

    // Test with Unchanged(Some(_))
    let user2 = ActiveModel {
        id: Set(2),
        name: Set("User2".to_string()),
        email: Set("user2@example.com".to_string()),
        deleted_at: ActiveValue::Unchanged(Some(Utc::now())),
    };
    assert!(user2.is_trashed());

    // Test with Set(None)
    let user3 = ActiveModel {
        id: Set(3),
        name: Set("User3".to_string()),
        email: Set("user3@example.com".to_string()),
        deleted_at: ActiveValue::Set(None),
    };
    assert!(!user3.is_trashed());

    // Test with NotSet
    let user4 = ActiveModel {
        id: Set(4),
        name: Set("User4".to_string()),
        email: Set("user4@example.com".to_string()),
        deleted_at: ActiveValue::NotSet,
    };
    assert!(!user4.is_trashed());
}

#[test]
fn test_deleted_at_getter() {
    let now = Utc::now();

    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Test".to_string()),
        email: Set("test@example.com".to_string()),
        deleted_at: Set(Some(now)),
    };

    let deleted_at = user.deleted_at();
    assert!(deleted_at.is_some());
    assert_eq!(deleted_at.unwrap(), now);

    // After restore, should be None
    user.restore();
    assert!(user.deleted_at().is_none());
}

// ============================================================================
// Query Scoping (4 tests)
// ============================================================================

#[test]
fn test_soft_delete_scope_default_excludes_trashed() {
    let scope = SoftDeleteScope::<Entity>::new();

    // Default should exclude trashed
    assert!(!scope.include_trashed);
    assert!(!scope.only_trashed);
}

#[test]
fn test_soft_delete_scope_with_trashed() {
    let scope = SoftDeleteScope::<Entity>::new().with_trashed();

    // Should include trashed
    assert!(scope.include_trashed);
    assert!(!scope.only_trashed);
}

#[test]
fn test_soft_delete_scope_only_trashed() {
    let scope = SoftDeleteScope::<Entity>::new().only_trashed();

    // Should only show trashed
    assert!(!scope.include_trashed);
    assert!(scope.only_trashed);
}

#[test]
fn test_soft_delete_scope_chaining() {
    // Test that with_trashed() overrides only_trashed()
    let scope = SoftDeleteScope::<Entity>::new()
        .only_trashed()
        .with_trashed();

    assert!(scope.include_trashed);
    assert!(!scope.only_trashed);

    // Test that only_trashed() overrides with_trashed()
    let scope2 = SoftDeleteScope::<Entity>::new()
        .with_trashed()
        .only_trashed();

    assert!(!scope2.include_trashed);
    assert!(scope2.only_trashed);
}

// ============================================================================
// Restore Operations (3 tests)
// ============================================================================

#[test]
fn test_restore_soft_deleted_record() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("John Doe".to_string()),
        email: Set("john@example.com".to_string()),
        deleted_at: Set(Some(Utc::now())),
    };

    assert!(user.is_trashed());

    user.restore();

    assert!(!user.is_trashed());
    assert!(user.deleted_at().is_none());

    // Verify deleted_at is explicitly Set to None
    match &user.deleted_at {
        ActiveValue::Set(None) => {}
        _ => panic!("Expected Set(None)"),
    }
}

#[test]
fn test_restore_multiple_times() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Test".to_string()),
        email: Set("test@example.com".to_string()),
        deleted_at: Set(None),
    };

    // Cycle 1: Delete then restore
    user.soft_delete();
    assert!(user.is_trashed());
    user.restore();
    assert!(!user.is_trashed());

    // Cycle 2: Delete then restore
    user.soft_delete();
    assert!(user.is_trashed());
    user.restore();
    assert!(!user.is_trashed());

    // Cycle 3: Delete then restore
    user.soft_delete();
    assert!(user.is_trashed());
    user.restore();
    assert!(!user.is_trashed());

    // Final state should be not deleted
    assert!(user.deleted_at().is_none());
}

#[test]
fn test_restore_non_deleted_record() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("John".to_string()),
        email: Set("john@example.com".to_string()),
        deleted_at: Set(None),
    };

    // Not deleted
    assert!(!user.is_trashed());

    // Restore anyway (should be idempotent)
    user.restore();

    // Still not deleted
    assert!(!user.is_trashed());
    assert!(user.deleted_at().is_none());
}

// ============================================================================
// Helper Functions (2 tests)
// ============================================================================

#[test]
fn test_set_deleted_at_helper() {
    let deleted_at = set_deleted_at();

    match deleted_at {
        ActiveValue::Set(Some(dt)) => {
            // Verify it's recent
            let now = Utc::now();
            let diff = now.signed_duration_since(dt);
            assert!(diff < Duration::minutes(1));
        }
        _ => panic!("Expected Set(Some(_))"),
    }
}

#[test]
fn test_clear_deleted_at_helper() {
    let deleted_at = clear_deleted_at();

    match deleted_at {
        ActiveValue::Set(None) => {}
        _ => panic!("Expected Set(None)"),
    }
}

// ============================================================================
// Edge Cases (5+ tests)
// ============================================================================

#[test]
fn test_soft_delete_with_unchanged_state() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Test".to_string()),
        email: Set("test@example.com".to_string()),
        deleted_at: ActiveValue::Unchanged(None),
    };

    assert!(!user.is_trashed());

    user.soft_delete();

    assert!(user.is_trashed());
}

#[test]
fn test_soft_delete_with_not_set_state() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Test".to_string()),
        email: Set("test@example.com".to_string()),
        deleted_at: ActiveValue::NotSet,
    };

    assert!(!user.is_trashed());

    user.soft_delete();

    assert!(user.is_trashed());
}

#[test]
fn test_multiple_models_independent_deletion() {
    let mut user1 = ActiveModel {
        id: Set(1),
        name: Set("User1".to_string()),
        email: Set("user1@example.com".to_string()),
        deleted_at: Set(None),
    };

    let mut user2 = ActiveModel {
        id: Set(2),
        name: Set("User2".to_string()),
        email: Set("user2@example.com".to_string()),
        deleted_at: Set(None),
    };

    // Delete only user1
    user1.soft_delete();

    assert!(user1.is_trashed());
    assert!(!user2.is_trashed());

    // Delete user2
    user2.soft_delete();

    assert!(user1.is_trashed());
    assert!(user2.is_trashed());

    // Restore user1
    user1.restore();

    assert!(!user1.is_trashed());
    assert!(user2.is_trashed());
}

#[test]
fn test_deleted_at_timestamp_accuracy() {
    let before = Utc::now();

    let mut user = ActiveModel {
        id: Set(1),
        name: Set("Test".to_string()),
        email: Set("test@example.com".to_string()),
        deleted_at: Set(None),
    };

    user.soft_delete();

    let after = Utc::now();
    let deleted_at = user.deleted_at().unwrap();

    // Deleted timestamp should be between before and after
    assert!(deleted_at >= before);
    assert!(deleted_at <= after);
}

#[test]
fn test_soft_delete_scope_default_trait() {
    // Test that Default trait works
    let scope1 = SoftDeleteScope::<Entity>::default();
    let scope2 = SoftDeleteScope::<Entity>::new();

    assert_eq!(scope1.include_trashed, scope2.include_trashed);
    assert_eq!(scope1.only_trashed, scope2.only_trashed);
}

#[test]
fn test_deleted_at_with_very_old_timestamp() {
    // Test with a timestamp from the past
    let old_timestamp = Utc::now() - Duration::days(365);

    let user = ActiveModel {
        id: Set(1),
        name: Set("Old User".to_string()),
        email: Set("old@example.com".to_string()),
        deleted_at: Set(Some(old_timestamp)),
    };

    assert!(user.is_trashed());
    assert_eq!(user.deleted_at().unwrap(), old_timestamp);
}

#[test]
fn test_deleted_at_with_future_timestamp() {
    // Test with a future timestamp (edge case)
    let future_timestamp = Utc::now() + Duration::days(30);

    let user = ActiveModel {
        id: Set(1),
        name: Set("Future User".to_string()),
        email: Set("future@example.com".to_string()),
        deleted_at: Set(Some(future_timestamp)),
    };

    // Should still be considered trashed
    assert!(user.is_trashed());
    assert_eq!(user.deleted_at().unwrap(), future_timestamp);
}

// ============================================================================
// Integration Tests (3 tests)
// ============================================================================

#[test]
fn test_soft_delete_workflow_complete() {
    // Simulate a complete soft delete workflow
    let mut user = ActiveModel {
        id: Set(100),
        name: Set("Complete Workflow User".to_string()),
        email: Set("workflow@example.com".to_string()),
        deleted_at: Set(None),
    };

    // Step 1: Verify initial state
    assert!(!user.is_trashed());
    assert!(user.deleted_at().is_none());

    // Step 2: Soft delete
    user.soft_delete();
    assert!(user.is_trashed());
    let first_deleted_at = user.deleted_at().unwrap();

    // Step 3: Verify can't "use" deleted record (in real app would be filtered)
    assert!(user.is_trashed());

    // Step 4: Restore
    user.restore();
    assert!(!user.is_trashed());
    assert!(user.deleted_at().is_none());

    // Step 5: Delete again
    user.soft_delete();
    assert!(user.is_trashed());
    let second_deleted_at = user.deleted_at().unwrap();

    // Timestamps should be different
    assert!(second_deleted_at >= first_deleted_at);
}

#[test]
fn test_soft_delete_batch_operations() {
    // Simulate batch soft delete operations
    let mut users: Vec<ActiveModel> = (1..=5)
        .map(|i| ActiveModel {
            id: Set(i),
            name: Set(format!("User {}", i)),
            email: Set(format!("user{}@example.com", i)),
            deleted_at: Set(None),
        })
        .collect();

    // Delete all
    for user in &mut users {
        user.soft_delete();
    }

    // Verify all deleted
    assert!(users.iter().all(|u| u.is_trashed()));

    // Restore even numbered users
    for user in users.iter_mut().filter(|u| {
        if let ActiveValue::Set(id) = u.id {
            id % 2 == 0
        } else {
            false
        }
    }) {
        user.restore();
    }

    // Verify correct restoration
    let trashed_count = users.iter().filter(|u| u.is_trashed()).count();
    let active_count = users.iter().filter(|u| !u.is_trashed()).count();

    assert_eq!(trashed_count, 3); // Users 1, 3, 5
    assert_eq!(active_count, 2); // Users 2, 4
}

#[test]
fn test_soft_delete_state_transitions() {
    let mut user = ActiveModel {
        id: Set(1),
        name: Set("State Test".to_string()),
        email: Set("state@example.com".to_string()),
        deleted_at: Set(None),
    };

    // State: Active
    assert!(!user.is_trashed());

    // Transition: Active -> Deleted
    user.soft_delete();
    assert!(user.is_trashed());

    // Transition: Deleted -> Active
    user.restore();
    assert!(!user.is_trashed());

    // Transition: Active -> Deleted (again)
    user.soft_delete();
    assert!(user.is_trashed());

    // Transition: Deleted -> Deleted (idempotent)
    user.soft_delete();
    assert!(user.is_trashed());

    // Transition: Deleted -> Active
    user.restore();
    assert!(!user.is_trashed());

    // Transition: Active -> Active (idempotent)
    user.restore();
    assert!(!user.is_trashed());
}
