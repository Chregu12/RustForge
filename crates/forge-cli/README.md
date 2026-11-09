# Forge CLI

Create a RustForge application from the Laravel-style starter.

```bash
cargo install --path crates/forge-cli --locked
forge new blog
cd blog
forge migrate
forge db:seed
forge serve
```

The server listens on `127.0.0.1:8000` by default. The welcome page is at `/`, the health check at `/up`, and the JSON posts resource at `/api/posts`.

```bash
forge route:list --path api
curl -i -X POST http://127.0.0.1:8000/api/posts \
  -H 'Content-Type: application/json' \
  -d '{"title":"First post","body":"Hello RustForge"}'
```

The generated project pins a RustForge Git revision. Use `forge new blog --framework-path /path/to/RustForge` when developing against a local checkout. Projects use Rust 1.96 or later.

## Application layout

| Path | Role |
| --- | --- |
| `bootstrap/app.rs` | Assemble the application routes |
| `config/app.rs` | Load listener settings and database URL |
| `routes/web.rs`, `routes/api.rs` | Define HTML and API entry points |
| `app/http/controllers`, `app/http/requests` | Handle requests and validate input |
| `app/models` | Define ORM models |
| `database/migrations`, `database/seeders` | Store reversible SQL migrations and seed data |
| `resources/views` | Store the starter's HTML template |

Copy `.env.example` to `.env` to set `DATABASE_URL`, `HOST`, or `PORT`. Exported variables take precedence. The starter uses file-backed SQLite at `database/database.sqlite`. PostgreSQL requires migrations written for PostgreSQL.

## Database commands

| Command | Effect |
| --- | --- |
| `forge migrate` | Apply pending migrations in one batch |
| `forge migrate --step` | Apply pending migrations in separate batches |
| `forge migrate:status` | Show pending and applied migrations |
| `forge migrate:rollback` | Roll back the last batch |
| `forge migrate:rollback --step 2` | Roll back the two latest migrations |
| `forge migrate:rollback --batch 3` | Roll back batch 3 |
| `forge migrate:reset` | Roll back all migrations |
| `forge migrate:refresh --seed` | Roll back and reapply migrations, then run seeders |
| `forge migrate:fresh --force --seed` | Drop all tables, migrate and seed |
| `forge make:model Topic --migration` | Generate a model and a matching migration |
| `forge make:migration create_topics_table` | Generate an `up.sql` and `down.sql` pair |
| `forge db:seed --class Posts` | Execute `database/seeders/posts.sql` |

Migration scripts run in a transaction with their history entry. Write each script for the configured database, without explicit transaction control. Forge also reads the older `src/migrations` and `migrations` locations. Nested command spellings such as `forge migrate status` remain accepted. Database writes in `APP_ENV=production` require `--force`. `migrate:fresh` always requires it.

To run the starter integration test from the RustForge workspace:

```bash
cargo build --locked -p forge-cli
python3 scripts/forge-starter-smoke.py
```

The starter uses SQLite, compiled routing and a static HTML view. The framework's optional packages and other CLI commands have separate maturity levels.
