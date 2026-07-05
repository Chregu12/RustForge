//! Database queue backend driver
//!
//! Provides a database-backed queue solution using SeaORM.

use crate::{JobMetadata, Queue, QueueError, QueueResult};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sea_orm::{
    entity::prelude::*, ActiveModelBehavior, ActiveValue::NotSet, ColumnTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Set,
};

/// Job entry entity
pub mod job_entry {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "jobs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub queue: String,
        pub payload: String,
        pub attempts: i32,
        pub reserved_at: Option<DateTime<Utc>>,
        pub available_at: DateTime<Utc>,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Failed job entry entity
pub mod failed_job_entry {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "failed_jobs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub connection: String,
        pub queue: String,
        pub payload: String,
        pub exception: String,
        pub failed_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Database queue driver
pub struct DatabaseQueue {
    db: DatabaseConnection,
    connection_name: String,
}

impl DatabaseQueue {
    /// Create a new database queue driver
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `connection_name` - Connection name for tracking
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_queue::drivers::database::DatabaseQueue;
    /// use sea_orm::{Database, DatabaseConnection};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = Database::connect("sqlite::memory:").await?;
    /// let queue = DatabaseQueue::new(db, "default".to_string());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(db: DatabaseConnection, connection_name: String) -> Self {
        Self {
            db,
            connection_name,
        }
    }

    /// Prune old jobs (completed or failed)
    pub async fn prune(&self, hours: i64) -> Result<u64, QueueError> {
        let cutoff = Utc::now() - ChronoDuration::hours(hours);

        let result = job_entry::Entity::delete_many()
            .filter(job_entry::Column::CreatedAt.lt(cutoff))
            .filter(job_entry::Column::ReservedAt.is_not_null())
            .exec(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to prune jobs: {}", e)))?;

        Ok(result.rows_affected)
    }

    /// Get failed jobs
    pub async fn get_failed_jobs(&self) -> Result<Vec<failed_job_entry::Model>, QueueError> {
        failed_job_entry::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to get failed jobs: {}", e)))
    }

    /// Retry all failed jobs
    pub async fn retry_failed_jobs(&self) -> Result<u64, QueueError> {
        let failed_jobs = self.get_failed_jobs().await?;
        let mut count = 0;

        for failed_job in failed_jobs {
            // Parse job metadata
            if let Ok(metadata) = serde_json::from_str::<JobMetadata>(&failed_job.payload) {
                // Push back to queue
                self.push(metadata).await?;

                // Delete from failed jobs
                failed_job_entry::Entity::delete_by_id(failed_job.id)
                    .exec(&self.db)
                    .await
                    .map_err(|e| {
                        QueueError::Backend(format!("Failed to delete failed job: {}", e))
                    })?;

                count += 1;
            }
        }

        Ok(count)
    }
}

#[async_trait]
impl Queue for DatabaseQueue {
    async fn push(&self, metadata: JobMetadata) -> QueueResult<String> {
        let payload = serde_json::to_string(&metadata)
            .map_err(|e| QueueError::SerializationError(e.to_string()))?;

        let available_at = metadata.execute_at.unwrap_or_else(Utc::now);

        let active = job_entry::ActiveModel {
            id: NotSet,
            queue: Set(metadata.queue.clone()),
            payload: Set(payload),
            attempts: Set(0),
            reserved_at: Set(None),
            available_at: Set(available_at),
            created_at: Set(Utc::now()),
        };

        let result = active
            .insert(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to push job: {}", e)))?;

        Ok(result.id.to_string())
    }

    async fn reserve(&self, queue: &str) -> QueueResult<Option<JobMetadata>> {
        // Find next available job
        let job = job_entry::Entity::find()
            .filter(job_entry::Column::Queue.eq(queue))
            .filter(job_entry::Column::ReservedAt.is_null())
            .filter(job_entry::Column::AvailableAt.lte(Utc::now()))
            .order_by_asc(job_entry::Column::Id)
            .one(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to find job: {}", e)))?;

        if let Some(job) = job {
            // Reserve the job
            let mut active: job_entry::ActiveModel = job.clone().into();
            active.reserved_at = Set(Some(Utc::now()));
            active.attempts = Set(job.attempts + 1);

            active
                .update(&self.db)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to reserve job: {}", e)))?;

            // Deserialize metadata
            let mut metadata: JobMetadata = serde_json::from_str(&job.payload)
                .map_err(|e| QueueError::DeserializationError(e.to_string()))?;

            // Set job ID and attempts
            metadata.id = job.id.to_string();
            metadata.attempts = job.attempts as u32;

            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    async fn complete(&self, job_id: &str) -> QueueResult<()> {
        let id: i64 = job_id
            .parse()
            .map_err(|e| QueueError::Backend(format!("Invalid job ID: {}", e)))?;

        job_entry::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to complete job: {}", e)))?;

        Ok(())
    }

    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        let id: i64 = job_id
            .parse()
            .map_err(|e| QueueError::Backend(format!("Invalid job ID: {}", e)))?;

        // Get the job
        let job = job_entry::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to find job: {}", e)))?
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        // Create failed job entry
        let failed = failed_job_entry::ActiveModel {
            id: NotSet,
            connection: Set(self.connection_name.clone()),
            queue: Set(job.queue.clone()),
            payload: Set(job.payload.clone()),
            exception: Set(error.to_string()),
            failed_at: Set(Utc::now()),
        };

        failed.insert(&self.db).await.map_err(|e| {
            QueueError::Backend(format!("Failed to create failed job entry: {}", e))
        })?;

        // Delete from jobs table
        job_entry::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to delete job: {}", e)))?;

        Ok(())
    }

    async fn retry(&self, metadata: JobMetadata) -> QueueResult<()> {
        // Just push it back to the queue
        self.push(metadata).await?;
        Ok(())
    }

    async fn size(&self, queue: &str) -> QueueResult<usize> {
        let count = job_entry::Entity::find()
            .filter(job_entry::Column::Queue.eq(queue))
            .filter(job_entry::Column::ReservedAt.is_null())
            .count(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to count jobs: {}", e)))?;

        Ok(count as usize)
    }

    async fn clear(&self, queue: &str) -> QueueResult<()> {
        job_entry::Entity::delete_many()
            .filter(job_entry::Column::Queue.eq(queue))
            .exec(&self.db)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to clear queue: {}", e)))?;

        Ok(())
    }
}

/// Migration helper to create the jobs and failed_jobs tables
pub fn get_migration_sql(database_type: &str) -> &'static str {
    match database_type {
        "postgres" | "postgresql" => {
            r#"
CREATE TABLE IF NOT EXISTS jobs (
    id BIGSERIAL PRIMARY KEY,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    reserved_at TIMESTAMP WITH TIME ZONE,
    available_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_jobs_queue_reserved ON jobs(queue, reserved_at);
CREATE INDEX IF NOT EXISTS idx_jobs_available ON jobs(available_at);

CREATE TABLE IF NOT EXISTS failed_jobs (
    id BIGSERIAL PRIMARY KEY,
    connection VARCHAR(255) NOT NULL,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    exception TEXT NOT NULL,
    failed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_failed_jobs_queue ON failed_jobs(queue);
"#
        }
        "mysql" => {
            r#"
CREATE TABLE IF NOT EXISTS jobs (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    reserved_at TIMESTAMP NULL,
    available_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_jobs_queue_reserved (queue, reserved_at),
    INDEX idx_jobs_available (available_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS failed_jobs (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    connection VARCHAR(255) NOT NULL,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    exception TEXT NOT NULL,
    failed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_failed_jobs_queue (queue)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
"#
        }
        "sqlite" => {
            r#"
CREATE TABLE IF NOT EXISTS jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    queue TEXT NOT NULL,
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    reserved_at TEXT,
    available_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_jobs_queue_reserved ON jobs(queue, reserved_at);
CREATE INDEX IF NOT EXISTS idx_jobs_available ON jobs(available_at);

CREATE TABLE IF NOT EXISTS failed_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection TEXT NOT NULL,
    queue TEXT NOT NULL,
    payload TEXT NOT NULL,
    exception TEXT NOT NULL,
    failed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_failed_jobs_queue ON failed_jobs(queue);
"#
        }
        _ => {
            r#"
CREATE TABLE jobs (
    id BIGINT PRIMARY KEY,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    reserved_at TIMESTAMP NULL,
    available_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_jobs_queue_reserved ON jobs(queue, reserved_at);
CREATE INDEX idx_jobs_available ON jobs(available_at);

CREATE TABLE failed_jobs (
    id BIGINT PRIMARY KEY,
    connection VARCHAR(255) NOT NULL,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    exception TEXT NOT NULL,
    failed_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_failed_jobs_queue ON failed_jobs(queue);
"#
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Job;
    use sea_orm::{Database, DatabaseBackend, Schema};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    struct TestJob {
        data: String,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "test_job"
        }
    }

    async fn setup_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to test database");

        // Create schema
        let schema = Schema::new(DatabaseBackend::Sqlite);

        let jobs_stmt = schema.create_table_from_entity(job_entry::Entity);
        db.execute(db.get_database_backend().build(&jobs_stmt))
            .await
            .expect("Failed to create jobs table");

        let failed_stmt = schema.create_table_from_entity(failed_job_entry::Entity);
        db.execute(db.get_database_backend().build(&failed_stmt))
            .await
            .expect("Failed to create failed_jobs table");

        db
    }

    #[tokio::test]
    async fn test_database_queue_push_and_reserve() {
        let db = setup_test_db().await;
        let queue = DatabaseQueue::new(db, "test".to_string());

        let job = TestJob {
            data: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        // Push job
        let job_id = queue.push(metadata).await.unwrap();
        assert!(!job_id.is_empty());

        // Reserve job
        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());

        let reserved_metadata = reserved.unwrap();
        assert_eq!(reserved_metadata.job_type, "test_job");
        assert_eq!(reserved_metadata.attempts, 0);
    }

    #[tokio::test]
    async fn test_database_queue_complete() {
        let db = setup_test_db().await;
        let queue = DatabaseQueue::new(db, "test".to_string());

        let job = TestJob {
            data: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        let job_id = queue.push(metadata).await.unwrap();
        let reserved = queue.reserve("default").await.unwrap().unwrap();

        // Complete job
        queue.complete(&reserved.id).await.unwrap();

        // Verify queue is empty
        let size = queue.size("default").await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_database_queue_fail() {
        let db = setup_test_db().await;
        let queue = DatabaseQueue::new(db, "test".to_string());

        let job = TestJob {
            data: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        queue.push(metadata).await.unwrap();
        let reserved = queue.reserve("default").await.unwrap().unwrap();

        // Fail job
        queue
            .fail(&reserved.id, "Test error")
            .await
            .unwrap();

        // Verify job is in failed_jobs
        let failed_jobs = queue.get_failed_jobs().await.unwrap();
        assert_eq!(failed_jobs.len(), 1);
    }
}
