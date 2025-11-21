//! Basic scheduling example

use async_trait::async_trait;
use rf_scheduler::{Scheduler, Task, TaskBuilder};

// Example task: Database backup
struct BackupTask {
    name: String,
}

#[async_trait]
impl Task for BackupTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("[{}] Running database backup...", self.name);
        // Simulate backup work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("[{}] Backup completed!", self.name);
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Example task: Send email reports
struct EmailReportTask;

#[async_trait]
impl Task for EmailReportTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("[email-report] Sending weekly email reports...");
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        println!("[email-report] Reports sent!");
        Ok(())
    }

    fn name(&self) -> &str {
        "email-report"
    }
}

// Example task: Cleanup temp files
struct CleanupTask;

#[async_trait]
impl Task for CleanupTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("[cleanup] Cleaning up temporary files...");
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        println!("[cleanup] Cleanup completed!");
        Ok(())
    }

    fn name(&self) -> &str {
        "cleanup"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RustForge Task Scheduler Example ===\n");

    let scheduler = Scheduler::new();

    // Example 1: Schedule with cron expression
    println!("Scheduling tasks...\n");

    scheduler
        .schedule(
            "0 0 2 * * *", // Every day at 2 AM
            BackupTask {
                name: "daily-backup".to_string(),
            },
        )
        .await?;
    println!("✓ Scheduled daily backup at 2 AM");

    // Example 2: Use convenience methods
    scheduler.hourly(CleanupTask).await?;
    println!("✓ Scheduled cleanup to run every hour");

    scheduler
        .daily_at(
            "09:00",
            BackupTask {
                name: "morning-backup".to_string(),
            },
        )
        .await?;
    println!("✓ Scheduled morning backup at 9 AM");

    // Example 3: Use fluent API
    TaskBuilder::new()
        .at("09:00")
        .on("monday")
        .schedule(
            &scheduler,
            BackupTask {
                name: "monday-backup".to_string(),
            },
        )
        .await?;
    println!("✓ Scheduled Monday backup at 9 AM");

    TaskBuilder::new()
        .at("17:00")
        .weekdays()
        .schedule(&scheduler, EmailReportTask)
        .await?;
    println!("✓ Scheduled weekday reports at 5 PM");

    // Example 4: More fluent API examples
    scheduler.every_minutes(5, CleanupTask).await?;
    println!("✓ Scheduled cleanup every 5 minutes");

    // Note: This would normally call scheduler.start().await to run forever
    // For this example, we'll just show the configuration

    println!("\nTotal scheduled tasks: {}", scheduler.task_count().await);

    println!("\n=== Scheduler Configuration Complete ===");
    println!("In production, call scheduler.start().await to begin execution");

    Ok(())
}
