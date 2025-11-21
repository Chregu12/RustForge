# ADR-007: Job Queue Backend

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Production Job Queues benötigen:
- Persistence (kein Job-Loss bei Crash)
- Priority Queues
- Delayed Jobs (schedule for later)
- Retry-Mechanismus mit Exponential Backoff
- Dead Letter Queue (DLQ) für Failed Jobs

## Decision

**Redis** als primäres Queue-Backend (mit Postgres als Alternative)

### Begründung Redis:

**Vorteile:**
- ✅ Low Latency (in-memory)
- ✅ Sorted Sets für Priority + Delayed Jobs
- ✅ Atomic Operations (LPUSH/BRPOP)
- ✅ Pub/Sub für Worker-Notifications
- ✅ Battle-tested (Sidekiq, BullMQ)

**Nachteile:**
- ❌ Persistence-Garantie schwächer als DB
- ❌ Zusätzliche Infrastruktur

### Begründung Postgres (Alternative):

**Vorteile:**
- ✅ ACID-Garantien
- ✅ Keine zusätzliche Infrastruktur
- ✅ Transaktionale Jobs (Job + DB-Update atomar)

**Nachteile:**
- ❌ Höhere Latenz
- ❌ Polling statt Push (oder LISTEN/NOTIFY)

### Entscheidung:

**Redis als Default, Postgres als Option**

- Production: Redis (Performance)
- Small Apps: Postgres (Simplicity)
- Beide Backends über trait abstrahiert

## API-Design:

```rust
// Trait für Backend-Abstraktion
#[async_trait]
pub trait QueueBackend: Send + Sync {
    async fn push(&self, queue: &str, job: &Job) -> Result<()>;
    async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<Job>>;
    async fn schedule(&self, queue: &str, job: &Job, delay: Duration) -> Result<()>;
    async fn ack(&self, job_id: &str) -> Result<()>;
    async fn nack(&self, job_id: &str) -> Result<()>;
    async fn dead_letter(&self, job: &Job) -> Result<()>;
}

// Redis Implementation
pub struct RedisBackend {
    pool: deadpool_redis::Pool,
}

#[async_trait]
impl QueueBackend for RedisBackend {
    async fn push(&self, queue: &str, job: &Job) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let json = serde_json::to_string(job)?;

        redis::cmd("LPUSH")
            .arg(format!("queue:{}", queue))
            .arg(json)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<Job>> {
        let mut conn = self.pool.get().await?;

        let result: Option<(String, String)> = redis::cmd("BRPOP")
            .arg(format!("queue:{}", queue))
            .arg(timeout.as_secs())
            .query_async(&mut conn)
            .await?;

        match result {
            Some((_, json)) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn schedule(&self, queue: &str, job: &Job, delay: Duration) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let json = serde_json::to_string(job)?;
        let score = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() + delay.as_secs();

        redis::cmd("ZADD")
            .arg(format!("delayed:{}", queue))
            .arg(score)
            .arg(json)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }
}

// Postgres Implementation
pub struct PostgresBackend {
    pool: PgPool,
}

#[async_trait]
impl QueueBackend for PostgresBackend {
    async fn push(&self, queue: &str, job: &Job) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO queue_jobs (queue, payload, status, created_at)
            VALUES ($1, $2, 'pending', NOW())
            "#,
            queue,
            serde_json::to_value(job)?,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<Job>> {
        // Use SKIP LOCKED for concurrent workers
        let record = sqlx::query!(
            r#"
            DELETE FROM queue_jobs
            WHERE id = (
                SELECT id FROM queue_jobs
                WHERE queue = $1 AND status = 'pending' AND run_at <= NOW()
                ORDER BY priority DESC, created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING payload
            "#,
            queue
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(r) => Ok(Some(serde_json::from_value(r.payload)?)),
            None => Ok(None),
        }
    }

    // ...
}
```

## Implementation Strategy

### Redis Queue Schema:

```
Keys:
- queue:{name}           → List (LPUSH/BRPOP)
- delayed:{name}         → Sorted Set (score = timestamp)
- processing:{worker_id} → Hash (active jobs)
- dlq:{name}             → List (failed jobs)
- stats:{name}           → Hash (counters)
```

### Postgres Queue Schema:

```sql
CREATE TABLE queue_jobs (
    id BIGSERIAL PRIMARY KEY,
    queue VARCHAR(255) NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    failed_at TIMESTAMPTZ,
    error TEXT
);

CREATE INDEX idx_queue_jobs_pending ON queue_jobs (queue, status, run_at)
    WHERE status = 'pending';

CREATE INDEX idx_queue_jobs_priority ON queue_jobs (priority DESC, created_at ASC)
    WHERE status = 'pending';
```

### Worker Implementation:

```rust
pub struct QueueWorker<B: QueueBackend> {
    backend: Arc<B>,
    queue: String,
    concurrency: usize,
}

impl<B: QueueBackend> QueueWorker<B> {
    pub async fn run(self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(self.concurrency);

        // Spawn worker tasks
        for _ in 0..self.concurrency {
            let backend = self.backend.clone();
            let queue = self.queue.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                loop {
                    match backend.pop(&queue, Duration::from_secs(30)).await {
                        Ok(Some(job)) => {
                            if let Err(e) = process_job(&job).await {
                                backend.nack(&job.id).await.ok();
                            } else {
                                backend.ack(&job.id).await.ok();
                            }
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            error!("Queue error: {}", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            });
        }

        // Delayed job scheduler
        tokio::spawn(async move {
            loop {
                // Move delayed jobs to main queue when ready
                // ...
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }
}
```

## Consequences

**Positiv:**
- ✅ Backend-agnostisch via Trait
- ✅ Redis für High-Performance
- ✅ Postgres für Simplicity
- ✅ Beide Production-Ready

**Negativ:**
- ❌ Doppelte Backend-Implementierung
- ❌ Test-Komplexität (beide Backends testen)
