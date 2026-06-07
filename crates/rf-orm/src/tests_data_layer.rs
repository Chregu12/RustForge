//! Data-layer unit & integration tests for rf-orm.
//!
//! Covers:
//! - PaginatedResults helpers
//! - BatchResult / MigrationStatus logic
//! - MigrationError variants
//! - MigrationManager (run, rollback, reset, fresh, status) with SQLite in-memory
//! - Collection helpers

// ============================================================
// QueryBuilder / PaginatedResults
// ============================================================
#[cfg(test)]
mod query_builder_tests {
    use crate::query_builder::PaginatedResults;

    #[test]
    fn test_paginated_single_page_has_no_neighbors() {
        let r = PaginatedResults {
            data: vec![1, 2],
            current_page: 1,
            per_page: 10,
            total: 2,
            total_pages: 1,
            from: 1,
            to: 2,
        };
        assert!(r.on_first_page());
        assert!(r.on_last_page());
        assert!(!r.has_more_pages());
        assert_eq!(r.next_page(), None);
        assert_eq!(r.previous_page(), None);
    }

    #[test]
    fn test_paginated_middle_page() {
        let r = PaginatedResults {
            data: vec![11, 12, 13],
            current_page: 2,
            per_page: 3,
            total: 9,
            total_pages: 3,
            from: 4,
            to: 6,
        };
        assert!(!r.on_first_page());
        assert!(!r.on_last_page());
        assert!(r.has_more_pages());
        assert_eq!(r.next_page(), Some(3));
        assert_eq!(r.previous_page(), Some(1));
    }

    #[test]
    fn test_paginated_empty_result() {
        let r: PaginatedResults<i32> = PaginatedResults {
            data: vec![],
            current_page: 1,
            per_page: 10,
            total: 0,
            total_pages: 0,
            from: 0,
            to: 0,
        };
        assert_eq!(r.data.len(), 0);
        assert_eq!(r.next_page(), None);
    }

    #[test]
    fn test_paginated_last_page() {
        let r = PaginatedResults {
            data: vec![91, 92, 93],
            current_page: 10,
            per_page: 10,
            total: 93,
            total_pages: 10,
            from: 91,
            to: 93,
        };
        assert!(!r.has_more_pages());
        assert!(r.on_last_page());
        assert!(!r.on_first_page());
        assert_eq!(r.next_page(), None);
        assert_eq!(r.previous_page(), Some(9));
    }

    #[test]
    fn test_paginated_results_from_first_page() {
        let r = PaginatedResults {
            data: vec![1, 2, 3],
            current_page: 1,
            per_page: 3,
            total: 15,
            total_pages: 5,
            from: 1,
            to: 3,
        };
        assert!(r.on_first_page());
        assert!(!r.on_last_page());
        assert!(r.has_more_pages());
        assert_eq!(r.next_page(), Some(2));
        assert_eq!(r.previous_page(), None);
    }
}

// ============================================================
// BatchResult + MigrationStatus + MigrationError
// ============================================================
#[cfg(test)]
mod migration_logic_tests {
    use crate::migrations::{BatchResult, MigrationError, MigrationStatus};
    use chrono::Utc;

    // --- BatchResult ---

    #[test]
    fn test_batch_result_starts_empty() {
        let r = BatchResult::new(1);
        assert_eq!(r.batch, 1);
        assert_eq!(r.migrations_run, 0);
        assert!(r.is_successful());
        assert!(r.successful.is_empty());
        assert!(r.failed.is_empty());
    }

    #[test]
    fn test_batch_result_add_success_increments_counter() {
        let mut r = BatchResult::new(2);
        r.add_success("m_001".to_string());
        r.add_success("m_002".to_string());
        assert_eq!(r.migrations_run, 2);
        assert_eq!(r.successful.len(), 2);
        assert!(r.is_successful());
    }

    #[test]
    fn test_batch_result_failure_makes_not_successful() {
        let mut r = BatchResult::new(1);
        r.add_success("m_001".to_string());
        r.add_failure("m_002".to_string(), "constraint violated".to_string());
        assert!(!r.is_successful());
        assert_eq!(r.successful.len(), 1);
        assert_eq!(r.failed.len(), 1);
        assert_eq!(r.failed[0].1, "constraint violated");
    }

    #[test]
    fn test_batch_result_display_contains_batch_number() {
        let mut r = BatchResult::new(5);
        r.add_success("a".to_string());
        let s = r.to_string();
        assert!(s.contains("Batch 5"), "display = '{}'", s);
        assert!(s.contains("1 migrations run"), "display = '{}'", s);
    }

    #[test]
    fn test_batch_result_display_counts_failures() {
        let mut r = BatchResult::new(3);
        r.add_success("x".to_string());
        r.add_failure("y".to_string(), "err".to_string());
        let s = r.to_string();
        assert!(s.contains("1 successful"));
        assert!(s.contains("1 failed"));
    }

    // --- MigrationStatus ---

    #[test]
    fn test_migration_status_executed_display() {
        let status = MigrationStatus {
            name: "create_users_table".to_string(),
            executed: true,
            batch: Some(1),
            executed_at: Some(Utc::now()),
        };
        let s = status.to_string();
        assert!(s.contains("[X]"));
        assert!(s.contains("create_users_table"));
        assert!(s.contains("batch: 1"));
    }

    #[test]
    fn test_migration_status_pending_display() {
        let status = MigrationStatus {
            name: "add_index_on_email".to_string(),
            executed: false,
            batch: None,
            executed_at: None,
        };
        let s = status.to_string();
        assert!(s.contains("[ ]"));
        assert!(s.contains("pending"));
        assert!(s.contains("add_index_on_email"));
    }

    // --- MigrationError ---

    #[test]
    fn test_migration_error_already_applied() {
        let err = MigrationError::AlreadyApplied("mig_001".to_string());
        assert!(err.to_string().contains("already been applied"));
        assert!(err.to_string().contains("mig_001"));
    }

    #[test]
    fn test_migration_error_not_found() {
        let err = MigrationError::NotFound("missing".to_string());
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_migration_error_no_migrations_to_rollback() {
        let err = MigrationError::NoMigrationsToRollback;
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_migration_error_execution_failed() {
        let err = MigrationError::ExecutionFailed {
            migration: "bad_mig".to_string(),
            error: "syntax error".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("bad_mig"));
        assert!(s.contains("syntax error"));
    }

    #[test]
    fn test_migration_error_invalid_state() {
        let err = MigrationError::InvalidState("corrupt".to_string());
        assert!(err.to_string().contains("Invalid migration state"));
    }
}

// ============================================================
// MigrationManager – SQLite in-memory integration tests
// ============================================================
#[cfg(test)]
mod migration_integration_tests {
    use crate::migrations::{Migration, MigrationError, MigrationResult, Migrator, SchemaContext};
    use async_trait::async_trait;
    use sea_orm::Database;

    // Three cheap no-op migrations
    struct NoopA;
    struct NoopB;
    struct NoopC;
    // One migration that actually creates a table
    struct CreateItemsTable;

    #[async_trait]
    impl Migration for NoopA {
        fn name(&self) -> &str { "2026_01_01_000001_noop_a" }
        async fn up(&self, _s: &SchemaContext) -> MigrationResult<()> { Ok(()) }
        async fn down(&self, _s: &SchemaContext) -> MigrationResult<()> { Ok(()) }
    }
    #[async_trait]
    impl Migration for NoopB {
        fn name(&self) -> &str { "2026_01_01_000002_noop_b" }
        async fn up(&self, _s: &SchemaContext) -> MigrationResult<()> { Ok(()) }
        async fn down(&self, _s: &SchemaContext) -> MigrationResult<()> { Ok(()) }
    }
    #[async_trait]
    impl Migration for NoopC {
        fn name(&self) -> &str { "2026_01_01_000003_noop_c" }
        async fn up(&self, _s: &SchemaContext) -> MigrationResult<()> { Ok(()) }
        async fn down(&self, _s: &SchemaContext) -> MigrationResult<()> { Ok(()) }
    }

    #[async_trait]
    impl Migration for CreateItemsTable {
        fn name(&self) -> &str { "2026_01_02_000001_create_items_table" }
        async fn up(&self, schema: &SchemaContext) -> MigrationResult<()> {
            schema.create("items", |t| {
                t.id();
                t.string("label");
                t.timestamps();
            }).await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
            Ok(())
        }
        async fn down(&self, schema: &SchemaContext) -> MigrationResult<()> {
            schema.drop("items").await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
            Ok(())
        }
    }

    async fn mem_db() -> sea_orm::DatabaseConnection {
        Database::connect("sqlite::memory:").await.expect("sqlite::memory:")
    }

    #[tokio::test]
    async fn test_run_executes_pending_only() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(NoopA));
        m.add_migration(Box::new(NoopB));

        let r = m.run().await.unwrap();
        assert_eq!(r.migrations_run, 2);
        assert_eq!(r.batch, 1);
        assert!(r.is_successful());

        // Second run: nothing pending
        let r2 = m.run().await.unwrap();
        assert_eq!(r2.migrations_run, 0, "already-applied must be skipped");
    }

    #[tokio::test]
    async fn test_status_applied_vs_pending() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(NoopA));
        m.add_migration(Box::new(NoopB));
        m.run().await.unwrap();

        // Add third migration, not run yet
        m.add_migration(Box::new(NoopC));

        let status = m.status().await.unwrap();
        assert_eq!(status.len(), 3);
        assert!(status[0].executed, "A applied");
        assert!(status[1].executed, "B applied");
        assert!(!status[2].executed, "C pending");
        assert_eq!(status[0].batch, Some(1));
        assert_eq!(status[2].batch, None);
    }

    #[tokio::test]
    async fn test_batch_numbers_increment() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(NoopA));
        m.run().await.unwrap();

        m.add_migration(Box::new(NoopB));
        let r2 = m.run().await.unwrap();
        assert_eq!(r2.batch, 2, "second run gets batch 2");

        let status = m.status().await.unwrap();
        assert_eq!(status[0].batch, Some(1));
        assert_eq!(status[1].batch, Some(2));
    }

    #[tokio::test]
    async fn test_rollback_last_batch() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(NoopA));
        m.add_migration(Box::new(NoopB));
        m.run().await.unwrap();

        let rb = m.rollback(None).await.unwrap();
        assert_eq!(rb.migrations_run, 2);
        assert!(rb.is_successful());

        let status = m.status().await.unwrap();
        assert!(!status[0].executed);
        assert!(!status[1].executed);
    }

    #[tokio::test]
    async fn test_rollback_steps_1_only_touches_last_batch() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(NoopA));
        m.run().await.unwrap(); // batch 1

        m.add_migration(Box::new(NoopB));
        m.run().await.unwrap(); // batch 2

        let rb = m.rollback(Some(1)).await.unwrap();
        assert_eq!(rb.migrations_run, 1, "only B from batch 2 rolled back");

        let status = m.status().await.unwrap();
        assert!(status[0].executed, "A still applied");
        assert!(!status[1].executed, "B rolled back");
    }

    #[tokio::test]
    async fn test_fresh_reruns_all_migrations() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(NoopA));
        m.add_migration(Box::new(NoopB));
        m.run().await.unwrap();

        let fresh = m.fresh().await.unwrap();
        assert_eq!(fresh.migrations_run, 2);
        assert_eq!(fresh.batch, 1);
        assert!(fresh.is_successful());

        let status = m.status().await.unwrap();
        assert!(status[0].executed);
        assert!(status[1].executed);
    }

    #[tokio::test]
    async fn test_rollback_empty_returns_error() {
        let db = mem_db().await;
        let m = Migrator::new(db);

        let result = m.rollback(None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MigrationError::NoMigrationsToRollback
        ));
    }

    #[tokio::test]
    async fn test_migration_with_real_schema_operations() {
        let db = mem_db().await;
        let mut m = Migrator::new(db);
        m.add_migration(Box::new(CreateItemsTable));

        let r = m.run().await.unwrap();
        assert_eq!(r.migrations_run, 1);
        assert!(r.is_successful());

        let rb = m.rollback(None).await.unwrap();
        assert_eq!(rb.migrations_run, 1);
        assert!(rb.is_successful());
    }
}

// ============================================================
// Collection helpers
// ============================================================
#[cfg(test)]
mod collection_tests {
    use crate::collection::Collection;

    #[test]
    fn test_collection_new_and_count() {
        let c = Collection::new(vec![1, 2, 3]);
        assert_eq!(c.count(), 3);
        assert!(!c.is_empty());
    }

    #[test]
    fn test_collection_empty() {
        let c: Collection<i32> = Collection::empty();
        assert!(c.is_empty());
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn test_collection_first_and_last() {
        let c = Collection::new(vec![10, 20, 30]);
        assert_eq!(c.first(), Some(&10));
        assert_eq!(c.last(), Some(&30));
    }

    #[test]
    fn test_collection_filter() {
        let c = Collection::new(vec![1, 2, 3, 4, 5]);
        let evens = c.filter(|n| n % 2 == 0);
        assert_eq!(evens.to_vec(), vec![2, 4]);
    }

    #[test]
    fn test_collection_map() {
        let c = Collection::new(vec![1, 2, 3]);
        let doubled = c.map(|n| n * 2);
        assert_eq!(doubled.to_vec(), vec![2, 4, 6]);
    }

    #[test]
    fn test_collection_contains_predicate() {
        let c = Collection::new(vec![1, 2, 3]);
        assert!(c.contains(|n| *n == 2));
        assert!(!c.contains(|n| *n == 99));
    }

    #[test]
    fn test_collection_sum() {
        let c = Collection::new(vec![1, 2, 3, 4]);
        assert_eq!(c.sum(), 10);
    }

    #[test]
    fn test_collection_to_vec() {
        let v = vec![7, 8, 9];
        let c = Collection::new(v.clone());
        assert_eq!(c.to_vec(), v);
    }

    #[test]
    fn test_collection_reject() {
        let c = Collection::new(vec![1, 2, 3, 4, 5]);
        let odds = c.reject(|n| n % 2 == 0);
        assert_eq!(odds.to_vec(), vec![1, 3, 5]);
    }

    #[test]
    fn test_collection_take_and_skip() {
        let c = Collection::new(vec![1, 2, 3, 4, 5]);
        let taken = c.take(3);
        assert_eq!(taken.to_vec(), vec![1, 2, 3]);

        let c2 = Collection::new(vec![1, 2, 3, 4, 5]);
        let skipped = c2.skip(2);
        assert_eq!(skipped.to_vec(), vec![3, 4, 5]);
    }

    #[test]
    fn test_collection_sort() {
        let c = Collection::new(vec![5, 3, 1, 4, 2]);
        let sorted = c.sort();
        assert_eq!(sorted.to_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_collection_reverse() {
        let c = Collection::new(vec![1, 2, 3]);
        let reversed = c.reverse();
        assert_eq!(reversed.to_vec(), vec![3, 2, 1]);
    }
}
