//! Comprehensive Horizon Dashboard example
//!
//! This example demonstrates how to set up and run the Horizon dashboard
//! for monitoring jobs and queues in a RustForge application.

use rf_horizon::Horizon;
use rf_jobs::QueueManager;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Horizon Dashboard Example ===\n");

    // Step 1: Create a QueueManager (connects to Redis)
    println!("1. Connecting to Redis...");
    let queue_manager = QueueManager::new("redis://localhost:6379").await?;
    println!("   Connected successfully!\n");

    // Step 2: Create Horizon instance with builder pattern
    println!("2. Configuring Horizon...");
    let horizon = Horizon::builder()
        .queue_manager(queue_manager.clone())
        .monitor_queue("default")
        .monitor_queue("emails")
        .monitor_queue("reports")
        .failed_job_retention_days(7)
        .metrics_retention_hours(48)
        .enable_dashboard(true)
        .build();

    println!("   Monitoring queues: default, emails, reports");
    println!("   Failed job retention: 7 days");
    println!("   Metrics retention: 48 hours\n");

    // Step 3: Start the dashboard server
    println!("3. Starting Horizon dashboard...");
    println!("   Dashboard will be available at: http://127.0.0.1:8080/horizon\n");
    println!("   API endpoints:");
    println!("   - GET  /horizon                    Dashboard homepage");
    println!("   - GET  /horizon/jobs               Jobs list");
    println!("   - GET  /horizon/failed             Failed jobs");
    println!("   - GET  /horizon/api/stats          Statistics");
    println!("   - GET  /horizon/api/jobs           Jobs list (JSON)");
    println!("   - POST /horizon/api/jobs/:id/retry Retry a job");
    println!("   - POST /horizon/api/failed/:id/retry Retry failed job");
    println!("   - POST /horizon/api/failed/batch-retry Retry multiple jobs\n");

    println!("Press Ctrl+C to stop the server\n");

    // Start the server (this blocks until shutdown)
    horizon
        .serve_with_queue_manager(queue_manager, "127.0.0.1:8080")
        .await?;

    Ok(())
}
