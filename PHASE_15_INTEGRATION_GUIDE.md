# Phase 15: Monitoring & Debugging Integration Guide

This guide shows how to integrate rf-horizon and rf-telescope into a complete RustForge application.

## Complete Application Example

```rust
use axum::{Router, routing::get};
use rf_horizon::Horizon;
use rf_telescope::Telescope;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize monitoring systems
    let horizon = Horizon::new()
        .monitor_queue("default")
        .monitor_queue("emails")
        .monitor_queue("payments")
        .failed_job_retention_days(7);

    let telescope = Telescope::new()
        .watch_requests()
        .watch_queries()
        .watch_exceptions()
        .watch_jobs()
        .watch_mail()
        .enabled_in_production(false);

    // Clone for use in different contexts
    let horizon_clone = horizon.clone();
    let telescope_clone = telescope.clone();

    // Start dashboards on separate ports
    tokio::spawn(async move {
        horizon_clone.serve("0.0.0.0:8080").await
    });

    tokio::spawn(async move {
        telescope_clone.serve("0.0.0.0:8090").await
    });

    // Build your application
    let app = Router::new()
        .route("/", get(|| async { "Hello, RustForge!" }))
        // Add telescope middleware to track requests
        .layer(telescope.middleware());

    // Start main application
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Application running on http://localhost:3000");
    println!("Horizon dashboard: http://localhost:8080");
    println!("Telescope dashboard: http://localhost:8090");

    axum::serve(listener, app).await?;
    Ok(())
}
```

## Dashboard Access

Once running, you can access:

1. **Main Application**: http://localhost:3000
2. **Horizon (Queue Monitoring)**: http://localhost:8080
3. **Telescope (Debugging)**: http://localhost:8090

## Using Horizon for Queue Management

### 1. Batch Processing Example

```rust
use rf_horizon::{Batch, batching::Job};
use async_trait::async_trait;

// Define your job
struct SendNotificationJob {
    user_id: String,
    message: String,
}

#[async_trait]
impl Job for SendNotificationJob {
    async fn handle(&self) -> anyhow::Result<()> {
        // Your job logic here
        send_notification(&self.user_id, &self.message).await?;
        Ok(())
    }

    fn name(&self) -> String {
        format!("SendNotificationJob({})", self.user_id)
    }
}

// Batch multiple notifications
async fn send_bulk_notifications(user_ids: Vec<String>) -> anyhow::Result<()> {
    let jobs: Vec<Arc<dyn Job>> = user_ids
        .into_iter()
        .map(|id| Arc::new(SendNotificationJob {
            user_id: id,
            message: "Important update!".to_string(),
        }) as Arc<dyn Job>)
        .collect();

    let batch = Batch::new("bulk-notifications")
        .jobs(jobs)
        .then(|batch| {
            println!("✓ All {} notifications sent!", batch.total_jobs);
        })
        .catch(|batch, error| {
            eprintln!("✗ Notification failed: {}", error);
        })
        .dispatch()
        .await?;

    // Wait for completion
    let status = batch.wait().await;
    println!("Batch completed with status: {:?}", status.status);

    Ok(())
}
```

### 2. Job Chaining Example

```rust
use rf_horizon::Chain;

async fn process_order(order_id: String) -> anyhow::Result<()> {
    Chain::new()
        .job(Arc::new(ValidateOrderJob { order_id: order_id.clone() }))
        .then(Arc::new(ChargePaymentJob { order_id: order_id.clone() }))
        .then(Arc::new(SendConfirmationJob { order_id: order_id.clone() }))
        .then(Arc::new(UpdateInventoryJob { order_id: order_id.clone() }))
        .dispatch()
        .await?;

    Ok(())
}
```

### 3. Failed Job Management

```rust
use rf_horizon::{FailedJobHandler, RetryStrategy};

async fn setup_failed_job_handler() -> FailedJobHandler {
    FailedJobHandler::new()
        .with_retry_strategy(RetryStrategy::Exponential {
            base_delay_seconds: 60
        })
        .with_max_retries(3)
}

// In your error handler
async fn handle_job_failure(
    handler: &FailedJobHandler,
    job_name: &str,
    error: &str,
) {
    let failed_job = rf_horizon::FailedJob::new(
        "default",
        job_name,
        "{}",  // Job payload
        error
    );

    handler.record(failed_job).await;
}

// Retry all failed jobs for a queue
async fn retry_failed_queue(handler: &FailedJobHandler) -> anyhow::Result<()> {
    let retried = handler.retry_all("emails").await?;
    println!("Retried {} failed jobs", retried);
    Ok(())
}
```

## Using Telescope for Debugging

### 1. Request Tracking

```rust
use rf_telescope::watchers::request::{RequestWatcher, RequestInfo};
use axum::{extract::State, middleware::Next, http::Request};

// Middleware to track requests
async fn telescope_request_middleware(
    State(watcher): State<Arc<RequestWatcher>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let ip = req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let response = next.run(req).await;

    let duration = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Record the request
    watcher.record(
        RequestInfo::new(method, path, ip)
            .with_status(status)
            .with_duration(duration)
    ).await;

    response
}
```

### 2. Query Tracking

```rust
use rf_telescope::watchers::query::{QueryWatcher, QueryInfo};

async fn track_query(
    watcher: &QueryWatcher,
    sql: &str,
    bindings: Vec<String>,
    duration_ms: f64,
) {
    watcher.record(
        QueryInfo::new(sql, "postgres")
            .with_bindings(bindings)
            .with_duration(duration_ms)
    ).await;
}

// Integration with your DB layer
async fn execute_query(
    watcher: &QueryWatcher,
    query: &str,
    params: Vec<String>,
) -> Result<Vec<Row>> {
    let start = std::time::Instant::now();
    let result = database.execute(query, params.clone()).await?;
    let duration = start.elapsed().as_secs_f64() * 1000.0;

    track_query(watcher, query, params, duration).await;

    Ok(result)
}
```

### 3. Exception Tracking

```rust
use rf_telescope::watchers::exception::{ExceptionWatcher, ExceptionInfo};

async fn track_exception(
    watcher: &ExceptionWatcher,
    error: &anyhow::Error,
    request_path: Option<&str>,
) {
    let mut exception = ExceptionInfo::new(
        "ApplicationError",
        error.to_string()
    );

    if let Some(path) = request_path {
        exception = exception.with_request(path);
    }

    // Add stack trace if available
    if let Some(backtrace) = error.backtrace() {
        let trace = format!("{:?}", backtrace);
        for line in trace.lines().take(10) {
            exception = exception.add_stack_line(line);
        }
    }

    watcher.record(exception).await;
}

// Error handler middleware
async fn error_handler(
    State(watcher): State<Arc<ExceptionWatcher>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    match next.run(req).await {
        Ok(response) => response,
        Err(error) => {
            track_exception(&watcher, &error, Some(&path)).await;
            // Return error response
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}
```

### 4. Job Monitoring

```rust
use rf_telescope::watchers::job::{JobWatcher, JobInfo, JobStatus};
use serde_json::json;

async fn track_job_execution(
    watcher: &JobWatcher,
    job_name: &str,
    queue: &str,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    // Create job entry
    let mut job = JobInfo::new(job_name, queue)
        .with_payload(payload)
        .processing();

    // Execute job
    match execute_job(&job).await {
        Ok(_) => {
            job = job.completed();
            watcher.record(job).await;
            Ok(())
        }
        Err(e) => {
            job = job.failed(e.to_string());
            watcher.record(job).await;
            Err(e)
        }
    }
}
```

### 5. Mail Preview

```rust
use rf_telescope::watchers::mail::{MailWatcher, MailInfo};

async fn send_and_track_email(
    watcher: &MailWatcher,
    mailer: &Mailer,
    to: &str,
    subject: &str,
    html: &str,
) -> anyhow::Result<()> {
    // Send email
    mailer.send(to, subject, html).await?;

    // Track in telescope
    watcher.record(
        MailInfo::new("noreply@example.com", subject)
            .to(to)
            .with_html(html)
    ).await;

    Ok(())
}
```

## Complete Integration Example

Here's a complete example showing both systems working together:

```rust
use axum::{Router, routing::post, extract::State, Json};
use rf_horizon::{Horizon, Batch, batching::Job};
use rf_telescope::{Telescope, watchers::*};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct BulkEmailRequest {
    user_ids: Vec<String>,
    subject: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct BulkEmailResponse {
    batch_id: String,
    total_jobs: usize,
}

struct AppState {
    horizon: Horizon,
    telescope: Telescope,
}

async fn send_bulk_emails(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BulkEmailRequest>,
) -> Json<BulkEmailResponse> {
    // Create jobs
    let jobs: Vec<Arc<dyn Job>> = request.user_ids
        .iter()
        .map(|id| Arc::new(SendEmailJob {
            user_id: id.clone(),
            subject: request.subject.clone(),
            body: request.body.clone(),
        }) as Arc<dyn Job>)
        .collect();

    let total_jobs = jobs.len();

    // Create batch with Horizon
    let batch = Batch::new("bulk-emails")
        .jobs(jobs)
        .then(|batch| {
            println!("✓ Sent {} emails successfully", batch.total_jobs);
        })
        .catch(|batch, error| {
            eprintln!("✗ Email batch error: {}", error);
        })
        .dispatch()
        .await
        .unwrap();

    let batch_id = batch.id().to_string();

    // Record batch in Horizon
    state.horizon.record_batch(
        batch_id.clone(),
        batch.status().await
    ).await;

    // Track in Telescope
    let job_watcher = job::JobWatcher::new(state.telescope.storage().clone());
    job_watcher.record(
        job::JobInfo::new("BulkEmailBatch", "emails")
            .with_payload(serde_json::json!({
                "batch_id": batch_id,
                "count": total_jobs
            }))
            .processing()
    ).await;

    Json(BulkEmailResponse {
        batch_id,
        total_jobs,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        horizon: Horizon::new()
            .monitor_queue("emails")
            .failed_job_retention_days(7),
        telescope: Telescope::new()
            .watch_requests()
            .watch_jobs()
            .watch_mail(),
    });

    // Start dashboards
    let horizon = state.horizon.clone();
    tokio::spawn(async move {
        horizon.serve("0.0.0.0:8080").await
    });

    let telescope = state.telescope.clone();
    tokio::spawn(async move {
        telescope.serve("0.0.0.0:8090").await
    });

    // Build app
    let app = Router::new()
        .route("/api/emails/bulk", post(send_bulk_emails))
        .with_state(state)
        .layer(telescope.middleware());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://localhost:3000");
    println!("Horizon: http://localhost:8080");
    println!("Telescope: http://localhost:8090");

    axum::serve(listener, app).await?;
    Ok(())
}
```

## Best Practices

### Horizon
1. **Batch Jobs**: Use batches for related jobs that should be tracked together
2. **Chain Jobs**: Use chains for dependent sequential operations
3. **Failed Jobs**: Always configure retry strategies appropriate for your use case
4. **Metrics**: Regularly update queue metrics for accurate monitoring
5. **Retention**: Set appropriate retention periods to prevent memory leaks

### Telescope
1. **Production**: Disable in production or use with caution
2. **Sensitive Data**: Avoid logging passwords, tokens, or PII
3. **Performance**: Set retention periods to limit memory usage
4. **Security**: Protect dashboards with authentication
5. **Sampling**: In high-traffic scenarios, consider sampling requests

## Monitoring Strategy

### Development
- Enable both Horizon and Telescope
- Monitor all queues and events
- Use dashboards for debugging

### Staging
- Enable both with production-like configuration
- Test monitoring overhead
- Validate dashboard performance

### Production
- Enable Horizon for queue monitoring
- Disable or limit Telescope
- Set strict retention policies
- Monitor resource usage
- Protect dashboard access

## Conclusion

rf-horizon and rf-telescope provide comprehensive monitoring and debugging capabilities for RustForge applications. Use them together for complete visibility into your application's behavior.
