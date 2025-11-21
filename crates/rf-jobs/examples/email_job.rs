//! Email job example
//!
//! This example demonstrates how to create and dispatch an email job
//! using the job registry system.
//!
//! Run with: cargo run --example email_job

use async_trait::async_trait;
use rf_jobs::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Send email job
///
/// This job sends an email to a recipient. In a real application,
/// this would integrate with an email service like SendGrid or AWS SES.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait]
impl JobWithRegistry for SendEmailJob {
    fn job_type(&self) -> &'static str {
        "send_email"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Sending email to: {}", self.to));
        ctx.log(&format!("Subject: {}", self.subject));

        // Simulate email sending
        tokio::time::sleep(Duration::from_millis(500)).await;

        // In production, you would:
        // - Validate email address
        // - Connect to email service
        // - Send the email
        // - Handle errors appropriately

        ctx.log("Email sent successfully!");
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        5 // Email sending might fail temporarily
    }

    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Exponential
    }

    fn base_backoff_seconds(&self) -> u64 {
        60 // Wait 1 minute before retrying
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Email Job Example ===\n");

    // 1. Create job registry and register job types
    let mut registry = JobRegistry::new();
    registry.register::<SendEmailJob>("send_email");
    println!("✓ Registered SendEmailJob\n");

    // 2. Create queue manager
    let manager = QueueManager::new("redis://localhost:6379").await?;
    println!("✓ Connected to Redis\n");

    // 3. Create some example jobs
    let jobs = vec![
        SendEmailJob {
            to: "alice@example.com".to_string(),
            subject: "Welcome to RustForge!".to_string(),
            body: "Thank you for joining us.".to_string(),
        },
        SendEmailJob {
            to: "bob@example.com".to_string(),
            subject: "Your order has shipped".to_string(),
            body: "Your order #12345 is on its way!".to_string(),
        },
        SendEmailJob {
            to: "charlie@example.com".to_string(),
            subject: "Password reset request".to_string(),
            body: "Click here to reset your password.".to_string(),
        },
    ];

    // 4. Dispatch jobs
    println!("Dispatching {} email jobs...", jobs.len());
    for job in jobs {
        let job_id = manager.dispatch(job).await?;
        println!("  ✓ Dispatched job {}", job_id);
    }
    println!();

    // 5. Start worker pool to process jobs
    let config = WorkerConfig::default()
        .workers(2)
        .queues(&["default"])
        .timeout(Duration::from_secs(10));

    let mut pool = WorkerPool::new(config, manager, registry).await?;
    println!("✓ Created worker pool with 2 workers\n");

    println!("Starting workers... (Press Ctrl+C to stop)\n");
    pool.start().await?;

    // Keep running for a bit to process jobs
    tokio::time::sleep(Duration::from_secs(5)).await;

    println!("\nShutting down...");
    pool.shutdown().await?;
    println!("✓ Workers stopped");

    Ok(())
}
