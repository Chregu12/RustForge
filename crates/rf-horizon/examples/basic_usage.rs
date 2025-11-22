//! Basic usage example for rf-horizon

use anyhow::Result;
use async_trait::async_trait;
use rf_horizon::{Batch, Chain, FailedJobHandler, Horizon, QueueMetrics};
use std::sync::Arc;

// Example job implementations
struct SendEmailJob {
    to: String,
    subject: String,
}

#[async_trait]
impl rf_horizon::batching::Job for SendEmailJob {
    async fn handle(&self) -> Result<()> {
        println!(
            "Sending email to {} with subject: {}",
            self.to, self.subject
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    fn name(&self) -> String {
        format!("SendEmailJob({})", self.to)
    }
}

struct ProcessPaymentJob {
    amount: f64,
}

#[async_trait]
impl rf_horizon::batching::Job for ProcessPaymentJob {
    async fn handle(&self) -> Result<()> {
        println!("Processing payment: ${}", self.amount);
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        Ok(())
    }

    fn name(&self) -> String {
        format!("ProcessPaymentJob(${})", self.amount)
    }
}

struct GenerateReportJob {
    report_type: String,
}

#[async_trait]
impl rf_horizon::batching::Job for GenerateReportJob {
    async fn handle(&self) -> Result<()> {
        println!("Generating {} report", self.report_type);
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        Ok(())
    }

    fn name(&self) -> String {
        format!("GenerateReportJob({})", self.report_type)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Horizon Queue Dashboard Demo ===\n");

    // Create Horizon instance using builder
    let horizon = Horizon::builder()
        .monitor_queue("default")
        .monitor_queue("emails")
        .monitor_queue("payments")
        .failed_job_retention_days(7)
        .build();

    println!("1. Job Batching Example");
    println!("------------------------");

    // Create a batch of email jobs
    let email_batch = Batch::new("welcome-emails")
        .jobs(vec![
            Arc::new(SendEmailJob {
                to: "user1@example.com".to_string(),
                subject: "Welcome!".to_string(),
            }),
            Arc::new(SendEmailJob {
                to: "user2@example.com".to_string(),
                subject: "Welcome!".to_string(),
            }),
            Arc::new(SendEmailJob {
                to: "user3@example.com".to_string(),
                subject: "Welcome!".to_string(),
            }),
        ])
        .then(|batch| {
            println!("✓ Batch '{}' completed successfully!", batch.name);
        })
        .catch(|batch, error| {
            eprintln!("✗ Batch '{}' had an error: {}", batch.name, error);
        });

    let handle = email_batch.dispatch().await?;

    // Track progress
    while !handle.is_finished().await {
        let progress = handle.progress().await;
        println!("  Progress: {:.0}%", progress * 100.0);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    let final_status = handle.status().await;
    println!("  Final status: {:?}\n", final_status.status);

    // Record batch in Horizon
    horizon
        .record_batch(handle.id().to_string(), final_status)
        .await;

    println!("2. Job Chaining Example");
    println!("-----------------------");

    // Create a chain of jobs
    let chain = Chain::new()
        .job(Arc::new(ProcessPaymentJob { amount: 99.99 }))
        .then(Arc::new(SendEmailJob {
            to: "customer@example.com".to_string(),
            subject: "Payment Receipt".to_string(),
        }))
        .then(Arc::new(GenerateReportJob {
            report_type: "Sales".to_string(),
        }));

    println!("  Executing {} jobs in sequence...", chain.len());
    let chain_handle = chain.dispatch().await?;
    chain_handle.wait().await;
    println!(
        "  ✓ Chain completed: {}/{} jobs\n",
        chain_handle.completed().await,
        chain_handle.total()
    );

    println!("3. Failed Job Handling Example");
    println!("------------------------------");

    // Create failed job handler
    let failed_handler = FailedJobHandler::new().with_max_retries(3);

    // Simulate a failed job
    let failed_job = rf_horizon::FailedJob::new(
        "emails",
        "SendEmailJob",
        r#"{"to": "failed@example.com"}"#,
        "SMTP connection timeout",
    );

    failed_handler.record(failed_job.clone()).await;
    horizon.record_failed_job(failed_job).await;

    println!("  Failed jobs count: {}", failed_handler.count().await);
    println!("  Retrying failed job...");

    let failed_jobs = failed_handler.all().await;
    if let Some(job) = failed_jobs.first() {
        failed_handler.retry(&job.id).await?;
        println!("  ✓ Retry initiated\n");
    }

    println!("4. Queue Metrics Example");
    println!("------------------------");

    // Update queue metrics
    let mut metrics = QueueMetrics::new("emails");
    metrics.record_success(45.5);
    metrics.record_success(32.1);
    metrics.record_success(51.3);
    metrics.set_pending(12);

    println!("  Queue: {}", metrics.queue_name);
    println!("  Jobs Processed: {}", metrics.jobs_processed);
    println!("  Jobs Pending: {}", metrics.jobs_pending);
    println!(
        "  Avg Processing Time: {:.2}ms",
        metrics.average_processing_time_ms
    );
    println!("  Success Rate: {:.1}%", metrics.success_rate() * 100.0);

    horizon.update_metrics("emails".to_string(), metrics).await;

    println!("\n5. Dashboard Server");
    println!("-------------------");
    println!("To start the dashboard, run:");
    println!("  horizon.serve(\"0.0.0.0:8080\").await?;");
    println!("Then visit: http://localhost:8080");

    Ok(())
}
