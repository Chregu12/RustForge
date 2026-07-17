# Foundry Tinker Enhanced

Advanced REPL for Foundry with command history, autocomplete, and built-in helpers.

## Features

- **Command History**: Persistent history stored in `~/.rustforge/tinker_history`
- **Tab Completion**: Auto-complete for commands, models, and methods
- **Syntax Highlighting**: Color-coded output for better readability
- **Built-in Helpers**: Quick access to common functions
- **Session Management**: Save sessions as executable scripts

## Usage

```bash
# Start tinker REPL
foundry tinker

# In tinker
tinker> helpers              # Show available helpers
tinker> models               # List models
tinker> config app.name      # Show config value
tinker> env DATABASE_URL     # Show env variable
tinker> save my_session      # Save current session
tinker> history              # Show command history
tinker> clear                # Clear screen
tinker> exit                 # Exit REPL
```

## Built-in Helpers

- `now()` - Get current timestamp
- `env(key, default)` - Get environment variable
- `config(key, default)` - Get configuration value
- `cache_get(key)` - Get from cache
- `cache_put(key, value)` - Store in cache
- `db_query(sql)` - Execute raw SQL
- `dd(value)` - Dump and die (pretty print)

## Example Session

```rust
tinker> let user = User::find(1)
tinker> dd(user)
{
  "id": 1,
  "name": "John Doe",
  "email": "john@example.com"
}

tinker> save user_lookup
Session saved as 'user_lookup'
```
