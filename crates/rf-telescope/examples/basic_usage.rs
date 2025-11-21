//! Basic usage example for rf-telescope

use rf_telescope::{
    Telescope,
    watchers::{
        cache::{CacheWatcher, CacheInfo},
        request::{RequestWatcher, RequestInfo},
        query::{QueryWatcher, QueryInfo},
        exception::{ExceptionWatcher, ExceptionInfo},
        job::{JobWatcher, JobInfo},
        mail::{MailWatcher, MailInfo},
    },
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Telescope Debugging Dashboard Demo ===\n");

    // Create Telescope instance
    let telescope = Telescope::new()
        .watch_requests()
        .watch_queries()
        .watch_exceptions()
        .watch_jobs()
        .watch_mail()
        .enabled_in_production(false)
        .retention_hours(24);

    println!("1. Request Monitoring Example");
    println!("-----------------------------");

    let request_watcher = RequestWatcher::new(telescope.storage().clone());

    // Record some HTTP requests
    request_watcher.record(
        RequestInfo::new("GET", "/api/users", "192.168.1.100")
            .with_status(200)
            .with_duration(45)
            .with_header("User-Agent", "Mozilla/5.0")
            .with_user("user-123")
    ).await;

    request_watcher.record(
        RequestInfo::new("POST", "/api/login", "192.168.1.101")
            .with_status(201)
            .with_duration(120)
    ).await;

    request_watcher.record(
        RequestInfo::new("GET", "/api/reports", "192.168.1.102")
            .with_status(200)
            .with_duration(1850)  // Slow request
    ).await;

    let requests = request_watcher.all().await;
    println!("  Total requests recorded: {}", requests.len());

    let slow_requests = request_watcher.slow_requests(1000).await;
    println!("  Slow requests (>1000ms): {}", slow_requests.len());

    println!();

    println!("2. Query Monitoring Example");
    println!("---------------------------");

    let query_watcher = QueryWatcher::new(telescope.storage().clone())
        .with_slow_threshold(100.0);

    // Record database queries
    query_watcher.record(
        QueryInfo::new("SELECT * FROM users WHERE id = ?", "postgres")
            .with_binding("123")
            .with_duration(15.5)
    ).await;

    query_watcher.record(
        QueryInfo::new("SELECT * FROM orders WHERE user_id = ? AND status = ?", "postgres")
            .with_binding("123")
            .with_binding("completed")
            .with_duration(45.2)
    ).await;

    query_watcher.record(
        QueryInfo::new("SELECT * FROM large_table ORDER BY created_at DESC", "postgres")
            .with_duration(850.3)  // Slow query
    ).await;

    let queries = query_watcher.all().await;
    println!("  Total queries recorded: {}", queries.len());

    let slow_queries = query_watcher.slow_queries().await;
    println!("  Slow queries (>100ms): {}", slow_queries.len());

    let stats = query_watcher.statistics().await;
    println!("  Average query time: {:.2}ms", stats.average_duration_ms);
    println!("  Max query time: {:.2}ms", stats.max_duration_ms);

    println!();

    println!("3. Exception Tracking Example");
    println!("------------------------------");

    let exception_watcher = ExceptionWatcher::new(telescope.storage().clone());

    // Record exceptions
    exception_watcher.record(
        ExceptionInfo::new("DatabaseError", "Connection pool exhausted")
            .with_location("db/connection.rs", 42)
            .add_stack_line("at db::pool::get_connection")
            .add_stack_line("at api::users::get_user")
            .add_stack_line("at main::handle_request")
            .with_context("pool_size", "10")
            .with_context("active_connections", "10")
            .with_request("/api/users/123")
    ).await;

    exception_watcher.record(
        ExceptionInfo::new("ValidationError", "Invalid email format")
            .with_location("validation/email.rs", 15)
            .with_request("/api/register")
    ).await;

    let exceptions = exception_watcher.all().await;
    println!("  Total exceptions recorded: {}", exceptions.len());

    let db_errors = exception_watcher.by_type("DatabaseError").await;
    println!("  Database errors: {}", db_errors.len());

    println!();

    println!("4. Job Monitoring Example");
    println!("-------------------------");

    let job_watcher = JobWatcher::new(telescope.storage().clone());

    // Record jobs
    job_watcher.record(
        JobInfo::new("SendWelcomeEmail", "emails")
            .with_payload(json!({"to": "user@example.com"}))
            .processing()
            .completed()
    ).await;

    job_watcher.record(
        JobInfo::new("ProcessPayment", "payments")
            .with_payload(json!({"amount": 99.99, "currency": "USD"}))
            .processing()
            .failed("Payment gateway timeout")
    ).await;

    let jobs = job_watcher.all().await;
    println!("  Total jobs recorded: {}", jobs.len());

    let failed_jobs = job_watcher.failed_jobs().await;
    println!("  Failed jobs: {}", failed_jobs.len());

    println!();

    println!("5. Mail Preview Example");
    println!("-----------------------");

    let mail_watcher = MailWatcher::new(telescope.storage().clone());

    // Record sent emails
    mail_watcher.record(
        MailInfo::new("noreply@example.com", "Welcome to Our Platform!")
            .to("user@example.com")
            .with_html("<h1>Welcome!</h1><p>Thanks for signing up.</p>")
            .with_text("Welcome! Thanks for signing up.")
    ).await;

    mail_watcher.record(
        MailInfo::new("invoices@example.com", "Your Invoice")
            .to("customer@example.com")
            .cc("accounting@example.com")
            .with_html("<h1>Invoice #12345</h1>")
            .with_attachment("invoice.pdf", "application/pdf", 25600)
    ).await;

    let emails = mail_watcher.all().await;
    println!("  Total emails recorded: {}", emails.len());

    let with_attachments = mail_watcher.with_attachments().await;
    println!("  Emails with attachments: {}", with_attachments.len());

    println!();

    println!("6. Cache Monitoring Example");
    println!("----------------------------");

    let cache_watcher = CacheWatcher::new(telescope.storage().clone());

    // Record cache operations
    cache_watcher.record(CacheInfo::hit("user:123", "redis")).await;
    cache_watcher.record(CacheInfo::miss("user:456", "redis")).await;
    cache_watcher.record(
        CacheInfo::set("session:abc", "redis")
            .with_value("session_data")
            .with_ttl(3600)
    ).await;
    cache_watcher.record(CacheInfo::hit("user:123", "redis")).await;
    cache_watcher.record(CacheInfo::hit("product:789", "redis")).await;

    let cache_stats = cache_watcher.statistics().await;
    println!("  Total cache operations: {}", cache_stats.total_operations);
    println!("  Cache hits: {}", cache_stats.hits);
    println!("  Cache misses: {}", cache_stats.misses);
    println!("  Hit rate: {:.2}%", cache_stats.hit_rate);

    println!();

    println!("7. Dashboard Summary");
    println!("--------------------");
    println!("  Total entries: {}", telescope.storage().count(None).await);
    println!("  - Requests: {}", telescope.storage().count(Some(rf_telescope::EntryType::Request)).await);
    println!("  - Queries: {}", telescope.storage().count(Some(rf_telescope::EntryType::Query)).await);
    println!("  - Exceptions: {}", telescope.storage().count(Some(rf_telescope::EntryType::Exception)).await);
    println!("  - Cache: {}", telescope.storage().count(Some(rf_telescope::EntryType::Cache)).await);
    println!("  - Jobs: {}", telescope.storage().count(Some(rf_telescope::EntryType::Job)).await);
    println!("  - Mail: {}", telescope.storage().count(Some(rf_telescope::EntryType::Mail)).await);

    println!();
    println!("8. Dashboard Server");
    println!("-------------------");
    println!("To start the dashboard, run:");
    println!("  telescope.serve(\"0.0.0.0:8090\").await?;");
    println!("Then visit: http://localhost:8090");

    Ok(())
}
