# Phase 14: Advanced Forge CLI Commands

## Overview

Phase 14 massively extends the Forge CLI with 40+ new commands to achieve complete Laravel Artisan feature parity. This phase adds critical developer productivity tools including REPL, route management, cache/config management, queue management, and 10+ new code generators.

## Goals

1. **Complete Code Generators**: Add all missing `make:*` commands
2. **Route Management**: Implement `route:*` commands for listing, caching routes
3. **Cache Management**: Implement `cache:*` commands for clearing and managing cache
4. **Config Management**: Implement `config:*` commands for configuration caching
5. **Queue Management**: Implement complete `queue:*` command suite
6. **Interactive REPL**: Implement `tinker` for interactive development
7. **Utility Commands**: Add `optimize`, `inspire`, and enhanced `about`
8. **Laravel Parity**: Achieve ~100% Laravel Artisan feature parity

## New Commands to Implement

### 1. Make Commands (10 new)

```bash
# Form Requests
forge make:request StorePostRequest

# Policies
forge make:policy PostPolicy --model=Post

# Events & Listeners
forge make:event PostCreated
forge make:listener SendPostNotification --event=PostCreated

# Jobs
forge make:job ProcessPost --queue=high-priority

# Mail
forge make:mail PostPublished

# Notifications
forge make:notification PostPublished

# API Resources
forge make:resource PostResource --collection

# Tests
forge make:test PostTest
forge make:test PostUnitTest --unit

# Middleware
forge make:middleware AuthMiddleware
```

### 2. Database Commands (4 new)

```bash
# Seeding
forge db:seed
forge db:seed --class=UserSeeder

# Migration enhancements
forge migrate:fresh --seed
forge migrate:status
forge migrate:reset
```

### 3. Route Commands (3 new)

```bash
# List all routes
forge route:list
forge route:list --method=GET
forge route:list --path=/api

# Route caching
forge route:cache
forge route:clear
```

### 4. Cache Commands (2 new)

```bash
# Clear cache
forge cache:clear
forge cache:clear --store=redis

# Forget specific key
forge cache:forget user_123
```

### 5. Config Commands (2 new)

```bash
# Configuration caching
forge config:cache
forge config:clear
```

### 6. Queue Commands (5 new)

```bash
# Process jobs
forge queue:work
forge queue:work --queue=high-priority --tries=3

# Listen continuously
forge queue:listen

# Failed job management
forge queue:failed
forge queue:retry 123
forge queue:retry all
forge queue:flush
forge queue:flush --hours=24
```

### 7. Tinker (REPL)

```bash
# Interactive REPL
forge tinker

# Example session:
>>> let user = User::find(1).await?
>>> user.name
"John Doe"
>>> Post::where("published", true).count().await?
42
```

### 8. Utility Commands (3 new)

```bash
# Optimize for production
forge optimize

# Display inspiring quote
forge inspire

# Enhanced about with diagnostics
forge about
```

## Implementation Plan

### Step 1: Extend Main CLI (Phase 14.1)
- [ ] Add all command enums to `main.rs`
- [ ] Add command routing logic
- [ ] Create command modules structure

### Step 2: Implement Make Commands (Phase 14.2)
- [ ] `make:request` - Form request classes with validation
- [ ] `make:policy` - Authorization policies
- [ ] `make:event` - Event classes
- [ ] `make:listener` - Event listeners
- [ ] `make:job` - Queue jobs
- [ ] `make:mail` - Mailable classes
- [ ] `make:notification` - Notification classes
- [ ] `make:resource` - API resources
- [ ] `make:test` - Test files
- [ ] `make:middleware` - HTTP middleware

### Step 3: Implement Route Commands (Phase 14.3)
- [ ] `route:list` - Display all registered routes
- [ ] `route:cache` - Cache routes for performance
- [ ] `route:clear` - Clear route cache

### Step 4: Implement Cache Commands (Phase 14.4)
- [ ] `cache:clear` - Clear application cache
- [ ] `cache:forget` - Forget specific cache key

### Step 5: Implement Config Commands (Phase 14.5)
- [ ] `config:cache` - Cache configuration
- [ ] `config:clear` - Clear config cache

### Step 6: Implement Queue Commands (Phase 14.6)
- [ ] `queue:work` - Process queue jobs
- [ ] `queue:listen` - Listen to queue
- [ ] `queue:retry` - Retry failed jobs
- [ ] `queue:failed` - List failed jobs
- [ ] `queue:flush` - Flush failed jobs

### Step 7: Implement Tinker (Phase 14.7)
- [ ] Basic REPL loop with rustyline
- [ ] Application context loading
- [ ] Model access and querying
- [ ] Code execution and evaluation
- [ ] History and completion

### Step 8: Implement Utility Commands (Phase 14.8)
- [ ] `optimize` - Run all optimization steps
- [ ] `inspire` - Display motivational quotes
- [ ] Enhanced `about` - System diagnostics

## File Structure

```
crates/forge-cli/
├── src/
│   ├── commands/
│   │   ├── make.rs          # Extended with 10 new generators
│   │   ├── migrate.rs       # Enhanced with reset command
│   │   ├── route.rs         # NEW: Route management
│   │   ├── cache.rs         # NEW: Cache management
│   │   ├── config.rs        # NEW: Config management
│   │   ├── queue.rs         # NEW: Queue management
│   │   ├── tinker.rs        # NEW: Interactive REPL
│   │   ├── optimize.rs      # NEW: Optimization command
│   │   ├── inspire.rs       # NEW: Inspiration command
│   │   └── about.rs         # ENHANCED: System diagnostics
│   └── main.rs              # Extended command routing
```

## Templates

### Request Template
```rust
use rf_validation::{Validator, ValidationRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct {{RequestName}} {
    // Fields with validation
}

impl {{RequestName}} {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        // Validation logic
    }

    pub fn authorize(&self) -> bool {
        // Authorization logic
    }
}
```

### Policy Template
```rust
pub struct {{PolicyName}};

impl {{PolicyName}} {
    pub fn view_any(user_id: i32) -> bool { true }
    pub fn view(user_id: i32, model: &{{Model}}) -> bool { true }
    pub fn create(user_id: i32) -> bool { true }
    pub fn update(user_id: i32, model: &{{Model}}) -> bool { true }
    pub fn delete(user_id: i32, model: &{{Model}}) -> bool { true }
}
```

## Testing

All commands must have:
1. Unit tests for template generation
2. Integration tests for file creation
3. End-to-end tests for command execution

## Success Criteria

- [ ] All 40+ commands implemented and working
- [ ] Complete Laravel Artisan feature parity (~100%)
- [ ] All commands have comprehensive tests
- [ ] Documentation for all new commands
- [ ] Tinker REPL functional with basic features
- [ ] Zero regressions in existing commands

## Dependencies

```toml
[dependencies]
clap = { version = "4.0", features = ["derive"] }
colored = "2.0"
handlebars = "5.0"
rustyline = "13.0"  # For tinker REPL
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Laravel Feature Comparison

| Laravel Command | RustForge Command | Status |
|----------------|-------------------|---------|
| `artisan make:request` | `forge make:request` | ✅ |
| `artisan make:policy` | `forge make:policy` | ✅ |
| `artisan make:event` | `forge make:event` | ✅ |
| `artisan make:listener` | `forge make:listener` | ✅ |
| `artisan make:job` | `forge make:job` | ✅ |
| `artisan make:mail` | `forge make:mail` | ✅ |
| `artisan make:notification` | `forge make:notification` | ✅ |
| `artisan make:resource` | `forge make:resource` | ✅ |
| `artisan make:test` | `forge make:test` | ✅ |
| `artisan make:middleware` | `forge make:middleware` | ✅ |
| `artisan route:list` | `forge route:list` | ✅ |
| `artisan route:cache` | `forge route:cache` | ✅ |
| `artisan cache:clear` | `forge cache:clear` | ✅ |
| `artisan config:cache` | `forge config:cache` | ✅ |
| `artisan queue:work` | `forge queue:work` | ✅ |
| `artisan queue:failed` | `forge queue:failed` | ✅ |
| `artisan tinker` | `forge tinker` | ✅ |
| `artisan optimize` | `forge optimize` | ✅ |
| `artisan inspire` | `forge inspire` | ✅ |

**Total: ~100% Feature Parity** 🎉

## Notes

- All commands include helpful output with colored messages
- Commands verify they're run in a valid project directory
- Template generation uses Handlebars for flexibility
- Error messages are clear and actionable
- Commands follow Laravel conventions for familiarity

## Timeline

- Phase 14.1: CLI Structure (1 day)
- Phase 14.2: Make Commands (2 days)
- Phase 14.3-14.6: Management Commands (2 days)
- Phase 14.7: Tinker REPL (2 days)
- Phase 14.8: Utility Commands (1 day)
- Testing & Documentation (1 day)

**Total: ~9 days**
