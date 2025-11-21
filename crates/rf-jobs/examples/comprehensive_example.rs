//! Comprehensive job system example
//!
//! Demonstrates all major features:
//! - Multiple job types
//! - Job registry
//! - Retry logic
//! - Delayed jobs
//! - Failed job handling
//!
//! Run with: cargo run --example comprehensive_example

use async_trait::async_trait;
use rf_jobs::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Job Type 1: Send Email
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
}

#[async_trait]
impl JobWithRegistry for SendEmailJob {
    fn job_type(&self) -> &'static str {
        "send_email"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("📧 Sending email to {}", self.to));
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        3
    }
}

// ============================================================================
// Job Type 2: Process Image
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessImageJob {
    image_url: String,
    resize: bool,
}

#[async_trait]
impl JobWithRegistry for ProcessImageJob {
    fn job_type(&self) -> &'static str {
        "process_image"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("🖼️  Processing image: {}", self.image_url));

        if self.resize {
            ctx.log("  - Resizing image");
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        5
    }

    fn base_backoff_seconds(&self) -> u64 {
        30
    }
}

// ============================================================================
// Job Type 3: Generate Report (with potential failure)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerateReportJob {
    report_id: String,
    should_fail: bool, // For testing retry logic
}

#[async_trait]
impl JobWithRegistry for GenerateReportJob {
    fn job_type(&self) -> &'static str {
        "generate_report"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("📊 Generating report: {}", self.report_id));

        if self.should_fail && ctx.attempt < 2 {
            ctx.log("  ⚠️  Report generation failed (simulated)");
            return Err(JobError::ExecutionFailed(
                "Report service temporarily unavailable".to_string(),
            ));
        }

        ctx.log("  ✅ Report generated successfully");
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        3
    }

    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Linear
    }

    async fn failed(&self, ctx: JobContext, error: &JobError) {
        ctx.log(&format!(
            "❌ Report generation permanently failed: {}",
            error
        ));
        // In production, you might:
        // - Send notification to admin
        // - Update database status
        // - Trigger cleanup
    }
}

// ============================================================================
// Job Type 4: Send Notification
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendNotificationJob {
    user_id: String,
    message: String,
}

#[async_trait]
impl JobWithRegistry for SendNotificationJob {
    fn job_type(&self) -> &'static str {
        "send_notification"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("🔔 Sending notification to user {}", self.user_id));
        ctx.log(&format!("   Message: {}", self.message));
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        2
    }
}

// ============================================================================
// Main Application
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n╔═══════════════════════════════════════════════╗");
    println!("║  RustForge Job System - Comprehensive Demo   ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    // 1. Create and configure job registry
    println!("📝 Step 1: Setting up job registry...");
    let mut registry = JobRegistry::new();
    registry.register::<SendEmailJob>("send_email");
    registry.register::<ProcessImageJob>("process_image");
    registry.register::<GenerateReportJob>("generate_report");
    registry.register::<SendNotificationJob>("send_notification");
    println!("   ✓ Registered 4 job types\n");

    // 2. Connect to Redis
    println!("🔌 Step 2: Connecting to Redis...");
    let manager = QueueManager::new("redis://localhost:6379").await?;
    println!("   ✓ Connected to redis://localhost:6379\n");

    // 3. Dispatch various jobs
    println!("🚀 Step 3: Dispatching jobs...\n");

    // Email jobs
    println!("   📧 Email jobs:");
    for i in 1..=3 {
        let job = SendEmailJob {
            to: format!("user{}@example.com", i),
            subject: format!("Welcome email {}", i),
        };
        let id = manager.dispatch(job).await?;
        println!("      ✓ Dispatched email job {}", id);
    }

    // Image processing jobs
    println!("\n   🖼️  Image processing jobs:");
    let images = vec![
        "https://example.com/img1.jpg",
        "https://example.com/img2.png",
    ];
    for (i, url) in images.iter().enumerate() {
        let job = ProcessImageJob {
            image_url: url.to_string(),
            resize: i % 2 == 0,
        };
        let id = manager.dispatch(job).await?;
        println!("      ✓ Dispatched image job {}", id);
    }

    // Report generation jobs (one will fail initially)
    println!("\n   📊 Report generation jobs:");
    let job = GenerateReportJob {
        report_id: "Q4-2024".to_string(),
        should_fail: true, // Will retry
    };
    let id = manager.dispatch(job).await?;
    println!("      ✓ Dispatched failing report job {} (will retry)", id);

    let job = GenerateReportJob {
        report_id: "Q3-2024".to_string(),
        should_fail: false,
    };
    let id = manager.dispatch(job).await?;
    println!("      ✓ Dispatched successful report job {}", id);

    // Delayed notification job
    println!("\n   🔔 Notification jobs:");
    let job = SendNotificationJob {
        user_id: "user123".to_string(),
        message: "Your report is ready!".to_string(),
    };
    let id = manager.dispatch_later(job, Duration::from_secs(2)).await?;
    println!("      ✓ Dispatched delayed notification job {}", id);
    println!("        (will run in 2 seconds)");

    // 4. Start worker pool
    println!("\n⚙️  Step 4: Starting worker pool...");
    let config = WorkerConfig::default()
        .workers(3)
        .queues(&["default"])
        .timeout(Duration::from_secs(30))
        .sleep(Duration::from_millis(100));

    let mut pool = WorkerPool::new(config, manager, registry).await?;
    println!("   ✓ Created pool with 3 workers");
    println!("   ✓ Listening on queue: default");
    println!("   ✓ Timeout: 30s\n");

    println!("🏃 Step 5: Processing jobs...\n");
    println!("─────────────────────────────────────────────────\n");

    pool.start().await?;

    // Let workers process jobs
    tokio::time::sleep(Duration::from_secs(8)).await;

    // 6. Graceful shutdown
    println!("\n─────────────────────────────────────────────────");
    println!("\n🛑 Step 6: Shutting down gracefully...");
    pool.shutdown().await?;
    println!("   ✓ All workers stopped\n");

    println!("╔═══════════════════════════════════════════════╗");
    println!("║           Demo completed successfully!        ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    println!("📝 Summary:");
    println!("   • Multiple job types registered and executed");
    println!("   • Retry logic demonstrated (failing report job)");
    println!("   • Delayed job execution shown (notification)");
    println!("   • Worker pool with 3 concurrent workers");
    println!("   • Graceful shutdown handling\n");

    Ok(())
}
