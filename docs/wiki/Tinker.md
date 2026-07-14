# Tinker — Interactive REPL

**Maturity:** `rf-tinker` [beta] · `rf-tinker-enhanced` [beta]
(tier definitions: [docs/TIERS.md](../../TIERS.md))

RustForge ships two REPL crates. Both are workspace members consumed as path
dependencies inside the monorepo — RustForge is not published to crates.io
(see [docs/RELEASING.md](../../RELEASING.md)).

---

## Table of contents

- [forge tinker — current CLI state](#forge-tinker--current-cli-state)
- [rf-tinker — the library crate](#rf-tinker--the-library-crate)
  - [Starting the REPL](#starting-the-repl)
  - [Meta-commands (dot-commands)](#meta-commands-dot-commands)
  - [DB facade queries](#db-facade-queries)
  - [Raw SQL](#raw-sql)
  - [Tab completion](#tab-completion)
  - [Syntax highlighting](#syntax-highlighting)
  - [History](#history)
  - [Known gaps in rf-tinker](#known-gaps-in-rf-tinker)
- [rf-tinker-enhanced — extended REPL](#rf-tinker-enhanced--extended-repl)
  - [Commands](#commands)
  - [Built-in helpers](#built-in-helpers)
  - [Session management](#session-management)
  - [Known gaps in rf-tinker-enhanced](#known-gaps-in-rf-tinker-enhanced)
- [Using the libraries directly](#using-the-libraries-directly)

---

## forge tinker — current CLI state

The `forge` binary (crate `forge-cli`, stable) exposes a `tinker` subcommand:

```
forge tinker
```

The alias `t` also resolves to `tinker` (`forge t`).

**Important:** the current CLI implementation (`crates/forge-cli/src/commands/tinker.rs`)
is a simplified placeholder. It reads from stdin with `io::stdin().read_line()` and
handles a small set of hard-coded commands (`help`, `exit`, `.clear`, `.models`). It
does not use `rustyline`, does not connect to the database, and the code comment
explicitly marks general input as "REPL evaluation not yet implemented." The model list
printed by `.models` is hard-coded example data (User, Post, Comment), not discovered
from your application.

The `rf-tinker` and `rf-tinker-enhanced` library crates (documented below) exist as
real implementations but are not yet wired into `forge tinker`. They can be used
directly from application code.

There is no `--database` flag on the `forge tinker` CLI command; the flag appears
only in `rf-tinker`'s module-level doc comment, not in the actual CLI argument
definition.

---

## rf-tinker — the library crate

**Tier:** beta — 7 files / ~1.4 k lines, real rustyline integration.

`rf-tinker` provides a `Tinker` struct you can instantiate and run programmatically.
Its REPL understands a `DB::` facade DSL (translated to SQL at parse time, not Rust
eval), raw SQL statements, and a set of dot-prefixed meta-commands.

### Starting the REPL

```rust
use rf_tinker::{Tinker, TinkerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = TinkerConfig {
        database_url: Some("sqlite://app.db".to_string()),
        history_file: Some(".tinker_history".to_string()),
        highlighting: true,
        completion: true,
        prompt: "Tinker> ".to_string(),
        max_history: 1000,
    };

    let mut tinker = Tinker::new(config);
    tinker.run().await
}
```

Or with default config and a live `sea_orm::DatabaseConnection`:

```rust
let tinker = Tinker::default().with_database(db_connection);
```

### Meta-commands (dot-commands)

All meta-commands begin with `.`. They are dispatched before any SQL or DB-facade
parsing.

| Command | Aliases | What it does |
|---------|---------|--------------|
| `.help` | `.h`, `.?` | Show help and query examples |
| `.exit` | `.quit`, `.q` | Exit the REPL |
| `.clear` | — | Clear the terminal screen |
| `.tables` | — | List all tables (Postgres, MySQL, SQLite) |
| `.schema <table>` | — | Show columns, types, and nullability for a table |
| `.databases` | — | List databases visible to the connection |
| `.reconnect` | — | Re-establish the configured database connection |
| `.env` | — | Print `APP_ENV`, `APP_DEBUG`, and a masked `DATABASE_URL` |
| `.history` | — | Print the path where history is saved |

`.tables` and `.schema` require an active database connection. Without one, a
warning is printed and no query is run.

### DB facade queries

`rf-tinker` parses `DB::` calls with a regex engine and translates them to SQL
at the point of input. This is **pattern matching, not Rust evaluation** — no
compilation takes place.

Supported patterns:

```
Tinker> DB::table("users").get()
-- translates to: SELECT * FROM users

Tinker> DB::table("users").where("id", 1).first()
-- translates to: SELECT * FROM users WHERE id = '1' LIMIT 1

Tinker> DB::table("users").where("active", true).limit(10)
-- translates to: SELECT * FROM users WHERE active = 'true' LIMIT 10

Tinker> DB::table("users").whereIn("role", ["admin","editor"])
-- translates to: SELECT * FROM users WHERE role IN ('admin','editor')

Tinker> DB::table("users").orderBy("created_at", "desc").limit(5)
-- translates to: SELECT * FROM users ORDER BY created_at DESC LIMIT 5

Tinker> DB::table("users").count()
-- translates to: SELECT COUNT(*) as count FROM users

Tinker> DB::select("SELECT id, email FROM users WHERE id < 10")
-- passes the string directly to the database

Tinker> DB::statement("UPDATE users SET active = 1 WHERE id = 42")
-- executes via db.execute(); returns affected row count
```

Column and table names are validated against `[a-zA-Z0-9_]` before interpolation
to prevent SQL injection through the DSL.

`Cache::` entries appear in tab completion (see below) but the executor does not
connect to the cache backend; entering a `Cache::` expression returns an
informational message.

### Raw SQL

Any input that begins with a recognised SQL keyword is forwarded directly to the
database:

```
Tinker> SELECT id, name FROM users LIMIT 5
Tinker> INSERT INTO tags (name) VALUES ('rust')
Tinker> PRAGMA table_info(users)
```

Recognised prefixes: `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE`, `DROP`,
`ALTER`, `SHOW`, `DESCRIBE`, `EXPLAIN`, `PRAGMA`.

### Tab completion

Tab completion is provided by `rustyline` via `TinkerCompleter`. Completions are
offered for:

- Dot-commands (`.help`, `.exit`, `.tables`, `.schema`, `.databases`, `.clear`,
  `.reconnect`, `.env`, `.history`)
- `DB::table`, `DB::select`, `DB::insert`, `DB::update`, `DB::delete`,
  `DB::statement`, `DB::raw`, `DB::transaction`
- Query-builder chain methods: `.get()`, `.first()`, `.count()`, `.where(`,
  `.whereIn(`, `.whereNull(`, `.whereNotNull(`, `.orWhere(`, `.orderBy(`,
  `.orderByDesc(`, `.limit(`, `.offset(`, `.join(`, `.leftJoin(`, `.select(`,
  `.distinct()`, `.groupBy(`, `.having(`, `.pluck(`, `.value(`, `.exists()`,
  `.doesntExist()`
- `Cache::get`, `Cache::put`, `Cache::forget`, `Cache::flush`
- Standard SQL keywords (`SELECT`, `FROM`, `WHERE`, `JOIN`, `ORDER BY`, `LIMIT`,
  `COUNT`, `GROUP BY`, etc.)

Completions match case-insensitively on the current word.

### Syntax highlighting

`TinkerHighlighter` colours input as it is typed:

| Element | Colour |
|---------|--------|
| SQL keywords | Bold blue |
| String literals (`'...'`, `"..."`) | Green |
| Numeric literals | Yellow |
| `DB::` / `Cache::` facade prefixes | Bold magenta |
| Dot meta-commands | Bold cyan |
| REPL prompt | Bold cyan |

### History

History is persisted to `.tinker_history` in the working directory (configurable
via `TinkerConfig::history_file`). Entries are loaded at startup and saved on
exit or Ctrl-D. Up/down arrow keys navigate history via rustyline's
`DefaultHistory`. The `.history` meta-command prints the path; it does not
display individual entries.

### Known gaps in rf-tinker

- **SQL result display is incomplete.** `execute_sql` calls `db.query_all()` and
  returns the row count as a message (`"N row(s) returned. Use raw SQL with explicit
  column selection for full output."`). The per-row JSON conversion is a placeholder
  in the current source. Column data is not printed.
- **`Cache::` is completion-only.** The executor does not connect to rf-cache;
  a `Cache::` expression prints an informational message and exits.
- **No Rust expression evaluation.** Only the `DB::` DSL and raw SQL are
  interpreted. Arbitrary Rust (`let x = 42;`) is passed to `execute_sql`, which will
  fail if the database rejects it.
- **`forge tinker` is not wired to this library.** See the section above.

---

## rf-tinker-enhanced — extended REPL

**Tier:** beta — 8 files / ~1.4 k lines.

`rf-tinker-enhanced` (`TinkerRepl`) layers persistent file-based history,
session save/load, and a richer set of built-in helpers on top of rustyline.
It is not currently wired to any CLI command.

### Commands

Commands in `rf-tinker-enhanced` are bare words (no leading dot):

| Command | Aliases | What it does |
|---------|---------|--------------|
| `help` | `?` | Show help |
| `exit` | `quit`, `q` | Exit the REPL |
| `clear` | `cls` | Clear the terminal screen |
| `history` | `hist` | Print the last 20 history entries |
| `helpers` | — | List built-in helper functions |
| `models` | — | Placeholder — prints "(No models loaded)" |
| `routes` | — | Placeholder — prints "(No routes loaded)" |
| `config <key>` | — | Read a config value set via `TinkerHelpers::set_config` |
| `env <key>` | — | Read an environment variable |
| `save <name>` | — | Save all commands entered this session to `~/.rustforge/sessions/<name>.json` |

Any other input is classified as `Execute(code)` — the execute branch prints
the code string and notes "Code execution not implemented - this is a demo."
There is no Rust eval engine.

### Built-in helpers

`TinkerHelpers` exposes functions shown by the `helpers` command:

| Helper | What it does |
|--------|-------------|
| `now()` | Returns the current UTC timestamp |
| `env(key, default?)` | Reads an environment variable |
| `config(key, default?)` | Reads a value from the in-session config map |
| `cache_get(key)` | Gets a value from an in-memory session cache |
| `cache_put(key, value)` | Stores a value in the in-memory session cache |
| `db_query(sql)` | Listed in the helper table but not implemented in the executor |
| `dd(value)` | Pretty-prints a `serde_json::Value` (dump-and-die style) |

The `cache_get`/`cache_put` helpers use an in-memory `HashMap` scoped to the
REPL session — they are not connected to `rf-cache` or Redis.

### Session management

Sessions are persisted to `~/.rustforge/sessions/` as JSON files. History is
persisted to `~/.rustforge/tinker_history`. Both paths are created on first use.

```
tinker> helpers
tinker> env DATABASE_URL
tinker> save my_session
-- Writes ~/.rustforge/sessions/my_session.json
```

`SessionManager` also supports `load(name)`, `list()`, and `delete(name)`.

### Known gaps in rf-tinker-enhanced

- **No Rust eval.** The `Execute` branch is explicitly a demo stub.
- **`models` and `routes` are placeholder.** Both print a message that nothing
  is loaded.
- **Not wired to a CLI.** There is no `forge` or `foundry` subcommand that
  starts `TinkerRepl`.
- **`db_query` listed but not implemented** in the `execute_command` match arm.

---

## Using the libraries directly

Add the crates as path dependencies in a workspace that includes RustForge:

```toml
[dependencies]
rf-tinker          = { path = "path/to/RustForge/crates/rf-tinker" }
# or
rf-tinker-enhanced = { path = "path/to/RustForge/crates/rf-tinker-enhanced" }
```

They are workspace members of the RustForge monorepo; there are no crates.io
releases to reference with a version string.

For the full stable surface and the import pattern for application code see
[Features.md](Features.md), [Laravel-Syntax.md](Laravel-Syntax.md), and
[Home.md](Home.md).
