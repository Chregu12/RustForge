# Tier 3: Advanced Artisan-like Features

This document describes the advanced features implemented for RustForge.

## 1. Enhanced Tinker Shell (`foundry-tinker-enhanced`)

Advanced REPL with persistent history and autocomplete.

### Commands

```bash
foundry tinker
```

### Available Commands in REPL

- `helpers` - Show all available helper functions
- `models` - List all available models
- `routes` - Show all registered routes
- `config <key>` - Show configuration value
- `env <key>` - Show environment variable
- `clear` - Clear the screen
- `history` - Show command history
- `save <name>` - Save session as executable script
- `exit` / `quit` - Exit tinker REPL

### Built-in Helpers

- `now()` - Current timestamp
- `env(key, default)` - Get environment variable
- `config(key, default)` - Get configuration value
- `cache_get(key)`, `cache_put(key, value)` - Cache operations
- `db_query(sql)` - Execute raw SQL
- `dd(value)` - Dump and die

## 2. Maintenance Mode

Activate and deactivate maintenance mode with optional secret bypass.

### Commands

```bash
# Enable maintenance mode
foundry app:down

# With custom message
foundry app:down --message "We'll be back soon!"

# With bypass secret
foundry app:down --secret mysecret123

# With retry-after header (in seconds)
foundry app:down --retry 3600

# Disable maintenance mode
foundry app:up
```

### Middleware Integration

The maintenance middleware checks for:
- `.maintenance` file existence
- Secret in `X-Maintenance-Secret` header
- Secret in `?secret=` query parameter

Returns 503 Service Unavailable with custom HTML page.

## 3. Health Checks & Diagnostics

Comprehensive system diagnostics and health verification.

### Commands

```bash
# Run all health checks
foundry health:check

# Or use alias
foundry doctor

# Run specific check
foundry health:check rust
foundry health:check disk
foundry health:check memory
foundry health:check env
foundry health:check database
```

### Checks Performed

- ✓ Rust Version
- ✓ Database Connection
- ✓ Cache Connection
- ✓ Required Files/Permissions
- ✓ Environment Variables
- ✓ Disk Space
- ✓ Memory Available
- ✓ Dependencies Versions
- ✓ Configuration Validity

### Example Output

```
╔═══════════════════════════════════════════╗
║         Health Check Report              ║
╚═══════════════════════════════════════════╝

  Check                   Status  Message
  ──────────────────────────────────────────────
  rust                    ✓       Rust version 1.75.0
  disk                    ✓       50 GB / 500 GB available
  memory                  ✓       8192 MB / 16384 MB available
  env                     ✓       All 5 environment variables set
  files                   ✓       All 3 files OK

  Overall Status: ✓
```

## 4. Environment Management

Validate and reload environment variables.

### Commands

```bash
# Validate .env file
foundry env:validate

# Reload environment variables
foundry env:reload
```

### Validation Features

- Checks required variables
- Type validation (string, integer, boolean, URL, path)
- Shows missing variables
- Displays current values

### Example Output

```
Environment Validation Results
══════════════════════════════════════════════════
  ✓ DATABASE_URL = postgres://localhost/mydb
  ✓ APP_ENV = production
  ✗ REDIS_URL
      Required but not set

  2 / 3 checks passed
```

## 5. Asset Publishing

Publish static assets with cache busting support.

### Commands

```bash
# Publish assets with default settings
foundry asset:publish

# Custom source and target directories
foundry asset:publish --source assets --target public

# Disable versioning
foundry asset:publish --no-versioning
```

### Features

- Recursive directory processing
- Content-based hashing (SHA-256)
- Cache busting via versioned filenames
- Asset manifest generation
- File extension filtering
- Directory exclusion (.git, node_modules)

### Example

```bash
# Input: assets/app.js
# Output: public/app.a1b2c3d4.js

# Manifest: public/asset-manifest.json
{
  "assets": {
    "app.js": {
      "original": "app.js",
      "versioned": "app.a1b2c3d4.js",
      "hash": "a1b2c3d4e5f6...",
      "size": 12345
    }
  },
  "generated_at": "2025-11-01T14:30:00Z"
}
```

## Integration

All commands are automatically registered in the Foundry application:

```rust
// Commands are registered in foundry-application
use foundry_maintenance::{AppDownCommand, AppUpCommand};
use foundry_health::{HealthCheckCommand, DoctorCommand};
use foundry_env::{EnvValidateCommand, EnvReloadCommand};
use foundry_assets::AssetPublishCommand;
```

## Testing

Each crate includes comprehensive unit tests:

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p foundry-tinker-enhanced
cargo test -p foundry-maintenance
cargo test -p foundry-health
cargo test -p foundry-env
cargo test -p foundry-assets
```

## Dependencies

- `rustyline` - Command history and line editing
- `colored` - Terminal output coloring
- `sysinfo` - System information (disk, memory)
- `sha2` - Content hashing for assets
- `walkdir` - Recursive directory traversal
- `dirs` - User directory paths

## File Structure

```
crates/
├── foundry-tinker-enhanced/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── command.rs
│   │   ├── completer.rs
│   │   ├── helpers.rs
│   │   ├── highlighter.rs
│   │   ├── history.rs
│   │   ├── session.rs
│   │   └── repl.rs
│   └── Cargo.toml
├── foundry-maintenance/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── config.rs
│   │   ├── mode.rs
│   │   ├── middleware.rs
│   │   └── commands.rs
│   └── Cargo.toml
├── foundry-health/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── config.rs
│   │   ├── checks.rs
│   │   ├── report.rs
│   │   └── command.rs
│   └── Cargo.toml
├── foundry-env/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── validator.rs
│   │   └── commands.rs
│   └── Cargo.toml
└── foundry-assets/
    ├── src/
    │   ├── lib.rs
    │   ├── hasher.rs
    │   ├── manifest.rs
    │   ├── publisher.rs
    │   └── command.rs
    └── Cargo.toml
```
