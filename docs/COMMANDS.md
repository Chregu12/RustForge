# RustForge Command Reference

> **Complete CLI Command Documentation**

This document provides a comprehensive reference for all RustForge CLI commands.

---

## Quick Reference

```bash
foundry list          # List all commands
foundry help <cmd>    # Get help for specific command
foundry --version     # Show version
```

---

## Code Generation Commands

### make:model
Generate a new model class.

```bash
foundry make:model Post
foundry make:model Post -m          # With migration
foundry make:model Post -mcs        # With migration, controller, seeder
```

**Options**:
- `-m, --migration` - Create a migration file
- `-c, --controller` - Create a controller
- `-s, --seeder` - Create a seeder
- `-f, --factory` - Create a factory

### make:controller
Generate a controller class.

```bash
foundry make:controller PostController
foundry make:controller Api/PostController --api
```

**Options**:
- `--api` - Generate an API controller
- `--resource` - Generate a resource controller with CRUD methods

### make:migration
Create a new migration file.

```bash
foundry make:migration create_posts_table
foundry make:migration add_status_to_posts
```

### make:seeder
Generate a database seeder.

```bash
foundry make:seeder DatabaseSeeder
foundry make:seeder UserSeeder
```

### make:factory
Generate a model factory.

```bash
foundry make:factory PostFactory
foundry make:factory UserFactory
```

### make:job
Create a new job class.

```bash
foundry make:job ProcessEmail
foundry make:job ProcessEmail --async
```

**Options**:
- `--async` - Make job asynchronous
- `--queued` - Add to queue

### make:event
Generate an event class.

```bash
foundry make:event UserRegistered
foundry make:event OrderShipped
```

### make:listener
Create an event listener.

```bash
foundry make:listener SendWelcomeEmail
foundry make:listener SendWelcomeEmail --event=UserRegistered
```

**Options**:
- `--event=NAME` - Specify which event to listen to

### make:middleware
Generate middleware.

```bash
foundry make:middleware AuthenticateUser
foundry make:middleware RateLimitMiddleware
```

### make:request
Create a form request validator.

```bash
foundry make:request StorePostRequest
foundry make:request UpdateUserRequest
```

### make:command
Generate a custom CLI command.

```bash
foundry make:command SyncExternalAPI
foundry make:command CleanupOldRecords
```

### make:resource
Create an API resource.

```bash
foundry make:resource UserResource
foundry make:resource PostResource
```

### make:mail
Generate a mail class.

```bash
foundry make:mail WelcomeEmail
foundry make:mail OrderConfirmation
```

### make:notification
Create a notification class.

```bash
foundry make:notification UserWelcome
foundry make:notification OrderShipped
```

### make:form
Generate a form builder class.

```bash
foundry make:form UserForm
foundry make:form ContactForm
```

### make:export
Create an export class.

```bash
foundry make:export UsersExport
foundry make:export ReportExport
```

### make:admin-resource
Generate an admin panel resource.

```bash
foundry make:admin-resource User
foundry make:admin-resource Post
```

---

## Database Commands

### database:create
Interactive database setup wizard.

```bash
foundry database:create
foundry database:create --existing
foundry database:create --validate-only
```

**Flags**:
```bash
--driver=mysql              # Database driver
--host=localhost            # Database host
--port=3306                 # Database port
--root-user=root            # Root username
--root-password=secret      # Root password
--db-name=myapp             # Database name
--db-user=appuser           # Application user
--db-password=apppass       # Application password
--existing                  # Use existing database
--validate-only             # Only validate connection
```

### migrate
Run database migrations.

```bash
foundry migrate
foundry migrate --step=5     # Run next 5 migrations
foundry migrate --force      # Force in production
```

### migrate:rollback
Rollback the last migration.

```bash
foundry migrate:rollback
foundry migrate:rollback --step=2    # Rollback last 2 batches
```

### migrate:fresh
Drop all tables and re-run migrations.

```bash
foundry migrate:fresh
foundry migrate:fresh --seed         # With seeding
```

### migrate:reset
Rollback all migrations.

```bash
foundry migrate:reset
```

### migrate:status
Show migration status.

```bash
foundry migrate:status
```

### db:seed
Seed the database.

```bash
foundry db:seed
foundry db:seed --class=UserSeeder
```

### db:show
Show database information.

```bash
foundry db:show
foundry db:show --table=users
```

---

## Interactive Commands

### tinker
Start the interactive REPL console.

```bash
foundry tinker
```

**REPL Commands**:
```bash
find <table> <id>                 # Find record by ID
list <table>                      # List records
list <table> --limit 20           # List with limit
count <table>                     # Count records
all <table>                       # Get all records
create <table> {json}             # Create record
update <table> <id> {json}        # Update record
delete <table> <id>               # Delete record
sql <query>                       # Execute raw SQL
help                              # Show help
exit                              # Exit tinker
```

---

## Queue Commands

### queue:work
Start processing queued jobs.

```bash
foundry queue:work
foundry queue:work --tries=3
foundry queue:work --queue=high
foundry queue:work --timeout=60
```

**Options**:
- `--tries=N` - Number of attempts
- `--queue=NAME` - Queue name
- `--timeout=N` - Job timeout in seconds

### queue:failed
List failed jobs.

```bash
foundry queue:failed
```

### queue:retry
Retry failed jobs.

```bash
foundry queue:retry
foundry queue:retry <id>     # Retry specific job
```

### queue:forget
Delete failed job.

```bash
foundry queue:forget <id>
```

---

## Cache Commands

### cache:clear
Clear all caches.

```bash
foundry cache:clear
foundry cache:clear --tags=users,posts
```

### cache:forget
Remove specific cache key.

```bash
foundry cache:forget user:1
```

---

## Configuration Commands

### config:cache
Cache configuration files.

```bash
foundry config:cache
```

### config:clear
Clear configuration cache.

```bash
foundry config:clear
```

---

## Scheduling Commands

### schedule:run
Run scheduled tasks.

```bash
foundry schedule:run
```

### schedule:list
List all scheduled tasks.

```bash
foundry schedule:list
```

---

## Search Commands

### search:index
Index a model for searching.

```bash
foundry search:index User
foundry search:index Post
```

### search:reindex
Reindex all models.

```bash
foundry search:reindex
foundry search:reindex --force
```

---

## Audit Commands

### audit:list
Show audit logs.

```bash
foundry audit:list
foundry audit:list --model=User
foundry audit:list --user=1
```

### audit:show
Show audit log for specific record.

```bash
foundry audit:show users:1
foundry audit:show posts:42
```

---

## OAuth Commands

### oauth:list-providers
List configured OAuth providers.

```bash
foundry oauth:list-providers
```

### oauth:test
Test OAuth provider configuration.

```bash
foundry oauth:test google
foundry oauth:test github
```

---

## Export Commands

### export:pdf
Export data to PDF.

```bash
foundry export:pdf users output.pdf
foundry export:pdf --template=report users report.pdf
```

### export:excel
Export data to Excel.

```bash
foundry export:excel users output.xlsx
foundry export:excel --columns=id,name,email users output.xlsx
```

### export:csv
Export data to CSV.

```bash
foundry export:csv users output.csv
```

---

## Admin Commands

### admin:publish
Publish admin panel assets.

```bash
foundry admin:publish
foundry admin:publish --force
```

---

## Package Commands

### package:install
Install a package.

```bash
foundry package:install <name>
foundry package:install <name> --version=1.0.0
```

### package:remove
Remove a package.

```bash
foundry package:remove <name>
```

### package:update
Update all packages.

```bash
foundry package:update
foundry package:update --package=<name>
```

### package:list
List installed packages.

```bash
foundry package:list
foundry package:list --outdated
```

### package:search
Search for packages.

```bash
foundry package:search query
```

---

## Monitoring Commands

### route:list
List all registered routes.

```bash
foundry route:list
foundry route:list --method=GET
foundry route:list --name=user
```

### event:list
List all registered events.

```bash
foundry event:list
```

### websocket:info
Show WebSocket information.

```bash
foundry websocket:info
```

### websocket:stats
Show WebSocket statistics.

```bash
foundry websocket:stats
```

### metrics:report
Show performance metrics.

```bash
foundry metrics:report
foundry metrics:report --last-hour
```

### metrics:clear
Clear metrics.

```bash
foundry metrics:clear
```

---

## Development Commands

### serve
Start development server.

```bash
foundry serve
foundry serve --port=8080
foundry serve --host=0.0.0.0
```

**Options**:
- `--port=N` - Server port (default: 8000)
- `--host=IP` - Bind address (default: 127.0.0.1)

### test
Run tests.

```bash
foundry test
foundry test --filter=UserTest
foundry test --coverage
```

**Options**:
- `--filter=NAME` - Run specific test
- `--coverage` - Generate coverage report

### optimize
Optimize the application.

```bash
foundry optimize
```

---

## Utility Commands

### list
List all available commands.

```bash
foundry list
foundry list --category=make
```

### about
Show framework information.

```bash
foundry about
```

### env
Show environment variables.

```bash
foundry env
foundry env --key=DATABASE_URL
```

### vendor:publish
Publish vendor assets.

```bash
foundry vendor:publish
foundry vendor:publish --tag=stubs
foundry vendor:publish --tag=config
foundry vendor:publish --force
```

---

## Advanced Features

### Verbosity Levels

Control output detail:

```bash
foundry migrate -q          # Quiet mode
foundry migrate -v          # Verbose
foundry migrate -vv         # Very verbose
foundry migrate -vvv        # Debug mode
```

### Programmatic Execution

Execute commands from code:

```rust
use foundry_api::Artisan;

let result = artisan.call("migrate").dispatch().await?;
```

---

## Command Categories

- **Core**: list, about, help
- **Generator**: make:model, make:controller, etc.
- **Database**: migrate, db:seed, etc.
- **Runtime**: serve, queue:work, schedule:run
- **Utility**: cache:clear, config:cache, etc.

---

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Validation error
- `3` - Database error
- `4` - File system error

---

*Last Updated: 2025-11-06*
*RustForge v0.2.0*
