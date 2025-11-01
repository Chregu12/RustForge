# RustForge Queued Commands Guide

This guide explains how to dispatch commands to a queue for asynchronous execution, similar to Laravel's queue system for commands.

## Overview

The queued commands system allows you to:
- **Dispatch long-running operations** asynchronously
- **Delay command execution** until a specified time
- **Retry failed commands** automatically
- **Organize commands by queue** for different processing priorities
- **Track job progress** with unique job IDs
- **Add custom metadata** to jobs

## Basic Usage

### Simple Queue Dispatch

```rust
use foundry_api::queued_commands::{QueuedCommand, CommandQueue};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = CommandQueue::default();

    // Create a queued command
    let cmd = QueuedCommand::new("import:data")
        .with_args(vec!["users.csv".to_string()]);

    // Dispatch to queue
    let job_id = queue.dispatch(cmd).await?;
    println!("Job dispatched: {}", job_id);

    Ok(())
}
```

### With Delay

Delay execution by a specified duration:

```rust
use std::time::Duration;

let cmd = QueuedCommand::new("send:emails")
    .with_delay(Duration::from_secs(300)); // Delay 5 minutes

let job_id = queue.dispatch(cmd).await?;
```

### With Retry Configuration

Configure maximum retry attempts:

```rust
let cmd = QueuedCommand::new("sync:external-api")
    .with_max_attempts(5);  // Retry up to 5 times

let job_id = queue.dispatch(cmd).await?;
```

## Multiple Queues

### Using Different Queues

Organize commands by priority or type:

```rust
use foundry_api::queued_commands::{CommandQueue, QueuedCommand};

// High priority queue
let urgent = CommandQueue::new("urgent");
let cmd = QueuedCommand::new("alert:critical")
    .on_queue("urgent");

urgent.dispatch(cmd).await?;

// Background queue for slow tasks
let background = CommandQueue::new("background");
let cmd = QueuedCommand::new("report:generate")
    .on_queue("background");

background.dispatch(cmd).await?;
```

### Queue Manager

Manage multiple queues:

```rust
use foundry_api::queued_commands::{QueueManager, QueuedCommand};

let mut manager = QueueManager::new();
manager.add_queue("high");
manager.add_queue("normal");
manager.add_queue("low");

// Dispatch to specific queue
if let Some(queue) = manager.queue("high") {
    let cmd = QueuedCommand::new("urgent:task");
    queue.dispatch(cmd).await?;
}

// List all queues
let queues = manager.list();
println!("Available queues: {:?}", queues);
```

## Advanced Features

### Timeout Configuration

Set maximum execution time:

```rust
use std::time::Duration;

let cmd = QueuedCommand::new("data:process")
    .with_timeout(Duration::from_secs(600));  // 10 minutes max

queue.dispatch(cmd).await?;
```

### Custom Metadata

Attach custom data to jobs:

```rust
let cmd = QueuedCommand::new("export:users")
    .with_metadata("export_type", serde_json::Value::String("pdf".to_string()))
    .with_metadata("user_id", serde_json::Value::Number(123.into()));

let job_id = queue.dispatch(cmd).await?;
```

### Multiple Arguments

Pass multiple arguments to queued commands:

```rust
let cmd = QueuedCommand::new("migrate:database")
    .with_arg("--database=legacy")
    .with_arg("--steps=5")
    .with_arg("--force");

queue.dispatch(cmd).await?;
```

### Batch Dispatch

Queue multiple commands at once:

```rust
let commands = vec![
    QueuedCommand::new("clean:cache"),
    QueuedCommand::new("optimize:images"),
    QueuedCommand::new("generate:sitemap"),
];

let job_ids = queue.dispatch_many(commands).await?;
println!("Dispatched {} jobs", job_ids.len());
```

## In Command Context

### Queueing from Within Commands

```rust
use foundry_api::queued_commands::{QueuedCommand, CommandQueue};
use foundry_plugins::{CommandContext, CommandResult, CommandError};

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let queue = CommandQueue::default();

    // Dispatch a follow-up command to the queue
    let follow_up = QueuedCommand::new("cleanup:temp-files")
        .with_delay(std::time::Duration::from_secs(3600)); // 1 hour

    let job_id = queue.dispatch(follow_up).await
        .map_err(|e| CommandError::Message(e.to_string()))?;

    Ok(CommandResult::success(
        format!("Task queued with ID: {}", job_id)
    ))
}
```

### Job Result Tracking

```rust
use foundry_api::queued_commands::JobDispatch;

let job = JobDispatch::new(
    "job123".to_string(),
    "import:data".to_string(),
    "default".to_string(),
);

let job = job.with_scheduled_at(
    chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(1))
        .unwrap()
        .to_rfc3339()
);

println!("Job ID: {}", job.job_id);
println!("Scheduled for: {:?}", job.scheduled_at);
```

## Error Handling

### Handle Dispatch Errors

```rust
use foundry_api::queued_commands::{CommandQueue, QueuedCommand, QueueError};

let queue = CommandQueue::default();
let cmd = QueuedCommand::new("task:process");

match queue.dispatch(cmd).await {
    Ok(job_id) => println!("Dispatched: {}", job_id),
    Err(QueueError::InvalidCommand(msg)) => {
        eprintln!("Invalid command configuration: {}", msg);
    }
    Err(QueueError::QueueUnavailable(queue)) => {
        eprintln!("Queue '{}' is not available", queue);
    }
    Err(QueueError::DispatchFailed(msg)) => {
        eprintln!("Failed to dispatch: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Validation

Commands are validated before dispatch:

```rust
// ❌ This will fail
let cmd = QueuedCommand::new("")
    .with_max_attempts(0);

// ✅ This will succeed
let cmd = QueuedCommand::new("valid:command")
    .with_max_attempts(3);
```

## Real-World Examples

### Email Campaign

```rust
use foundry_api::queued_commands::{CommandQueue, QueuedCommand};
use std::time::Duration;

async fn queue_emails() -> Result<(), Box<dyn std::error::Error>> {
    let queue = CommandQueue::new("emails");

    for user_id in 1..=1000 {
        let delay = Duration::from_secs(5 * user_id as u64);  // Stagger sends

        let cmd = QueuedCommand::new("email:send")
            .with_arg(format!("--user={}", user_id))
            .with_delay(delay)
            .with_max_attempts(3);

        queue.dispatch(cmd).await?;
    }

    Ok(())
}
```

### Data Import Pipeline

```rust
async fn import_data_pipeline(
    files: Vec<String>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let queue = CommandQueue::new("imports");
    let mut job_ids = Vec::new();

    for (idx, file) in files.iter().enumerate() {
        let cmd = QueuedCommand::new("import:csv")
            .with_arg(file)
            .with_arg(format!("--batch={}", idx + 1))
            .with_timeout(std::time::Duration::from_secs(600))
            .with_max_attempts(5);

        let job_id = queue.dispatch(cmd).await?;
        job_ids.push(job_id);
    }

    Ok(job_ids)
}
```

### Scheduled Reports

```rust
async fn schedule_daily_reports(queue: &CommandQueue) -> Result<(), Box<dyn std::error::Error>> {
    let reports = vec!["sales", "inventory", "customer_activity"];

    for report_type in reports {
        let cmd = QueuedCommand::new("report:generate")
            .with_arg(format!("--type={}", report_type))
            .with_arg("--format=pdf")
            .with_metadata("report_type", serde_json::Value::String(report_type.to_string()))
            .with_max_attempts(2);

        queue.dispatch(cmd).await?;
    }

    Ok(())
}
```

## Best Practices

1. **Use meaningful command names**
   ```rust
   // Good
   QueuedCommand::new("invoice:send")
   QueuedCommand::new("backup:database")

   // Avoid
   QueuedCommand::new("run")
   QueuedCommand::new("task")
   ```

2. **Set appropriate timeouts for long operations**
   ```rust
   QueuedCommand::new("backup:large-database")
       .with_timeout(Duration::from_secs(3600))  // 1 hour
   ```

3. **Configure retry attempts based on operation**
   ```rust
   // External API call - retry more
   .with_max_attempts(5)

   // Critical operation - retry less
   .with_max_attempts(1)
   ```

4. **Use delays for staggering**
   ```rust
   // Prevent thundering herd
   let delay = Duration::from_millis(100 * user_id);
   cmd.with_delay(delay)
   ```

5. **Add metadata for tracking and debugging**
   ```rust
   cmd.with_metadata("user_id", serde_json::Value::Number(user_id.into()))
      .with_metadata("import_source", serde_json::Value::String("api".to_string()))
   ```

## Queue Processing Strategies

### Priority Queues

```rust
let urgent = CommandQueue::new("urgent");      // Process first
let normal = CommandQueue::new("normal");      // Process second
let background = CommandQueue::new("background"); // Process last
```

### Time-based Queues

```rust
// Immediate processing
let now = CommandQueue::new("now");

// Batch processing during off-hours
let batch = CommandQueue::new("batch");

// Long-running background jobs
let slow = CommandQueue::new("slow");
```

### Job-specific Queues

```rust
let email_queue = CommandQueue::new("email");
let report_queue = CommandQueue::new("reports");
let sync_queue = CommandQueue::new("sync");
```

## Comparison with Laravel

| Feature | Laravel | RustForge | Status |
|---------|---------|-----------|--------|
| Queue dispatch | ✓ | ✓ | ✅ |
| Delayed jobs | ✓ | ✓ | ✅ |
| Retry attempts | ✓ | ✓ | ✅ |
| Custom queues | ✓ | ✓ | ✅ |
| Metadata | ✓ | ✓ | ✅ |
| Job tracking | ✓ | ✓ | ✅ |
| Timeout | ✓ | ✓ | ✅ |

## Performance Considerations

- **Job ID generation**: Uses UUID v4 for uniqueness
- **Batch operations**: `dispatch_many()` for multiple commands
- **Async dispatch**: Non-blocking queue operations
- **Memory efficient**: Metadata stored as JSON
- **Scalable**: Works with distributed queue systems

## Testing

```rust
#[tokio::test]
async fn test_queue_dispatch() {
    let queue = CommandQueue::default();
    let cmd = QueuedCommand::new("test:command");

    let result = queue.dispatch(cmd).await;
    assert!(result.is_ok());
}

#[test]
fn test_queued_command_builder() {
    let cmd = QueuedCommand::new("email:send")
        .with_args(vec!["user@example.com".to_string()])
        .with_delay(Duration::from_secs(60))
        .with_max_attempts(3);

    assert_eq!(cmd.command, "email:send");
    assert_eq!(cmd.args.len(), 1);
    assert!(cmd.is_delayed());
}
```

## Troubleshooting

### Job Not Processing

Check:
- Queue is running/configured
- Job doesn't have errors in metadata
- Retry attempts haven't been exceeded

### Timeout Issues

```rust
// Increase timeout if needed
cmd.with_timeout(Duration::from_secs(1800))  // 30 minutes
```

### Queue Backlog

```rust
// Use priority queues
let urgent = CommandQueue::new("urgent");
urgent.dispatch(high_priority_cmd).await?;
```

## See Also

- [Programmatic Execution Guide](PROGRAMMATIC_EXECUTION_GUIDE.md)
- [Isolatable Commands Guide](ISOLATABLE_COMMANDS_GUIDE.md)
- [Event System Guide](PROGRAMMATIC_EXECUTION_GUIDE.md#5-event-system)
- [RustForge Documentation](https://github.com/Chregu12/RustForge)
