//! Jobs Demo - Background Job Processing Examples
//!
//! This demo showcases:
//! - Defining background jobs
//! - Dispatching jobs to queue
//! - Worker pool processing
//! - Job scheduling
//! - Retry logic
//! - Failed job handling

use async_trait::async_trait;
use rf_jobs::{Job, JobContext, JobResult, QueueManager, Scheduler, WorkerConfig, WorkerPool};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Example Jobs
// ============================================================================

/// Example 1: Send Email Job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
    body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Sending email to {}", self.to));

        // Simulate email sending
        tokio::time::sleep(Duration::from_secs(1)).await;

        ctx.log(&format!("Email sent successfully to {}", self.to));
        Ok(())
    }

    fn queue(&self) -> &str {
        "emails"
    }

    fn max_attempts(&self) -> u32 {
        3
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(60)
    }
}

/// Example 2: Process Image Job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessImageJob {
    path: String,
    operations: Vec<String>,
}

#[async_trait]
impl Job for ProcessImageJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Processing image: {}", self.path));

        for operation in &self.operations {
            ctx.log(&format!("Applying operation: {}", operation));
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        ctx.log("Image processing complete");
        Ok(())
    }

    fn queue(&self) -> &str {
        "images"
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

/// Example 3: Generate Report Job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerateReportJob {
    report_type: String,
    start_date: String,
    end_date: String,
}

#[async_trait]
impl Job for GenerateReportJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!(
            "Generating {} report from {} to {}",
            self.report_type, self.start_date, self.end_date
        ));

        // Simulate report generation
        tokio::time::sleep(Duration::from_secs(2)).await;

        ctx.log("Report generated successfully");
        Ok(())
    }

    fn queue(&self) -> &str {
        "reports"
    }
}

/// Example 4: Cache Cleanup Job (for scheduling)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheCleanupJob;

#[async_trait]
impl Job for CacheCleanupJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log("Running cache cleanup");

        // Simulate cleanup
        tokio::time::sleep(Duration::from_secs(1)).await;

        ctx.log("Cache cleanup complete");
        Ok(())
    }
}

/// Example 5: Failing Job (for retry demonstration)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FailingJob {
    fail_count: u32,
}

#[async_trait]
impl Job for FailingJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Attempting job (will fail {} times)", self.fail_count));

        if ctx.attempt() <= self.fail_count {
            ctx.warn("Job failed, will retry");
            return Err(rf_jobs::JobError::Custom(
                "Simulated failure".to_string(),
            ));
        }

        ctx.log("Job succeeded!");
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        5
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(5)
    }
}

// ============================================================================
// Demo Application
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    println!("🚀 rf-jobs Demo - Background Job Processing");
    println!("==========================================\n");

    // Check if Redis is available
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    println!("📡 Connecting to Redis at: {}", redis_url);

    let manager = match QueueManager::new(&redis_url).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("❌ Failed to connect to Redis: {}", e);
            eprintln!("💡 Make sure Redis is running:");
            eprintln!("   docker run -d -p 6379:6379 redis");
            eprintln!("   or: brew services start redis");
            return Ok(());
        }
    };

    println!("✅ Connected to Redis successfully\n");

    // Dispatch example jobs
    println!("📤 Dispatching example jobs...\n");

    // 1. Email job
    let email_job = SendEmailJob {
        to: "user@example.com".to_string(),
        subject: "Welcome!".to_string(),
        body: "Thanks for signing up".to_string(),
    };
    let job_id = manager.dispatch(email_job).await?;
    println!("✉️  Email job dispatched: {}", job_id);

    // 2. Image processing job
    let image_job = ProcessImageJob {
        path: "/images/photo.jpg".to_string(),
        operations: vec![
            "resize".to_string(),
            "crop".to_string(),
            "watermark".to_string(),
        ],
    };
    let job_id = manager.dispatch(image_job).await?;
    println!("🖼️  Image processing job dispatched: {}", job_id);

    // 3. Report generation job
    let report_job = GenerateReportJob {
        report_type: "sales".to_string(),
        start_date: "2024-01-01".to_string(),
        end_date: "2024-01-31".to_string(),
    };
    let job_id = manager.dispatch(report_job).await?;
    println!("📊 Report generation job dispatched: {}", job_id);

    // 4. Delayed job (will be available after 10 seconds)
    let delayed_email = SendEmailJob {
        to: "delayed@example.com".to_string(),
        subject: "Delayed Message".to_string(),
        body: "This was sent 10 seconds later".to_string(),
    };
    let job_id = manager
        .dispatch_later(delayed_email, Duration::from_secs(10))
        .await?;
    println!("⏰ Delayed email job dispatched: {}", job_id);

    // 5. Failing job (will retry)
    let failing_job = FailingJob { fail_count: 2 };
    let job_id = manager.dispatch(failing_job).await?;
    println!("❌ Failing job dispatched: {} (will fail 2 times, then succeed)", job_id);

    println!("\n👷 Starting worker pool...\n");

    // Configure and start worker pool
    let config = WorkerConfig::default()
        .workers(2) // 2 concurrent workers
        .queues(&["default", "emails", "images", "reports"])
        .timeout(Duration::from_secs(60))
        .sleep(Duration::from_secs(1));

    let mut pool = WorkerPool::new(config, manager.clone()).await?;
    pool.start().await?;

    println!("✅ Worker pool started with 2 workers");
    println!("📋 Listening on queues: default, emails, images, reports\n");

    // Setup scheduler
    println!("📅 Setting up scheduler...\n");

    let mut scheduler = Scheduler::new(manager.clone());

    // Schedule cache cleanup every 2 minutes (for demo purposes)
    // In production: "0 */15 * * *" for every 15 minutes
    scheduler.schedule("*/2 * * * *", "cache-cleanup", || CacheCleanupJob)?;

    scheduler.start().await?;
    println!("✅ Scheduler started");
    println!("   - Cache cleanup: every 2 minutes\n");

    // Display queue status
    println!("📊 Queue Status:");
    let default_size = manager.size("default").await.unwrap_or(0);
    let emails_size = manager.size("emails").await.unwrap_or(0);
    let images_size = manager.size("images").await.unwrap_or(0);
    let reports_size = manager.size("reports").await.unwrap_or(0);

    println!("   - default: {} jobs", default_size);
    println!("   - emails:  {} jobs", emails_size);
    println!("   - images:  {} jobs", images_size);
    println!("   - reports: {} jobs", reports_size);

    println!("\n👀 Watching job processing...");
    println!("   Press Ctrl+C to stop\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    println!("\n🛑 Shutting down...");

    // Graceful shutdown
    scheduler.shutdown().await?;
    pool.shutdown().await?;

    println!("✅ Shutdown complete");
    println!("\n📈 Final Queue Status:");

    let default_size = manager.size("default").await.unwrap_or(0);
    let emails_size = manager.size("emails").await.unwrap_or(0);
    let failed_jobs = manager.failed_jobs().await.unwrap_or_default();

    println!("   - default: {} jobs remaining", default_size);
    println!("   - emails:  {} jobs remaining", emails_size);
    println!("   - failed:  {} jobs", failed_jobs.len());

    if !failed_jobs.is_empty() {
        println!("\n❌ Failed Jobs:");
        for failed in failed_jobs.iter().take(5) {
            println!(
                "   - Job {}: {} ({})",
                failed.payload.id, failed.payload.job_type, failed.error
            );
        }
    }

    Ok(())
}
