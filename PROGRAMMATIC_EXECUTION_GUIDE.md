# RustForge Programmatic Command Execution Guide

This guide demonstrates how to execute RustForge commands programmatically from Rust code, similar to Laravel's `Artisan::call()` method.

## Overview

RustForge now provides multiple ways to execute commands programmatically:

1. **Artisan Facade** - High-level, Laravel-like API
2. **Event System** - Hook into command lifecycle events
3. **Command Chaining** - Execute multiple commands in sequence
4. **Existing Signal Handling** - Handle OS signals gracefully

## 1. Basic Artisan Facade Usage

### Simple Command Execution

```rust
use foundry_api::Artisan;
use foundry_application::FoundryApp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = FoundryApp::new(config)?;
    let invoker = FoundryInvoker::new(app);
    let artisan = Artisan::new(invoker);

    // Execute a simple command
    let result = artisan.call("list").dispatch().await?;

    println!("Status: {:?}", result.status);
    println!("Message: {}", result.message.unwrap_or_default());

    Ok(())
}
```

### Command with Arguments

```rust
use foundry_api::Artisan;

async fn create_command(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    // Create a new command with arguments
    let result = artisan
        .call("make:command")
        .with_args(vec!["TestCommand".to_string()])
        .dispatch()
        .await?;

    println!("Command created: {}", result.message.unwrap_or_default());
    Ok(())
}
```

### Using Response Formats

```rust
use foundry_api::Artisan;
use foundry_plugins::ResponseFormat;

async fn get_json_output(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    let result = artisan
        .call("list")
        .with_format(ResponseFormat::Json)
        .dispatch()
        .await?;

    // Work with JSON output
    if let Some(data) = result.data {
        println!("JSON Output: {}", data);
    }

    Ok(())
}
```

## 2. Command Chaining

Execute multiple commands in sequence:

```rust
use foundry_api::Artisan;

async fn run_migrations(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    let results = artisan
        .chain()
        .add("migrate")
        .add_with_args("seed:run", vec!["--class".to_string(), "DatabaseSeeder".to_string()])
        .add("cache:clear")
        .dispatch()
        .await?;

    println!("Executed {} commands", results.len());
    for (idx, result) in results.iter().enumerate() {
        println!("Command {}: {:?}", idx, result.status);
    }

    Ok(())
}
```

### Handling Chain Errors

```rust
async fn chain_with_error_handling(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    let chain_result = artisan
        .chain()
        .stop_on_error(true) // Default behavior
        .add("migrate")
        .add("seed:run")
        .dispatch()
        .await;

    match chain_result {
        Ok(results) => {
            println!("All commands executed successfully!");
            for result in results {
                println!("Result: {:?}", result.status);
            }
        }
        Err((failed_idx, err, completed)) => {
            println!("Chain failed at command index {}: {}", failed_idx, err);
            println!("Completed commands: {}", completed.len());
        }
    }

    Ok(())
}
```

## 3. Output Capture

```rust
use foundry_api::Artisan;

async fn capture_output(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    // Execute commands
    artisan.call("migrate").dispatch().await?;
    artisan.call("seed:run").dispatch().await?;

    // Get captured output
    let output = artisan.output();
    println!("All output lines: {:?}", output);

    // Get as single string
    let output_str = artisan.output_string();
    println!("Full output:\n{}", output_str);

    // Clear for next batch
    artisan.clear_output();

    Ok(())
}
```

## 4. Event System

### Listening to Command Events

```rust
use foundry_api::{EventDispatcher, CommandEvent};

async fn listen_to_events(dispatcher: &EventDispatcher) -> Result<(), Box<dyn std::error::Error>> {
    let mut rx = dispatcher.subscribe();

    // Spawn a task to listen for events
    let dispatcher_clone = dispatcher.clone();
    tokio::spawn(async move {
        let mut rx = dispatcher_clone.subscribe();
        while let Ok(event) = rx.recv().await {
            match event {
                CommandEvent::Starting(e) => {
                    println!("📍 Command starting: {}", e.command);
                }
                CommandEvent::Finished(e) => {
                    println!("✅ Command finished: {} ({}ms)", e.command, e.duration_ms);
                }
                CommandEvent::Failed(e) => {
                    println!("❌ Command failed: {} (error: {})", e.command, e.error_code);
                }
            }
        }
    });

    Ok(())
}
```

### Using Event-Dispatching Invoker

```rust
use foundry_api::{FoundryInvoker, EventDispatcher, EventDispatchingInvoker};
use foundry_api::invocation::CommandInvoker;

async fn setup_event_dispatching(
    invoker: FoundryInvoker,
) -> Result<Box<dyn CommandInvoker>, Box<dyn std::error::Error>> {
    let dispatcher = EventDispatcher::new();

    // Set up listeners
    let dispatcher_clone = dispatcher.clone();
    tokio::spawn(async move {
        let mut rx = dispatcher_clone.subscribe();
        while let Ok(event) = rx.recv().await {
            match event {
                _ => {} // Handle events
            }
        }
    });

    // Create the event-dispatching invoker
    let event_invoker = EventDispatchingInvoker::new(
        Box::new(invoker),
        dispatcher,
    );

    Ok(Box::new(event_invoker))
}
```

## 5. Signal Handling

RustForge includes comprehensive signal handling support via the `foundry-signal-handler` crate:

```rust
use foundry_signal_handler::{SignalHandler, Signal};

async fn handle_signals() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = SignalHandler::new();

    // Register a signal handler
    handler.on_signal(Signal::SIGTERM, || {
        println!("Caught SIGTERM, cleaning up...");
        // Perform cleanup
    }).await?;

    // Async cleanup example
    handler.on_signal_async(Signal::SIGINT, || async {
        println!("Caught SIGINT, closing connections...");
        // Async cleanup operations
        Ok(())
    }).await?;

    // Wait for signals
    let signal = handler.wait().await?;
    println!("Received signal: {:?}", signal);

    Ok(())
}
```

## 6. Complete Integration Example

```rust
use foundry_api::{Artisan, EventDispatcher, EventDispatchingInvoker};
use foundry_application::FoundryApp;
use foundry_api::invocation::FoundryInvoker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize app
    let app = FoundryApp::new(config)?;
    let invoker = FoundryInvoker::new(app);

    // Set up event dispatching
    let dispatcher = EventDispatcher::new();
    let dispatcher_clone = dispatcher.clone();

    // Listen for events
    tokio::spawn(async move {
        let mut rx = dispatcher_clone.subscribe();
        while let Ok(event) = rx.recv().await {
            println!("Event: {:?}", event);
        }
    });

    // Create Artisan instance
    let artisan = Artisan::new(invoker.clone());

    // Run migrations and seeds
    println!("Setting up database...");
    let results = artisan
        .chain()
        .add("migrate")
        .add("seed:run")
        .dispatch()
        .await?;

    println!("Setup complete!");
    println!("Output:\n{}", artisan.output_string());

    // List commands
    println!("\nAvailable commands:");
    let _ = artisan.call("list").dispatch().await;

    Ok(())
}
```

## 7. Error Handling

```rust
use foundry_api::Artisan;

async fn handle_command_errors(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    match artisan.call("make:command").with_arg("InvalidName").dispatch().await {
        Ok(result) => {
            println!("Command executed: {:?}", result.status);

            if let Some(error) = result.error {
                println!("Command error: {}", error.message);
                println!("Error code: {}", error.code);
            }
        }
        Err(err) => {
            println!("Invocation error: {}", err);
        }
    }

    Ok(())
}
```

## 8. Advanced Patterns

### Conditional Execution

```rust
async fn conditional_execution(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    let migrate_result = artisan.call("migrate").dispatch().await?;

    if migrate_result.status == CommandStatus::Success {
        // Only run seeds if migration succeeded
        artisan.call("seed:run").dispatch().await?;
    }

    Ok(())
}
```

### Dry Run Mode

```rust
async fn preview_changes(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    let result = artisan
        .call("migrate")
        .dry_run(true)
        .dispatch()
        .await?;

    println!("What would happen:\n{}", result.message.unwrap_or_default());
    Ok(())
}
```

### Force Mode

```rust
async fn force_execution(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    let result = artisan
        .call("migrate")
        .force(true)
        .dispatch()
        .await?;

    Ok(())
}
```

## Key Features

✅ **Programmatic Command Execution** - Call commands from Rust code
✅ **Command Chaining** - Execute multiple commands in sequence
✅ **Output Capture** - Capture command output for processing
✅ **Event System** - Hook into command lifecycle events
✅ **Signal Handling** - Graceful shutdown support
✅ **Error Handling** - Comprehensive error information
✅ **Flexible Output Formats** - JSON, Human-readable, custom

## Best Practices

1. **Always use `await`** - Commands are async and must be awaited
2. **Handle errors gracefully** - Always handle `Result` types
3. **Use event dispatching** for application-wide command monitoring
4. **Capture output** when processing command results
5. **Chain related commands** for better error handling
6. **Listen for signals** in long-running applications
7. **Use dry-run mode** to preview changes

## Migration from Laravel

| Laravel | RustForge |
|---------|-----------|
| `Artisan::call()` | `artisan.call().dispatch()` |
| `Artisan::queue()` | Use `queue` service in context |
| Event listeners | `EventDispatcher::subscribe()` |
| Signal trapping | `foundry_signal_handler` crate |
| Output capture | `artisan.output()` |
| Command chaining | `artisan.chain()` |

## Performance Considerations

- Command execution is **fully async** - use with `tokio::spawn()` for concurrent execution
- Event dispatching uses **tokio broadcast channels** for efficient multi-subscriber support
- Output capture stores strings in memory - for large outputs, consider streaming
- Signal handling uses **signal-hook** for platform-specific efficient signal delivery

## See Also

- [RustForge Documentation](https://github.com/Chregu12/RustForge)
- [foundry-signal-handler Documentation](crates/foundry-signal-handler/)
- [Artisan API Reference](crates/foundry-api/src/artisan.rs)
- [Event System Reference](crates/foundry-api/src/events.rs)
