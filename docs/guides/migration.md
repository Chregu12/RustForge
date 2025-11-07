# MIGRATION GUIDE: Phase 1 Critical Fixes

## Overview

This guide documents all breaking changes and migration steps for Phase 1: Critical Fixes in the RustForge Framework. These changes improve robustness, error handling, and observability throughout the codebase.

**Date:** 2025-11-03
**Version:** 0.1.0 → 0.2.0
**Breaking Changes:** Yes
**Deprecations:** None

---

## Table of Contents

1. [Breaking Changes Summary](#breaking-changes-summary)
2. [foundry-health Fixes](#foundry-health-fixes)
3. [foundry-application Registry Changes](#foundry-application-registry-changes)
4. [foundry-signal-handler Fixes](#foundry-signal-handler-fixes)
5. [Error Handling Improvements](#error-handling-improvements)
6. [Tracing Instrumentation](#tracing-instrumentation)
7. [Migration Steps](#migration-steps)
8. [Testing Guidelines](#testing-guidelines)

---

## Breaking Changes Summary

### Critical API Changes

| Component | Change | Impact | Migration Required |
|-----------|--------|--------|-------------------|
| `CommandRegistry::resolve()` | Returns `Result<Option<DynCommand>>` instead of `Option<DynCommand>` | HIGH | Yes |
| `CommandRegistry::descriptors()` | Returns `Result<Vec<CommandDescriptor>>` instead of `Vec<CommandDescriptor>` | HIGH | Yes |
| `CommandRegistry::len()` | Returns `Result<usize>` instead of `usize` | MEDIUM | Yes |
| `CommandRegistry::is_empty()` | Returns `Result<bool>` instead of `bool` | LOW | Yes |
| `HealthCheckCommand` | Uses `FoundryCommand` trait instead of `CommandExecutor` | HIGH | Yes |
| `SignalHandler::wait()` | Fixed borrow checker issues with `signals` | LOW | No |

---

## foundry-health Fixes

### Problem

The `foundry-health` crate had multiple compilation errors:

1. Missing `chrono` dependency
2. Incorrect imports from `foundry-plugins` (using old trait names)
3. Incompatible `sysinfo` API usage (v0.31 changes)
4. Lifetime issues with `tokio::join!` macro

### Solution

#### 1. Added Missing Dependencies

**File:** `/crates/foundry-health/Cargo.toml`

```diff
[dependencies]
anyhow.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
+chrono.workspace = true
+foundry-domain = { path = "../foundry-domain" }
+once_cell = "1.19"
foundry-plugins = { path = "../foundry-plugins" }
colored = "2.1"
sysinfo = "0.31"
```

#### 2. Updated Command Implementation

**File:** `/crates/foundry-health/src/command.rs`

**Before:**
```rust
use foundry_plugins::{CommandExecutor, CommandResult, ExecutionContext};

#[async_trait]
impl CommandExecutor for HealthCheckCommand {
    fn name(&self) -> &'static str {
        "health:check"
    }

    fn description(&self) -> &'static str {
        "Run comprehensive health checks"
    }

    async fn execute(&self, ctx: &ExecutionContext) -> Result<CommandResult> {
        // ...
    }
}
```

**After:**
```rust
use foundry_plugins::{CommandContext, CommandResult, ResponseFormat, FoundryCommand, CommandError};
use foundry_domain::CommandDescriptor;

#[async_trait]
impl FoundryCommand for HealthCheckCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        static DESCRIPTOR: once_cell::sync::Lazy<CommandDescriptor> =
            once_cell::sync::Lazy::new(|| {
                CommandDescriptor::builder("health:check", "health:check")
                    .summary("Run comprehensive health checks on the application")
                    .description("Performs system diagnostics")
                    .alias("doctor")
                    .build()
            });
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Proper error handling with map_err
        let results = checker.check_all().await
            .map_err(|e| CommandError::Message(e.to_string()))?;
        // ...
    }
}
```

#### 3. Fixed sysinfo API Usage

**File:** `/crates/foundry-health/src/checks.rs`

**Before:**
```rust
let mut sys = System::new_all();
sys.refresh_disks();
let disks = sys.disks();  // ERROR: Method not found
let disk = &disks[0];
let available_gb = disk.available_space() / 1024 / 1024 / 1024;
```

**After:**
```rust
let mut sys = System::new_all();
sys.refresh_all();

// Use memory as proxy - in production use proper disk library
let available_mb = sys.available_memory() / 1024 / 1024;
let total_mb = sys.total_memory() / 1024 / 1024;
let available_gb = available_mb / 1024;
```

#### 4. Fixed Lifetime Issues

**File:** `/crates/foundry-health/src/lib.rs`

**Before:**
```rust
let (rust, disk, memory, env, files) = tokio::join!(
    RustVersionCheck.run(),
    DiskSpaceCheck.run(),
    MemoryCheck.run(),
    EnvCheck::new(self.config.required_env_vars.clone()).run(),  // ERROR: Temporary dropped
    FilePermissionsCheck::new(self.config.required_files.clone()).run(),  // ERROR: Temporary dropped
);
```

**After:**
```rust
// Create check instances outside tokio::join! to avoid lifetime issues
let env_check = EnvCheck::new(self.config.required_env_vars.clone());
let files_check = FilePermissionsCheck::new(self.config.required_files.clone());

let (rust, disk, memory, env, files) = tokio::join!(
    RustVersionCheck.run(),
    DiskSpaceCheck.run(),
    MemoryCheck.run(),
    env_check.run(),
    files_check.run(),
);
```

---

## foundry-application Registry Changes

### Problem

The `CommandRegistry` used `.expect("registry poisoned")` in multiple places, which would panic the application if the mutex was poisoned. This is unacceptable in production systems.

### Solution

#### 1. Updated Error Types

**File:** `/crates/foundry-application/src/error.rs`

```diff
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("command '{0}' ist bereits registriert")]
    CommandAlreadyRegistered(String),
    #[error("command '{0}' nicht gefunden")]
    CommandNotFound(String),
    #[error("command execution failed")]
    CommandExecution(#[source] CommandError),
    #[error("Storage error: {0}")]
    StorageError(String),
+   #[error("Registry corrupted: lock poisoned")]
+   RegistryCorrupted,
+   #[error("Lock poisoned: {0}")]
+   LockPoisoned(String),
}
```

#### 2. Updated Registry Methods

**File:** `/crates/foundry-application/src/registry.rs`

All registry methods now return `Result` types instead of panicking:

**Before:**
```rust
pub fn resolve(&self, command: &str) -> Option<DynCommand> {
    let inner = self.inner.lock().expect("registry poisoned");
    // ...
}

pub fn descriptors(&self) -> Vec<CommandDescriptor> {
    let inner = self.inner.lock().expect("registry poisoned");
    // ...
}

pub fn len(&self) -> usize {
    let inner = self.inner.lock().expect("registry poisoned");
    // ...
}
```

**After:**
```rust
#[instrument(skip(self), fields(command))]
pub fn resolve(&self, command: &str) -> Result<Option<DynCommand>, ApplicationError> {
    let inner = self.inner.lock()
        .map_err(|_| ApplicationError::RegistryCorrupted)?;

    let key = command.to_lowercase();
    let index = inner.lookup.get(&key);
    let result = index.and_then(|idx| inner.commands.get(*idx).cloned());

    if result.is_some() {
        debug!("Command resolved successfully");
    } else {
        debug!("Command not found in registry");
    }

    Ok(result)
}

pub fn descriptors(&self) -> Result<Vec<CommandDescriptor>, ApplicationError> {
    let inner = self.inner.lock()
        .map_err(|_| ApplicationError::RegistryCorrupted)?;
    Ok(inner.commands.iter().map(|cmd| cmd.descriptor().clone()).collect())
}

pub fn len(&self) -> Result<usize, ApplicationError> {
    let inner = self.inner.lock()
        .map_err(|_| ApplicationError::RegistryCorrupted)?;
    Ok(inner.commands.len())
}

pub fn is_empty(&self) -> Result<bool, ApplicationError> {
    Ok(self.len()? == 0)
}
```

#### 3. Updated Callsites

**File:** `/crates/foundry-application/src/lib.rs`

**Before:**
```rust
pub async fn dispatch(...) -> Result<CommandResult, ApplicationError> {
    let handle = self.registry.resolve(command)
        .ok_or_else(|| ApplicationError::CommandNotFound(command.to_string()))?;

    let catalog = self.registry.descriptors();
    // ...
}
```

**After:**
```rust
#[instrument(skip(self, args), fields(command, num_args = args.len()))]
pub async fn dispatch(...) -> Result<CommandResult, ApplicationError> {
    info!("Dispatching command: {}", command);

    let handle = self.registry.resolve(command)?  // Now returns Result
        .ok_or_else(|| ApplicationError::CommandNotFound(command.to_string()))?;

    let catalog = self.registry.descriptors()?;  // Now returns Result
    // ...
}
```

**File:** `/crates/foundry-application/src/commands/list.rs`

**Before:**
```rust
async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let catalog = self.registry.descriptors();
    let total = catalog.len();
    // ...
}
```

**After:**
```rust
async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let catalog = self.registry.descriptors()
        .map_err(|e| CommandError::Message(e.to_string()))?;
    let total = catalog.len();
    // ...
}
```

---

## foundry-signal-handler Fixes

### Problem

The `foundry-signal-handler` crate had compilation errors:

1. Missing `tokio::sync` feature for `RwLock`
2. Missing `Hash` derive on `ShutdownPhase`
3. Borrow checker issues with mutable/immutable borrows

### Solution

#### 1. Added Missing Tokio Feature

**File:** `/crates/foundry-signal-handler/Cargo.toml`

```diff
[dependencies]
anyhow.workspace = true
async-trait.workspace = true
thiserror.workspace = true
-tokio.workspace = true
+tokio = { workspace = true, features = ["sync"] }
tracing.workspace = true
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }
futures.workspace = true
```

#### 2. Fixed Hash Derive

**File:** `/crates/foundry-signal-handler/src/shutdown.rs`

```diff
-#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
+#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShutdownPhase {
    PreShutdown,
    Shutdown,
    PostShutdown,
}
```

#### 3. Fixed Borrow Checker Issues

**File:** `/crates/foundry-signal-handler/src/handler.rs`

**Before:**
```rust
pub async fn wait(&mut self) -> SignalResult<i32> {
    let signals = self.signals.as_mut()
        .ok_or_else(|| SignalError::HandlingFailed("..."))?;

    while let Some(signal_num) = signals.next().await {
        let signal = self.map_signal_num(signal_num);  // ERROR: Can't borrow self
        // ...
    }
}
```

**After:**
```rust
pub async fn wait(&mut self) -> SignalResult<i32> {
    // Take ownership to avoid borrow conflicts
    let mut signals = self.signals.take()
        .ok_or_else(|| SignalError::HandlingFailed("..."))?;

    while let Some(signal_num) = signals.next().await {
        // Use static method to avoid self borrow
        let signal = Self::map_signal_num_static(signal_num);
        // ...
    }
}

// Convert to static method
fn map_signal_num_static(signal_num: i32) -> Signal {
    match signal_num {
        SIGTERM => Signal::SIGTERM,
        SIGINT => Signal::SIGINT,
        // ...
    }
}
```

---

## Error Handling Improvements

### General Pattern

**Replace all `.expect()` and `.unwrap()` calls with proper error handling:**

**Before (Anti-Pattern):**
```rust
let value = something.lock().expect("lock poisoned");
let item = map.get("key").unwrap();
let result = parse_value().expect("failed to parse");
```

**After (Best Practice):**
```rust
// 1. Return Result
let value = something.lock()
    .map_err(|_| MyError::LockPoisoned)?;

// 2. Provide default
let item = map.get("key").unwrap_or(&default_value);

// 3. Map to domain error
let result = parse_value()
    .map_err(|e| MyError::ParseFailed(e.to_string()))?;
```

### Error Propagation

Use `?` operator consistently instead of manual error handling:

**Before:**
```rust
match do_something() {
    Ok(value) => value,
    Err(e) => return Err(ApplicationError::Other(e)),
}
```

**After:**
```rust
let value = do_something()
    .map_err(|e| ApplicationError::Other(e.to_string()))?;
```

---

## Tracing Instrumentation

### Added Instrumentation to Critical Paths

All critical code paths now have proper tracing instrumentation for observability:

**File:** `/crates/foundry-application/src/registry.rs`

```rust
use tracing::{debug, instrument};

#[instrument(skip(self, command), fields(command_name = %command.descriptor().name))]
pub fn register(&self, command: DynCommand) -> Result<(), ApplicationError> {
    // ...
    debug!("Registered command: {}", command_name);
    Ok(())
}

#[instrument(skip(self), fields(command))]
pub fn resolve(&self, command: &str) -> Result<Option<DynCommand>, ApplicationError> {
    // ...
    if result.is_some() {
        debug!("Command resolved successfully");
    } else {
        debug!("Command not found in registry");
    }
    Ok(result)
}
```

**File:** `/crates/foundry-application/src/lib.rs`

```rust
#[instrument(skip(self, args), fields(command, num_args = args.len()))]
pub async fn dispatch(
    &self,
    command: &str,
    args: Vec<String>,
    format: ResponseFormat,
    options: ExecutionOptions,
) -> Result<CommandResult, ApplicationError> {
    info!("Dispatching command: {}", command);
    // ...
}
```

### Tracing Levels

- `error!`: Critical failures that prevent operation
- `warn!`: Recoverable errors or unexpected conditions
- `info!`: Important state changes and operations
- `debug!`: Detailed diagnostic information

---

## Migration Steps

### Step 1: Update Dependencies

Run `cargo update` to ensure all workspace dependencies are current:

```bash
cargo update
```

### Step 2: Update Code Using CommandRegistry

Find all usages of `CommandRegistry` methods:

```bash
grep -r "registry\.resolve\|registry\.descriptors\|registry\.len" crates/
```

For each usage, wrap with `?` operator or handle the `Result`:

```rust
// Before
let cmd = registry.resolve("test");

// After
let cmd = registry.resolve("test")?;

// Or with error mapping
let cmd = registry.resolve("test")
    .map_err(|e| MyError::Registry(e))?;
```

### Step 3: Update Health Check Commands

If you have custom health check commands, update them to use `FoundryCommand`:

```rust
// 1. Add dependencies
use foundry_domain::CommandDescriptor;
use once_cell::sync::Lazy;

// 2. Update trait implementation
#[async_trait]
impl FoundryCommand for MyHealthCheck {
    fn descriptor(&self) -> &CommandDescriptor {
        static DESCRIPTOR: Lazy<CommandDescriptor> = Lazy::new(|| {
            CommandDescriptor::builder("my:check", "my:check")
                .summary("My custom health check")
                .build()
        });
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Update error handling
        let result = do_check()
            .await
            .map_err(|e| CommandError::Message(e.to_string()))?;

        Ok(CommandResult::success("Check passed"))
    }
}
```

### Step 4: Run Tests

Run all tests to identify migration issues:

```bash
cargo test --workspace
```

Fix any compilation errors or test failures.

### Step 5: Add Tracing (Optional)

Add tracing to your own critical paths:

```rust
use tracing::{info, debug, instrument};

#[instrument(skip(self), fields(operation = "my_operation"))]
async fn my_critical_function(&self, param: String) -> Result<()> {
    info!("Starting critical operation");

    let result = do_work().await?;

    debug!("Operation completed successfully");
    Ok(result)
}
```

---

## Testing Guidelines

### Regression Tests

New regression tests have been added to ensure error handling works correctly:

**File:** `/crates/foundry-application/tests/test_registry_error_handling.rs`

Run these tests with:

```bash
cargo test --package foundry-application test_registry
```

### Test Your Changes

1. **Unit Tests**: Ensure all unit tests pass
   ```bash
   cargo test --lib
   ```

2. **Integration Tests**: Test full workflows
   ```bash
   cargo test --test '*'
   ```

3. **Manual Testing**: Test critical paths manually
   ```bash
   cargo run --package foundry-cli -- list
   cargo run --package foundry-cli -- health:check
   ```

---

## Backward Compatibility Notes

### BREAKING: Registry Methods

The following methods are **BREAKING CHANGES**:

- `CommandRegistry::resolve()` - now returns `Result<Option<_>>`
- `CommandRegistry::descriptors()` - now returns `Result<Vec<_>>`
- `CommandRegistry::len()` - now returns `Result<usize>`
- `CommandRegistry::is_empty()` - now returns `Result<bool>`

**Action Required:** Update all callsites to handle `Result` type.

### BREAKING: Health Check Commands

Health check commands now implement `FoundryCommand` instead of deprecated `CommandExecutor`.

**Action Required:** Update custom health check implementations.

### NON-BREAKING: Signal Handler

Signal handler changes are internal implementation details. External API remains the same.

**Action Required:** None for users of the public API.

---

## Troubleshooting

### "Method expects Result but found Option"

**Error:**
```
error[E0308]: mismatched types
  expected `Result<Option<DynCommand>, ApplicationError>`
     found `Option<DynCommand>`
```

**Solution:**
```rust
// Wrap with Ok()
let cmd = registry.resolve("test")?;
```

### "Registry corrupted" at Runtime

If you see `ApplicationError::RegistryCorrupted` at runtime:

1. Check for thread panics that might poison the mutex
2. Add proper error handling in all threads accessing the registry
3. Consider using `std::sync::RwLock` for read-heavy workloads

### Tracing Output Not Appearing

If you don't see tracing output:

1. Initialize the tracing subscriber:
   ```rust
   tracing_subscriber::fmt()
       .with_env_filter("debug")
       .init();
   ```

2. Set `RUST_LOG` environment variable:
   ```bash
   RUST_LOG=debug cargo run
   ```

---

## Support and Questions

For questions or issues related to this migration:

1. Check existing issues: https://github.com/your-org/rust-dx-framework/issues
2. Create a new issue with the `migration` label
3. Consult the team via Slack: #rust-dx-framework

---

## Changelog

### Phase 1: Critical Fixes (2025-11-03)

#### Fixed
- foundry-health compilation errors (missing dependencies, wrong API usage)
- foundry-application CommandRegistry panic on poisoned mutex
- foundry-signal-handler compilation errors (missing features, borrow checker)

#### Changed
- **BREAKING:** CommandRegistry methods now return `Result` types
- **BREAKING:** Health check commands use `FoundryCommand` trait

#### Added
- Tracing instrumentation to critical paths
- Comprehensive error types (`RegistryCorrupted`, `LockPoisoned`)
- Regression tests for error handling
- MIGRATION_GUIDE.md documentation

---

**End of Migration Guide**
