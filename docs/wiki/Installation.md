# Installation

RustForge is a **git monorepo** — it is not published to crates.io.
You consume it as a git dependency (pinned to a release tag) or as a path
dependency when working inside the repository itself.

---

## Prerequisites

### Rust toolchain

| Requirement | Value |
|---|---|
| Minimum Supported Rust Version (MSRV) | **1.79.0** |
| Tested / pinned toolchain (CI and local dev) | **1.96.0** |

The workspace ships a `rust-toolchain.toml` that pins the toolchain to 1.96.0.
When you work inside the repository or use a path dependency, this pin is picked
up automatically by `cargo`. When you consume RustForge as a git dependency from
your own project, your own toolchain is used — you need Rust >= 1.79.0.

Install or update Rust via [rustup](https://rustup.rs/):

```sh
# Install rustup (if you don't have it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Check your version
rustc --version   # must be >= 1.79.0

# Update to the latest stable if needed
rustup update stable
```

### Database (optional)

The ORM macros (`Model!`, `create!`, `find!`, `update!`, `delete!`) and the `DB`
facade default to **in-memory SQLite** (rusqlite, zero-config). For Postgres, set
`DATABASE_URL` to a `postgres://` URL — see [database support](#database-support).
MySQL is not supported.

---

## Adding RustForge to your project

### Option A — Git dependency (recommended)

Pin to the latest release tag in your project's `Cargo.toml`:

```toml
[dependencies]
# Umbrella crate — pulls in the full stable surface via `use rf::prelude::*`
rf = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }

# Async runtime (required)
tokio = { version = "1", features = ["full"] }

# Serialization (required for Model! derive)
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP layer (required; must match rf-routing's axum version)
axum = "0.8"
```

To track `main` instead (unstable, not recommended for production):

```toml
rf = { git = "https://github.com/Chregu12/RustForge", branch = "main" }
```

### Option B — Path dependency (contributors / monorepo)

If you have cloned the repository or embedded it as a git submodule:

```toml
[dependencies]
rf = { path = "../RustForge/crates/rf" }

tokio = { version = "1", features = ["full"] }
serde  = { version = "1", features = ["derive"] }
serde_json = "1"
axum   = "0.8"
```

### Option C — Individual crates

You can depend on individual stable crates rather than the umbrella:

```toml
[dependencies]
rf-validation = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-auth       = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-cache      = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

See [docs/STABLE_CORE.md](../STABLE_CORE.md) for the full list of stable crates and
[docs/TIERS.md](../TIERS.md) for their maturity status.

---

## Verifying the setup

Run an example from the repo to confirm everything builds and links correctly:

```sh
git clone https://github.com/Chregu12/RustForge.git
cd RustForge

# Build and run the blog-slice example (serves on http://127.0.0.1:3000)
cargo run -p blog-slice

# In a second terminal:
curl -X POST http://127.0.0.1:3000/posts \
  -H "Content-Type: application/json" \
  -d '{"title":"Hello","body":"World"}'

curl http://127.0.0.1:3000/posts
```

You should see a JSON array containing the post you just created. If that works,
your toolchain and the dependency graph are correct.

---

## Database support

The `DB` facade and all ORM macros default to **SQLite** and require no
configuration. To use Postgres, set the `DATABASE_URL` environment variable
before starting your app:

```sh
# In-memory SQLite (default — data is lost when the process exits)
cargo run -p my-app

# Persistent SQLite file
DATABASE_URL=./app.db cargo run -p my-app

# Postgres
DATABASE_URL=postgres://user:pass@localhost/mydb cargo run -p my-app
```

The backend is selected automatically at runtime: a `postgres://` or
`postgresql://` URL routes to Postgres via sqlx; anything else stays SQLite.

**Postgres caveats:**
- The primary key column must be named `id` (framework convention; `RETURNING id` is appended on INSERT).
- `NUMERIC`/`DECIMAL` columns are not decoded to JSON yet — cast to `TEXT` or `FLOAT8` in the query.

---

## Installing the forge CLI (optional)

The `forge` CLI generates compiling scaffold code (`make:model`, `make:controller`,
`make:request`, `make:migration`, `forge deploy generate`). It is part of the
monorepo and must be installed from the cloned repository:

```sh
git clone https://github.com/Chregu12/RustForge.git
cd RustForge
cargo install --path crates/forge-cli
forge --version
```

---

## Experimental crates

The following crates are **excluded from the v1.0 stable surface** and carry no
compatibility guarantee: `rf-nova`, `rf-swagger`, `rf-telescope`, `rf-cms`,
`rf-breeze`, `rf-vite`, `rf-livereload`. They are not pulled in by `rf` (the
umbrella) or by a plain `cargo build` inside the workspace. See
[docs/TIERS.md](../TIERS.md) for the full tier taxonomy.

---

## Next steps

- [Quick Start](Quick-Start) — build and run your first app end-to-end
- [docs/STABLE_CORE.md](../STABLE_CORE.md) — the v1 API contract and every entry point
- [docs/TIERS.md](../TIERS.md) — maturity tier for every crate
- [docs/COOKBOOK.md](../COOKBOOK.md) — task-oriented recipes with real snippets
