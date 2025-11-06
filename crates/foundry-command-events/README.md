# foundry-command-events

A comprehensive event system for command lifecycle management in RustForge.

## Features

- **Event-based Command Lifecycle**: Hook into command execution at various stages
- **Async Event Listeners**: Fully async/await compatible event handling
- **Priority-based Execution**: Control the order of listener execution
- **Error Handling**: Graceful error handling that doesn't break the event chain
- **Broadcasting**: Fire events to multiple listeners simultaneously
- **One-time Listeners**: Support for listeners that auto-remove after execution

## Event Types

### CommandStarting
Fired before a command starts execution.

```rust
use foundry_command_events::{EventDispatcher, CommandStarting};
use chrono::Utc;

let event = CommandStarting {
    command: "make:model".to_string(),
    args: vec!["User".to_string()],
    timestamp: Utc::now(),
    context: HashMap::new(),
};

dispatcher.dispatch(event).await?;
```

### CommandFinished
Fired after a command completes successfully.

```rust
use foundry_command_events::{EventDispatcher, CommandFinished};

let event = CommandFinished {
    command: "make:model".to_string(),
    duration: 150,
    exit_code: 0,
    output: "Model created successfully".to_string(),
};

dispatcher.dispatch(event).await?;
```

### CommandFailed
Fired when a command fails.

```rust
use foundry_command_events::{EventDispatcher, CommandFailed};

let event = CommandFailed {
    command: "make:model".to_string(),
    error: "File already exists".to_string(),
    exit_code: 1,
    duration: 50,
};

dispatcher.dispatch(event).await?;
```

### CommandTerminated
Fired when a command receives SIGTERM.

### CustomEvent
User-defined events with arbitrary data.

```rust
use foundry_command_events::{EventDispatcher, CustomEvent};

let event = CustomEvent::new("user.registered")
    .with_data("user_id", serde_json::json!(123))
    .with_data("email", serde_json::json!("user@example.com"));

dispatcher.dispatch(event).await?;
```

## Usage

### Basic Event Listening

```rust
use foundry_command_events::{EventDispatcher, CommandFinished};
use foundry_command_events::listener::FunctionListener;

#[tokio::main]
async fn main() {
    let dispatcher = EventDispatcher::new();

    // Register a listener
    let listener = FunctionListener::new(|event: &CommandFinished| {
        Box::pin(async move {
            println!("Command {} finished in {}ms", event.command, event.duration);
            Ok(())
        })
    });

    dispatcher.listen(listener).await;

    // Dispatch event
    let event = CommandFinished {
        command: "make:model".to_string(),
        duration: 100,
        exit_code: 0,
        output: "Success".to_string(),
    };

    dispatcher.dispatch(event).await.unwrap();
}
```

### Priority-based Listeners

```rust
use foundry_command_events::listener::{FunctionListener, ListenerPriority};

let high_priority = FunctionListener::new(|event: &CommandFinished| {
    Box::pin(async move {
        println!("High priority listener");
        Ok(())
    })
})
.with_priority(ListenerPriority::Highest);

let low_priority = FunctionListener::new(|event: &CommandFinished| {
    Box::pin(async move {
        println!("Low priority listener");
        Ok(())
    })
})
.with_priority(ListenerPriority::Lowest);

dispatcher.listen(high_priority).await;
dispatcher.listen(low_priority).await;
```

### One-time Listeners

```rust
let listener = FunctionListener::new(|event: &CommandFinished| {
    Box::pin(async move {
        println!("This will only run once");
        Ok(())
    })
})
.once();

dispatcher.listen(listener).await;
```

### Event Context

```rust
use foundry_command_events::EventContext;

let context = EventContext::new("make:model")
    .with_args(vec!["User".to_string()])
    .with_metadata("type", "model")
    .finish(0, "Success");

println!("Duration: {:?}ms", context.duration());
```

## Error Handling

Listeners can return errors without breaking the event chain:

```rust
let listener1 = FunctionListener::new(|event: &CommandFinished| {
    Box::pin(async move {
        Err(EventError::ListenerError("Something went wrong".to_string()))
    })
});

let listener2 = FunctionListener::new(|event: &CommandFinished| {
    Box::pin(async move {
        println!("This will still execute");
        Ok(())
    })
});

dispatcher.listen(listener1).await;
dispatcher.listen(listener2).await;

// Both listeners run, errors are logged but don't stop execution
dispatcher.dispatch(event).await?;
```

## Advanced Usage

### Custom Event Listeners

Implement the `EventListener` trait for custom behavior:

```rust
use async_trait::async_trait;
use foundry_command_events::{EventListener, CommandFinished};
use foundry_command_events::error::Result;

struct LoggingListener;

#[async_trait]
impl EventListener<CommandFinished> for LoggingListener {
    async fn handle(&self, event: &CommandFinished) -> Result<()> {
        println!("[LOG] Command: {}, Duration: {}ms",
            event.command, event.duration);
        Ok(())
    }
}

dispatcher.listen(LoggingListener).await;
```

### Managing Listeners

```rust
// Remove all listeners for an event type
dispatcher.clear_listeners::<CommandFinished>().await;

// Remove all listeners
dispatcher.clear_all().await;

// Get listener count
let count = dispatcher.listener_count::<CommandFinished>().await;
```

## Integration with Commands

```rust
use foundry_command_events::{EventDispatcher, CommandStarting, CommandFinished};

async fn execute_command(dispatcher: &EventDispatcher) {
    // Fire starting event
    dispatcher.dispatch(CommandStarting {
        command: "make:model".to_string(),
        args: vec![],
        timestamp: Utc::now(),
        context: HashMap::new(),
    }).await.unwrap();

    // Execute command logic...
    let start = std::time::Instant::now();

    // Your command logic here

    let duration = start.elapsed().as_millis() as u64;

    // Fire finished event
    dispatcher.dispatch(CommandFinished {
        command: "make:model".to_string(),
        duration,
        exit_code: 0,
        output: "Success".to_string(),
    }).await.unwrap();
}
```

## Testing

Run tests with:

```bash
cargo test --package foundry-command-events
```

## License

MIT OR Apache-2.0
