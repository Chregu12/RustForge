# RustForge Isolatable Commands Guide

This guide explains how to prevent concurrent execution of commands using the isolation system, similar to Laravel's `isolatable()` method.

## Overview

The isolatable commands system ensures that only one instance of a command can run at a time. This is useful for:
- **Long-running operations** - Database migrations, backups
- **Resource-intensive tasks** - Batch processing, reporting
- **State-modifying operations** - Cache clearing, queue processing
- **Scheduled tasks** - Prevent overlapping cron jobs

## Basic Usage

### File-based Locking (Default)

File-based locks work across multiple processes and are the recommended approach for production:

```rust
use foundry_api::isolatable::{CommandIsolation, LockStrategy};

async fn execute_isolated_command() -> Result<(), Box<dyn std::error::Error>> {
    let isolation = CommandIsolation::new("migrate");

    // Try to acquire lock
    match isolation.lock() {
        Ok(_guard) => {
            println!("Lock acquired, running command");
            // Your command logic here
            // Lock is automatically released when guard is dropped
            Ok(())
        }
        Err(e) => {
            eprintln!("Could not acquire lock: {}", e);
            Err(Box::new(e))
        }
    }
}
```

### Memory-based Locking

Memory locks are faster but only work within a single process:

```rust
use foundry_api::isolatable::{CommandIsolation, LockStrategy};

let isolation = CommandIsolation::new("cache:clear")
    .with_strategy(LockStrategy::Memory);

match isolation.lock() {
    Ok(_guard) => {
        println!("Lock acquired");
    }
    Err(e) => {
        eprintln!("Already running: {}", e);
    }
}
```

## Lock Timeout

### Default Timeout

Wait indefinitely for the lock:

```rust
let isolation = CommandIsolation::new("migrate");

let guard = isolation.lock()?;  // Waits forever
```

### Set Timeout

Timeout if lock is not acquired within specified time:

```rust
use std::time::Duration;

let isolation = CommandIsolation::new("migrate")
    .with_timeout(Duration::from_secs(300)); // 5 minutes

match isolation.lock_with_timeout(Duration::from_secs(60)) {
    Ok(_guard) => println!("Acquired lock"),
    Err(e) => eprintln!("Timeout: {}", e),
}
```

## Lock Strategies

### File-based Strategy (Default)

```rust
use foundry_api::isolatable::{CommandIsolation, LockStrategy};

let isolation = CommandIsolation::new("migrate")
    .with_strategy(LockStrategy::File)
    .with_lock_dir(".foundry/locks");

// Creates: .foundry/locks/migrate.lock
let guard = isolation.lock()?;
```

**Advantages:**
- ✅ Works across multiple processes
- ✅ Works across multiple machines (with shared filesystem)
- ✅ Survives process crashes
- ✅ Suitable for long-running tasks

**Disadvantages:**
- ⚠️ Slower than memory locks
- ⚠️ Requires filesystem access

### Memory-based Strategy

```rust
let isolation = CommandIsolation::new("cache:clear")
    .with_strategy(LockStrategy::Memory);

let guard = isolation.lock()?;
```

**Advantages:**
- ✅ Very fast
- ✅ No filesystem overhead
- ✅ Good for short operations

**Disadvantages:**
- ⚠️ Single process only
- ⚠️ Lost on process crash
- ⚠️ Not suitable for distributed setups

## Custom Lock Directory

Configure where lock files are stored:

```rust
use std::path::PathBuf;

let isolation = CommandIsolation::new("migrate")
    .with_lock_dir("/tmp/foundry-locks");

let guard = isolation.lock()?;
// Creates: /tmp/foundry-locks/migrate.lock
```

## Checking Lock Status

### Check if Locked

```rust
let isolation = CommandIsolation::new("migrate");

if isolation.is_locked() {
    println!("Command is already running");
} else {
    println!("Safe to run");
}
```

### Get Lock File Path

```rust
let lock_path = isolation.lock_path();
println!("Lock file: {}", lock_path.display());
```

## In Command Context

### Simple Isolated Command

```rust
use foundry_api::isolatable::CommandIsolation;
use foundry_plugins::{CommandContext, CommandResult, CommandError};

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let isolation = CommandIsolation::new("migrate");

    let _guard = isolation.lock().map_err(|e| {
        CommandError::Message(format!("Could not acquire lock: {}", e))
    })?;

    // Run your command
    println!("Running migration...");
    // ... migration logic ...
    println!("Migration complete!");

    Ok(CommandResult::success("Migration completed"))
}
```

### Command with Timeout

```rust
use foundry_api::isolatable::CommandIsolation;
use std::time::Duration;

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let isolation = CommandIsolation::new("backup");

    // Try to acquire lock with 10 minute timeout
    let _guard = isolation.lock_with_timeout(Duration::from_secs(600))
        .map_err(|e| CommandError::Message(e.to_string()))?;

    // Run backup...
    Ok(CommandResult::success("Backup completed"))
}
```

### Graceful Timeout Handling

```rust
use foundry_api::isolatable::{CommandIsolation, IsolationError};
use std::time::Duration;

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let isolation = CommandIsolation::new("cache:clear");

    match isolation.lock_with_timeout(Duration::from_secs(30)) {
        Ok(_guard) => {
            println!("Clearing cache...");
            Ok(CommandResult::success("Cache cleared"))
        }
        Err(IsolationError::Timeout { command, timeout }) => {
            eprintln!("Command '{}' is already running (timeout: {:?})", command, timeout);
            Ok(CommandResult::skipped(
                "Cache clear is already in progress"
            ))
        }
        Err(e) => {
            Err(CommandError::Message(e.to_string()))
        }
    }
}
```

## Error Handling

### Error Types

```rust
use foundry_api::isolatable::IsolationError;

match isolation.lock() {
    Ok(guard) => {
        // Success
    }
    Err(IsolationError::AlreadyRunning { command, locked_at }) => {
        eprintln!("Command '{}' is already running (locked at: {})",
            command, locked_at);
    }
    Err(IsolationError::Timeout { command, timeout }) => {
        eprintln!("Timeout waiting for '{}' after {:?}",
            command, timeout);
    }
    Err(IsolationError::LockFileError(msg)) => {
        eprintln!("Lock file error: {}", msg);
    }
    Err(IsolationError::PermissionDenied(msg)) => {
        eprintln!("Permission denied: {}", msg);
    }
    Err(IsolationError::IoError(msg)) => {
        eprintln!("IO error: {}", msg);
    }
    Err(other) => {
        eprintln!("Other error: {}", other);
    }
}
```

## Advanced Patterns

### Retry with Exponential Backoff

```rust
use foundry_api::isolatable::CommandIsolation;
use std::time::Duration;

async fn execute_with_retries(
    command: &str,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let isolation = CommandIsolation::new(command);
    let mut delay = Duration::from_millis(100);

    for attempt in 0..max_retries {
        match isolation.lock() {
            Ok(guard) => {
                println!("Acquired lock on attempt {}", attempt + 1);
                // Run command
                drop(guard);
                return Ok(());
            }
            Err(_) if attempt < max_retries - 1 => {
                println!("Lock not available, retrying in {:?}", delay);
                tokio::time::sleep(delay).await;
                delay = Duration::from_millis((delay.as_millis() as u64 * 2).min(30000));
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Err("Failed to acquire lock after retries".into())
}
```

### Lock with Cleanup

```rust
let isolation = CommandIsolation::new("data:import");

// Try lock
match isolation.lock() {
    Ok(guard) => {
        match run_import() {
            Ok(result) => {
                println!("Import successful: {:?}", result);
            }
            Err(e) => {
                eprintln!("Import failed: {}", e);
                // Lock is released here via guard drop
            }
        }
    }
    Err(e) => {
        // Clean up any partial state if needed
        eprintln!("Could not acquire lock: {}", e);
    }
}
```

### Monitoring Lock Status

```rust
use std::time::Duration;

async fn monitor_locks(commands: Vec<&str>) {
    let mut handles = vec![];

    for cmd in commands {
        let isolation = CommandIsolation::new(cmd)
            .with_strategy(LockStrategy::File);

        let handle = tokio::spawn(async move {
            loop {
                let locked = isolation.is_locked();
                let status = if locked { "🔒 Locked" } else { "🔓 Free" };
                println!("{}: {}", cmd, status);

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;
}
```

## Comparison with Laravel

| Feature | Laravel | RustForge | Status |
|---------|---------|-----------|--------|
| Isolatable | ✓ | ✓ | ✅ |
| File locks | ✓ | ✓ | ✅ |
| Timeout | ✓ | ✓ | ✅ |
| Lock directory | ✓ | ✓ | ✅ |
| Memory locks | ⚠️ | ✓ | ✅ Enhanced |
| Error details | ✓ | ✓ | ✅ |

## Best Practices

1. **Always use file-based locks for production**
   ```rust
   let isolation = CommandIsolation::new("migrate")
       .with_strategy(LockStrategy::File);
   ```

2. **Use reasonable timeouts for interactive commands**
   ```rust
   let isolation = CommandIsolation::new("cache:clear")
       .with_timeout(Duration::from_secs(60));
   ```

3. **Handle errors gracefully**
   ```rust
   match isolation.lock() {
       Ok(_) => { /* run command */ }
       Err(e) => { /* handle error */ }
   }
   ```

4. **Use guard for automatic cleanup**
   ```rust
   {
       let _guard = isolation.lock()?;
       // command runs
       // guard automatically dropped here
   }
   ```

5. **Create lock directory before use**
   ```rust
   let isolation = CommandIsolation::new("migrate")
       .with_lock_dir(".foundry/locks");
   ```

## Troubleshooting

### Lock File Not Cleaned Up

If lock files persist after command execution:

```rust
// Manual cleanup
isolation.release_all()?;
```

### Permission Denied Error

Ensure the lock directory is writable:

```bash
mkdir -p .foundry/locks
chmod 755 .foundry/locks
```

### Timeout Always Exceeded

Check if another instance is truly running:

```bash
ls -la .foundry/locks/
cat .foundry/locks/migrate.lock
```

## See Also

- [Programmatic Execution Guide](PROGRAMMATIC_EXECUTION_GUIDE.md)
- [Event System Guide](PROGRAMMATIC_EXECUTION_GUIDE.md#5-event-system)
- [Signal Handling in foundry-signal-handler](crates/foundry-signal-handler/)
