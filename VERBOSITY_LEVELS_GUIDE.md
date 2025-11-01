# RustForge Verbosity Levels Guide

This guide explains how to use verbosity levels in RustForge commands, providing users with control over output detail.

## Overview

Verbosity levels allow commands to output different amounts of information based on user preference. This is similar to common Unix tools like `curl`, `wget`, and Laravel's Artisan.

## Verbosity Levels

RustForge supports 5 verbosity levels:

### 1. **Quiet** (`-q`, `--quiet`)
- **Level**: 0
- **Description**: Suppress all output except errors
- **Use case**: Scripting, automated tasks, clean output
- **Output**: Only critical errors

```bash
foundry command -q
```

### 2. **Normal** (default)
- **Level**: 1
- **Description**: Standard output, balanced information
- **Use case**: Regular command usage
- **Output**: Success/failure messages, key information

```bash
foundry migrate
```

### 3. **Verbose** (`-v`)
- **Level**: 2
- **Description**: Additional information and details
- **Use case**: Understanding what the command is doing
- **Output**: Normal output + detailed steps, file paths, counts

```bash
foundry migrate -v
```

### 4. **Very Verbose** (`-vv`)
- **Level**: 3
- **Description**: Much more detailed information
- **Use case**: Debugging, detailed inspection
- **Output**: Verbose output + internal states, SQL queries, performance metrics

```bash
foundry migrate -vv
```

### 5. **Debug** (`-vvv`, `--debug`)
- **Level**: 4
- **Description**: All debug information, complete tracing
- **Use case**: Development, deep debugging
- **Output**: All information including stack traces, internal variables, timings

```bash
foundry migrate -vvv
# or
foundry migrate --debug
```

## Using Verbosity in Commands

### 1. Extract Verbosity from Arguments

```rust
use foundry_api::Verbosity;
use foundry_plugins::CommandResult;

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let verbosity = Verbosity::from_args(&ctx.args);

    // Check verbosity level
    if verbosity.is_verbose() {
        println!("Verbose mode enabled");
    }

    if verbosity.is_debug() {
        println!("Debug mode enabled");
    }

    Ok(CommandResult::success("Done"))
}
```

### 2. Using the Console Helper

```rust
use foundry_api::Console;
use foundry_plugins::CommandResult;

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let verbosity = Verbosity::from_args(&ctx.args);
    let console = Console::new(verbosity.level());

    // Always shown
    console.line("Running migration...");

    // Shown unless quiet
    console.info("Connected to database");

    // Shown with -v
    console.verbose("Executing SQL: SELECT * FROM users");

    // Shown with -vv
    console.very_verbose("Row count: 42");

    // Shown with -vvv
    console.debug("Internal state: {:?}", state);

    // Success message
    console.success("Migration completed!");

    Ok(CommandResult::success("Done"))
}
```

### 3. Conditional Output

```rust
use foundry_api::{Console, VerbosityLevel};
use foundry_plugins::CommandResult;

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let verbosity = Verbosity::from_args(&ctx.args);
    let console = Console::new(verbosity.level());

    // Conditional operations
    if console.is_verbose() {
        // Only perform expensive logging if verbose
        let details = get_detailed_info();
        console.verbose(&format!("Details: {:?}", details));
    }

    // Use conditional output methods
    console.info_if(
        console.is_verbose(),
        "This is shown only if verbose"
    );

    Ok(CommandResult::success("Done"))
}
```

### 4. Complex Output Management

```rust
use foundry_api::Console;

async fn process_items(console: &Console) {
    let items = vec!["item1", "item2", "item3"];

    console.section("Processing Items");

    for (idx, item) in items.iter().enumerate() {
        console.info(&console.progress(idx + 1, items.len(), item));

        // Verbose details
        console.verbose(&format!("Processing: {}", item));

        // Debug internals
        console.debug(&format!("Item pointer: {:p}", item));
    }

    console.blank();
    console.success("All items processed!");
}
```

## Console Methods

### Basic Output

| Method | Quiet | Normal | Verbose | Debug |
|--------|-------|--------|---------|-------|
| `line()` | ✓ | ✓ | ✓ | ✓ |
| `info()` | ✗ | ✓ | ✓ | ✓ |
| `verbose()` | ✗ | ✗ | ✓ | ✓ |
| `very_verbose()` | ✗ | ✗ | ✗ | ✓ |
| `debug()` | ✗ | ✗ | ✗ | ✓ |
| `error()` | ✓ | ✓ | ✓ | ✓ |

### Formatted Output

```rust
let console = Console::new(VerbosityLevel::Verbose);

// Colored output
console.success("✓ Operation successful");
console.warn("⚠ Warning message");
console.error("✗ Error occurred");

// Structured output
console.section("Configuration");
console.item("Database", "postgresql://localhost/db");
console.item("Port", "5432");
console.blank();

// Lists
console.list_item("First item");
console.list_item("Second item");

// Tables
console.table_row(&["Name", "Value", "Status"]);
console.table_row(&["connection", "active", "✓"]);

// Progress
let progress = console.progress(5, 10, "Processing files");
console.info(progress);
```

### Conditional Output

```rust
let console = Console::new(VerbosityLevel::Normal);

console.line("Always shown");
console.line_if(true, "Shown if condition is true");
console.info_if(console.is_verbose(), "Shown only if verbose");
console.verbose_if(condition, "Shown if condition and verbose");
```

## Practical Examples

### Example 1: Database Migration Command

```rust
use foundry_api::{Console, Verbosity};
use foundry_plugins::{CommandContext, CommandResult, CommandError};

pub struct MigrateCommand;

#[async_trait]
impl FoundryCommand for MigrateCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        // ... descriptor implementation
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let verbosity = Verbosity::from_args(&ctx.args);
        let console = Console::new(verbosity.level());

        console.section("Running Database Migrations");

        let migrations = vec![
            ("2024_01_create_users", false),
            ("2024_02_create_posts", false),
            ("2024_03_add_timestamps", false),
        ];

        let mut executed = 0;

        for (idx, (migration_name, exists)) in migrations.iter().enumerate() {
            let status = if *exists { "✓ Skipped" } else { "⤳ Migrating" };
            console.info(&format!(
                "[{}/{}] {}",
                idx + 1,
                migrations.len(),
                status
            ));

            if verbosity.is_verbose() {
                console.verbose(&format!("Running: {}", migration_name));
            }

            if !*exists {
                // Execute migration
                console.very_verbose(&format!("Executing SQL for: {}", migration_name));
                console.debug(&format!("Duration: 45ms"));
                executed += 1;
            }
        }

        console.blank();
        console.success(&format!("✓ {} migration(s) executed", executed));

        Ok(CommandResult::success(
            format!("Successfully migrated {} changes", executed)
        ))
    }
}
```

### Example 2: File Operations Command

```rust
use foundry_api::Console;

async fn process_files(console: &Console) -> Result<()> {
    let files = vec!["app.rs", "lib.rs", "main.rs"];

    console.section("Processing Files");

    for file in files {
        let size = get_file_size(file)?;

        console.info(&format!("Processing: {}", file));

        if console.is_verbose() {
            console.verbose(&format!("  Size: {} bytes", size));
            console.verbose(&format!("  Modified: {}", get_modified_time(file)?));
        }

        if console.is_very_verbose() {
            console.very_verbose(&format!("  Permissions: 0644"));
            console.very_verbose(&format!("  Owner: user"));
        }

        if console.is_debug() {
            console.debug(&format!("  inode: {}", get_inode(file)?));
            console.debug(&format!("  Checksum: {}", calculate_checksum(file)?));
        }
    }

    console.success("All files processed!");
    Ok(())
}
```

### Example 3: API Command

```rust
use foundry_api::Console;

async fn call_api(console: &Console) -> Result<()> {
    console.section("Calling External API");

    let url = "https://api.example.com/data";
    console.info(&format!("Endpoint: {}", url));

    if console.is_verbose() {
        console.verbose("Setting headers:");
        console.verbose("  Authorization: Bearer <token>");
        console.verbose("  Content-Type: application/json");
    }

    console.info("Sending request...");

    if console.is_very_verbose() {
        console.very_verbose("Request payload:");
        console.very_verbose(r#"{"key": "value"}"#);
    }

    let response = make_request(url).await?;

    if console.is_debug() {
        console.debug(&format!("Response time: {}ms", response.elapsed_ms()));
        console.debug(&format!("Status code: {}", response.status()));
    }

    console.success(&format!("Request completed with {} items", response.items.len()));
    Ok(())
}
```

## Best Practices

1. **Use appropriate levels**
   - Normal: Key operations and results
   - Verbose (-v): Intermediate steps and decisions
   - Debug (-vvv): Internal states and detailed tracing

2. **Avoid redundancy**
   - Don't repeat information at different levels
   - Build information hierarchically

3. **Performance considerations**
   - Put expensive operations in `is_verbose()` checks
   - Avoid string formatting for debug output that won't be shown

4. **Consistent messaging**
   - Use same terminology at all levels
   - Maintain consistent formatting

5. **Error handling**
   - Always show errors, even in quiet mode
   - Provide detailed errors at verbose levels

## Comparison with Other Tools

### curl
```bash
curl example.com        # Normal
curl -v example.com     # Verbose
curl -vv example.com    # Very verbose
```

### Laravel Artisan
```bash
php artisan migrate
php artisan migrate --verbose (-v)
php artisan migrate -vv
php artisan migrate -vvv
```

### RustForge
```bash
foundry migrate         # Normal
foundry migrate -v      # Verbose
foundry migrate -vv     # Very verbose
foundry migrate -vvv    # Debug
foundry migrate -q      # Quiet
```

## Integration with Artisan

The Verbosity system works seamlessly with the Artisan facade:

```rust
use foundry_api::Artisan;

async fn execute_with_verbosity(artisan: &Artisan) -> Result<(), Box<dyn std::error::Error>> {
    // Artisan respects verbosity flags
    let result = artisan
        .call("migrate")
        .with_args(vec!["-vv".to_string()])
        .dispatch()
        .await?;

    println!("{:?}", result.status);
    Ok(())
}
```

## Summary

The Verbosity system provides:
- ✅ 5 levels of output control
- ✅ Standard Unix-like flag syntax
- ✅ Easy integration into commands
- ✅ Helper Console class for structured output
- ✅ Colored output support
- ✅ Conditional and formatted messaging
- ✅ Performance-conscious implementation

This makes RustForge commands more user-friendly and matches the feature set of Laravel's Artisan with regards to output verbosity.
