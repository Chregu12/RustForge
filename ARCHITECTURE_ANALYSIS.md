# RustForge Command System Architecture Analysis

## Executive Summary

RustForge implements a **plugin-based command architecture** with clear separation of concerns:
- **CLI Entry Point** (`foundry-cli`): Parses arguments and dispatches to the kernel
- **Kernel/Dispatcher** (`FoundryApp`): Routes commands via `CommandRegistry`
- **Trait-Based Execution** (`FoundryCommand`): Commands implement async trait with metadata
- **Service Container**: `Container` provides dependency injection for commands
- **HTTP/MCP Gateway** (`foundry-api`): Exposes commands via REST API for programmatic access

---

## 1. Command Structure and Registration

### 1.1 Command Trait Definition

Located in `/crates/foundry-plugins/src/lib.rs`:

```rust
#[async_trait]
pub trait FoundryCommand: Send + Sync {
    fn descriptor(&self) -> &CommandDescriptor;
    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError>;
}
```

Commands must implement:
- **descriptor()**: Returns metadata about the command (ID, name, summary, category, aliases)
- **execute()**: Async method containing command logic, receives `CommandContext`

### 1.2 CommandDescriptor

Defines command metadata:

```rust
pub struct CommandDescriptor {
    pub id: CommandId,              // Unique ID (e.g., "database.migrate")
    pub name: String,                // Command name (e.g., "migrate")
    pub summary: String,              // Short description
    pub description: Option<String>,   // Detailed description
    pub category: CommandKind,         // Core, Generator, Database, Runtime, Utility
    pub aliases: Vec<String>,          // Alternative names (e.g., "db:migrate")
}
```

Built with builder pattern:
```rust
let descriptor = CommandDescriptor::builder("database.migrate", "migrate")
    .summary("Executes all pending migrations")
    .description("Detailed description...")
    .category(CommandKind::Database)
    .alias("db:migrate")
    .build();
```

### 1.3 Command Registration

Location: `/crates/foundry-application/src/registry.rs`

```rust
pub struct CommandRegistry {
    inner: Arc<Mutex<RegistryState>>,  // Thread-safe with Arc<Mutex>
}

impl CommandRegistry {
    pub fn register(&self, command: DynCommand) -> Result<(), ApplicationError>
    pub fn resolve(&self, command: &str) -> Option<DynCommand>
    pub fn descriptors(&self) -> Vec<CommandDescriptor>
}
```

**Key Features:**
- Case-insensitive command lookup (converts to lowercase)
- Aliases supported - multiple keys map to same command index
- `DynCommand = Arc<dyn FoundryCommand>` for reference counting

**Bootstrap Process** (`/crates/foundry-application/src/commands/mod.rs`):

```rust
impl BootstrapCommands {
    pub fn register_all(registry: &CommandRegistry) -> Result<(), ApplicationError> {
        // Register 50+ commands in FoundryApp::build()
        let list = Arc::new(ListCommand::new(registry.clone()));
        registry.register(list)?;
        // ... more registrations
    }
}
```

---

## 2. Main Command Execution Logic (Kernel/Dispatcher)

### 2.1 FoundryApp - The Kernel

Location: `/crates/foundry-application/src/lib.rs`

```rust
#[derive(Clone)]
pub struct FoundryApp {
    registry: CommandRegistry,
    config: Value,
    artifacts: Arc<dyn ArtifactPort>,
    migrations: Arc<dyn MigrationPort>,
    seeds: Arc<dyn SeedPort>,
    storage_manager: Arc<StorageManager>,
    validation: Arc<dyn ValidationPort>,
    storage: Arc<dyn StoragePort>,
    cache: Arc<dyn CachePort>,
    queue: Arc<dyn QueuePort>,
    events: Arc<dyn EventPort>,
    container: Container,
    providers: ProviderRegistry,
}
```

**Core Dispatch Method:**

```rust
pub async fn dispatch(
    &self,
    command: &str,
    args: Vec<String>,
    format: ResponseFormat,
    options: ExecutionOptions,
) -> Result<CommandResult, ApplicationError> {
    // 1. Resolve command from registry
    let handle = self
        .registry
        .resolve(command)
        .ok_or_else(|| ApplicationError::CommandNotFound(command.to_string()))?;

    // 2. Build metadata
    let catalog = self.registry.descriptors();
    let metadata = serde_json::json!({
        "invocation": {
            "command": command,
            "args": args_snapshot,
            "format": format,
            "options": options,
        },
        "catalog": catalog,
    });

    // 3. Create CommandContext with all services
    let ctx = CommandContext {
        args,
        format,
        metadata,
        config: self.config.clone(),
        options,
        artifacts: self.artifacts.clone(),
        migrations: self.migrations.clone(),
        seeds: self.seeds.clone(),
        validation: self.validation.clone(),
        storage: self.storage.clone(),
        cache: self.cache.clone(),
        queue: self.queue.clone(),
        events: self.events.clone(),
    };

    // 4. Execute command
    let result = handle.execute(ctx).await?;
    Ok(result)
}
```

**Flow Diagram:**
```
CLI args → FoundryApp::dispatch()
  ├→ registry.resolve(command_name)
  ├→ Build CommandContext with services
  ├→ handle.execute(ctx)
  └→ Return CommandResult
```

### 2.2 Bootstrap and Initialization

```rust
pub fn bootstrap(
    config: Value,
    artifacts: Arc<dyn ArtifactPort>,
    migrations: Arc<dyn MigrationPort>,
    seeds: Arc<dyn SeedPort>,
) -> Result<Self, ApplicationError> {
    FoundryAppBuilder::new(config, artifacts, migrations, seeds).build()
}
```

**Builder Pattern** for customization:

```rust
pub struct FoundryAppBuilder { ... }

impl FoundryAppBuilder {
    pub fn with_validation_port(mut self, port: Arc<dyn ValidationPort>) -> Self
    pub fn with_storage_port(mut self, port: Arc<dyn StoragePort>) -> Self
    pub fn with_cache_port(mut self, port: Arc<dyn CachePort>) -> Self
    pub fn with_queue_port(mut self, port: Arc<dyn QueuePort>) -> Self
    pub fn with_event_port(mut self, port: Arc<dyn EventPort>) -> Self
    pub fn with_container(mut self, container: Container) -> Self
    pub fn with_providers(mut self, providers: ProviderRegistry) -> Self
}
```

---

## 3. Output Handling

### 3.1 Response Formats

Location: `/crates/foundry-plugins/src/lib.rs`

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Human,     // Pretty-printed output for CLI
    Json,      // JSON serialization for programmatic access
}
```

### 3.2 CommandResult Structure

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub status: CommandStatus,           // Success, Failure, Skipped
    pub message: Option<String>,          // Human-readable message
    pub data: Option<Value>,              // Structured JSON data
    pub error: Option<AppError>,          // Error details if failed
}

pub enum CommandStatus {
    Success,
    Failure,
    Skipped,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self
    pub fn with_data(mut self, data: Value) -> Self
    pub fn failure(error: AppError) -> Self
    pub fn skipped(message: impl Into<String>) -> Self
}
```

### 3.3 CLI Output Rendering

Location: `/crates/foundry-cli/src/main.rs` (line 87-109)

```rust
fn render_result(result: &CommandResult, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => {
            // Print message to stdout
            if let Some(message) = &result.message {
                println!("{message}");
            }
            // Print errors to stderr
            if let Some(error) = &result.error {
                eprintln!("Error [{}]: {}", error.code, error.message);
                // Print context fields
            }
            Ok(())
        }
        OutputFormat::Json => {
            // Pretty-print JSON
            let payload = serde_json::to_string_pretty(result)?;
            println!("{payload}");
            Ok(())
        }
    }
}
```

### 3.4 HTTP Response Envelope

Location: `/crates/foundry-api/src/http.rs` (line 216-247)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpEnvelope {
    pub status: CommandStatus,
    pub message: Option<String>,
    pub data: Option<Value>,
    pub error: Option<AppError>,
}

impl From<CommandResult> for HttpEnvelope {
    fn from(result: CommandResult) -> Self {
        Self {
            status: result.status,
            message: result.message,
            data: result.data,
            error: result.error,
        }
    }
}
```

**HTTP Status Code Mapping:**

```rust
let http_status = match result.status {
    CommandStatus::Success | CommandStatus::Skipped => StatusCode::OK,
    CommandStatus::Failure => StatusCode::UNPROCESSABLE_ENTITY,
};
```

---

## 4. Input Reception (Arguments & Options)

### 4.1 CommandContext

Location: `/crates/foundry-plugins/src/lib.rs` (line 14-41)

```rust
#[derive(Clone)]
pub struct CommandContext {
    pub args: Vec<String>,              // Raw command arguments
    pub format: ResponseFormat,         // Output format (Human/Json)
    pub metadata: Value,                // Invocation metadata with catalog
    pub config: Value,                  // Environment variables as JSON
    pub options: ExecutionOptions,      // dry_run, force flags
    pub artifacts: Arc<dyn ArtifactPort>,
    pub migrations: Arc<dyn MigrationPort>,
    pub seeds: Arc<dyn SeedPort>,
    pub validation: Arc<dyn ValidationPort>,
    pub storage: Arc<dyn StoragePort>,
    pub cache: Arc<dyn CachePort>,
    pub queue: Arc<dyn QueuePort>,
    pub events: Arc<dyn EventPort>,
}
```

### 4.2 ExecutionOptions

```rust
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ExecutionOptions {
    #[serde(default)]
    pub dry_run: bool,    // Simulate without side effects
    #[serde(default)]
    pub force: bool,      // Force overwrite existing artifacts
}
```

### 4.3 CLI Argument Parsing

Location: `/crates/foundry-cli/src/main.rs` (line 22-54)

```rust
#[derive(Parser, Debug)]
struct Cli {
    #[arg(long, value_enum, default_value_t = OutputFormat::Human, global = true)]
    format: OutputFormat,
    
    #[arg(short, long, help = "Verbose logging")]
    verbose: bool,
    
    #[arg(long, help = "Simulate without side effects", global = true)]
    dry_run: bool,
    
    #[arg(long, help = "Force overwrite artifacts", global = true)]
    force: bool,
    
    #[arg(value_name = "COMMAND")]
    command: Option<String>,
    
    #[arg(value_name = "ARGS", trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,  // Raw args passed to command
}
```

**Argument Flow:**

```
$ foundry migrate --name 2024_01_01_create_users
   ↓
command = "migrate"
args = ["--name", "2024_01_01_create_users"]
   ↓
FoundryApp::dispatch(command, args, format, options)
```

### 4.4 Command-Specific Argument Parsing

Commands parse `ctx.args` directly. Example from `MigrateSeedCommand`:

```rust
async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
    if !ctx.args.is_empty() {
        return Err(CommandError::Message(
            "`migrate:seed` accepts no additional arguments".into(),
        ));
    }
    // ... execute logic
}
```

Access config values:

```rust
fn config_value(ctx: &CommandContext, key: &str) -> Option<String> {
    ctx.config
        .as_object()
        .and_then(|map| map.get(key))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}
```

---

## 5. Error Handling

### 5.1 Error Hierarchy

**CommandError** (plugin layer):
```rust
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Message(String),
    
    #[error("serialization error: {0}")]
    Serialization(String),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

**ApplicationError** (application layer):
```rust
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("command '{0}' already registered")]
    CommandAlreadyRegistered(String),
    
    #[error("command '{0}' not found")]
    CommandNotFound(String),
    
    #[error("command execution failed")]
    CommandExecution(#[source] CommandError),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl From<CommandError> for ApplicationError {
    fn from(err: CommandError) -> Self {
        ApplicationError::CommandExecution(err)
    }
}
```

**AppError** (HTTP/API layer):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,               // Machine-readable error code
    pub message: String,             // Human-readable message
    pub status: u16,                // HTTP status code
    pub context: Vec<ErrorContextField>,  // Field-specific errors
    #[serde(skip)]
    source: Option<Arc<dyn StdError + Send + Sync>>,
}

pub struct ErrorContextField {
    pub key: String,
    pub value: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self
    pub fn not_found(resource: impl Into<String>) -> Self
    pub fn unauthorized() -> Self
    pub fn forbidden() -> Self
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self
    pub fn with_status(mut self, status: u16) -> Self
    pub fn status_code(&self) -> StatusCode
}
```

### 5.2 Error Flow in CLI

Location: `/crates/foundry-cli/src/main.rs` (line 338-378)

```rust
let outcome = runtime.block_on(app.dispatch(&command, command_args, response_format, options));

match outcome {
    Ok(result) => {
        // Log success to audit
        let record = AuditRecord::from_success(...);
        audit_logger.log(&record)?;
        
        // Fail if command status is Failure
        if result.status == CommandStatus::Failure {
            bail!(message);
        }
        
        render_result(&result, output_format)?;
        Ok(())
    }
    Err(err) => {
        // Log error to audit
        let record = AuditRecord::from_error(...);
        audit_logger.log(&record)?;
        Err(eyre!(err))
    }
}
```

### 5.3 HTTP Error Handling

Location: `/crates/foundry-api/src/http.rs` (line 257-289)

```rust
async fn invoke(
    State(state): State<AppState>,
    payload: AppJson<InvocationRequest>,
) -> Result<(StatusCode, Json<HttpEnvelope>), HttpError> {
    let request = payload.into_inner();
    
    match state.invoker.invoke(request).await {
        Ok(result) => {
            let status = if let Some(error) = &result.error {
                error.status_code()  // Use error's HTTP status
            } else {
                match result.status {
                    CommandStatus::Success | CommandStatus::Skipped => StatusCode::OK,
                    CommandStatus::Failure => StatusCode::UNPROCESSABLE_ENTITY,
                }
            };
            Ok((status, Json(HttpEnvelope::from(result))))
        }
        Err(err) => {
            let error = map_application_error(err);
            Err(HttpError::from(error))
        }
    }
}

fn map_application_error(err: ApplicationError) -> AppError {
    match err {
        ApplicationError::CommandNotFound(cmd) => 
            AppError::new("COMMAND_NOT_FOUND", format!("Command `{cmd}` not found"))
                .with_status(StatusCode::NOT_FOUND.as_u16()),
        ApplicationError::CommandExecution(inner) => 
            AppError::new("COMMAND_EXECUTION_ERROR", format!("Execution Error: {inner}"))
                .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
        // ...
    }
}
```

---

## 6. Service Container Integration

### 6.1 Container Architecture

Location: `/crates/foundry-application/src/lib.rs`

```rust
pub struct FoundryApp {
    // ...
    container: Container,
    providers: ProviderRegistry,
}

impl FoundryApp {
    pub fn container(&self) -> Container {
        self.container.clone()
    }

    pub fn providers(&self) -> ProviderRegistry {
        self.providers.clone()
    }
}
```

**Re-exported from `foundry_service_container`:**

```rust
pub use foundry_service_container::{
    Container, ProviderRegistry, ServiceProvider,
    ApplicationServiceProvider,
    AuthServiceProvider,
    CacheServiceProvider,
    DatabaseServiceProvider,
    MailServiceProvider,
};
```

### 6.2 Service Ports

Commands access services via injected trait objects in `CommandContext`:

```rust
pub struct CommandContext {
    pub artifacts: Arc<dyn ArtifactPort>,
    pub migrations: Arc<dyn MigrationPort>,
    pub seeds: Arc<dyn SeedPort>,
    pub validation: Arc<dyn ValidationPort>,
    pub storage: Arc<dyn StoragePort>,
    pub cache: Arc<dyn CachePort>,
    pub queue: Arc<dyn QueuePort>,
    pub events: Arc<dyn EventPort>,
}
```

**Key Traits:**

```rust
pub trait ArtifactPort: Send + Sync {
    fn write_file(&self, path: &str, contents: &str, force: bool) -> Result<(), CommandError>;
}

#[async_trait]
pub trait MigrationPort: Send + Sync {
    async fn apply(&self, config: &Value, dry_run: bool) -> Result<MigrationRun, CommandError>;
    async fn rollback(&self, config: &Value, dry_run: bool) -> Result<MigrationRun, CommandError>;
}

#[async_trait]
pub trait SeedPort: Send + Sync {
    async fn run(&self, config: &Value, dry_run: bool) -> Result<SeedRun, CommandError>;
}

#[async_trait]
pub trait ValidationPort: Send + Sync {
    async fn validate(&self, payload: Value, rules: ValidationRules) 
        -> Result<ValidationReport, CommandError>;
}

#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn put(&self, disk: &str, path: &str, contents: Vec<u8>) 
        -> Result<StoredFile, CommandError>;
    async fn get(&self, disk: &str, path: &str) -> Result<Vec<u8>, CommandError>;
    async fn delete(&self, disk: &str, path: &str) -> Result<(), CommandError>;
    async fn exists(&self, disk: &str, path: &str) -> Result<bool, CommandError>;
    async fn url(&self, disk: &str, path: &str) -> Result<String, CommandError>;
}

#[async_trait]
pub trait CachePort: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>, CommandError>;
    async fn put(&self, key: &str, value: Value, ttl: Option<Duration>) 
        -> Result<(), CommandError>;
    async fn forget(&self, key: &str) -> Result<(), CommandError>;
    async fn clear(&self, prefix: Option<&str>) -> Result<(), CommandError>;
}

#[async_trait]
pub trait QueuePort: Send + Sync {
    async fn dispatch(&self, job: QueueJob) -> Result<(), CommandError>;
}

#[async_trait]
pub trait EventPort: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<(), CommandError>;
}
```

### 6.3 Default Implementations

Location: `/crates/foundry-application/src/lib.rs` (line 235-253)

```rust
let validation = validation.unwrap_or_else(|| Arc::new(SimpleValidationService));
let storage = storage
    .unwrap_or_else(|| Arc::new(FileStorageAdapter::new(storage_manager.clone())));
let cache = cache.unwrap_or_else(|| Arc::new(InMemoryCacheStore::default()));
let queue = queue.unwrap_or_else(|| Arc::new(InMemoryQueue::default()));
let events = events.unwrap_or_else(|| Arc::new(InMemoryEventBus::default()));
```

**Initialization in FoundryApp::build():**

```rust
Ok(FoundryApp {
    registry,
    config,
    artifacts,
    migrations,
    seeds,
    storage_manager: storage_manager.clone(),
    validation,
    storage,
    cache,
    queue,
    events,
    container,
    providers,
})
```

---

## 7. Programmatic Command Execution

### 7.1 FoundryInvoker API

Location: `/crates/foundry-api/src/invocation.rs`

```rust
#[derive(Clone)]
pub struct FoundryInvoker {
    app: FoundryApp,
}

impl FoundryInvoker {
    pub fn new(app: FoundryApp) -> Self {
        Self { app }
    }

    pub fn descriptors(&self) -> Vec<CommandDescriptor> {
        self.app.registry().descriptors()
    }

    pub fn app(&self) -> &FoundryApp {
        &self.app
    }
}

#[async_trait]
impl CommandInvoker for FoundryInvoker {
    #[instrument(name = "foundry.invoke", skip(self))]
    async fn invoke(&self, request: InvocationRequest) -> Result<CommandResult, ApplicationError> {
        self.app
            .dispatch(
                &request.command,
                request.args.clone(),
                request.format.clone(),
                request.options,
            )
            .await
    }
}
```

### 7.2 InvocationRequest Structure

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvocationRequest {
    #[serde(default)]
    pub command: String,
    
    #[serde(default)]
    pub args: Vec<String>,
    
    #[serde(default = "default_format")]
    pub format: ResponseFormat,
    
    #[serde(default)]
    pub correlation_id: Option<String>,
    
    #[serde(default)]
    pub options: ExecutionOptions,
}

impl InvocationRequest {
    pub fn new(command: impl Into<String>) -> Self { ... }
    pub fn with_args(mut self, args: Vec<String>) -> Self { ... }
    pub fn with_format(mut self, format: ResponseFormat) -> Self { ... }
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self { ... }
    pub fn with_options(mut self, options: ExecutionOptions) -> Self { ... }
}
```

### 7.3 HTTP Invoke Endpoint

Location: `/crates/foundry-api/src/http.rs` (line 257-289)

```
POST /invoke

Request:
{
    "command": "migrate",
    "args": ["--name", "2024_01_01_create_users"],
    "format": "json",
    "options": { "dry_run": false, "force": false }
}

Response:
{
    "status": "success",
    "message": "migrate → 5 migration(s) applied",
    "data": {
        "plan": { ... },
        "run": { "applied": [...], "skipped": [...] },
        "input": { "args": [...] }
    }
}
```

---

## 8. Complete Flow Diagram

### CLI Flow
```
foundry migrate --name 2024_create_users
    ↓
Cli::parse() → Command="migrate", Args=["--name", "2024_create_users"]
    ↓
load_config() → Read .env as Value
    ↓
FoundryApp::bootstrap(config, artifacts, migrations, seeds)
    ├→ CommandRegistry::register_all() → 50+ commands
    ├→ Initialize service ports (cache, storage, queue, events)
    └→ Create Container and ProviderRegistry
    ↓
app.dispatch("migrate", args, ResponseFormat::Human, ExecutionOptions)
    ├→ registry.resolve("migrate") → MigrateCommand
    ├→ Build CommandContext with all services
    ├→ MigrateCommand::execute(ctx)
    │   ├→ Access ctx.args
    │   ├→ Access ctx.config values
    │   ├→ Use ctx.migrations.apply()
    │   └→ Return CommandResult
    └→ Result
    ↓
render_result(result, OutputFormat::Human)
    ├→ Print message to stdout
    ├→ Print errors to stderr
    └→ Exit with success/failure
```

### HTTP/API Flow
```
POST /invoke
{
    "command": "migrate",
    "args": ["--name", "2024_create_users"],
    "format": "json"
}
    ↓
FoundryInvoker::invoke(InvocationRequest)
    ↓
app.dispatch(...)  [Same as CLI, but async]
    ↓
HttpEnvelope::from(CommandResult)
    ↓
Response 200/422
{
    "status": "success",
    "message": "...",
    "data": {...}
}
```

---

## 9. Key Implementation Examples

### Example: Simple Command

```rust
pub struct ListCommand {
    descriptor: CommandDescriptor,
    registry: CommandRegistry,
}

impl ListCommand {
    pub fn new(registry: CommandRegistry) -> Self {
        let descriptor = CommandDescriptor::builder("core.list", "list")
            .summary("Lists all available commands")
            .category(CommandKind::Core)
            .alias("ls")
            .build();
        Self { descriptor, registry }
    }
}

#[async_trait]
impl FoundryCommand for ListCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let catalog = self.registry.descriptors();
        
        let message = match ctx.format {
            ResponseFormat::Human => {
                let table = render_table(&catalog);
                table.render()
            }
            ResponseFormat::Json => "commands available".to_string(),
        };

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(json!({
                "total": catalog.len(),
                "commands": catalog,
            })),
            error: None,
        })
    }
}
```

### Example: Migration Command

```rust
#[async_trait]
impl FoundryCommand for MigrateCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Validate arguments
        if !ctx.args.is_empty() {
            return Err(CommandError::Message("migrate accepts no args".into()));
        }

        // Get config
        let db_url = config_value(&ctx, "DATABASE_URL")
            .ok_or_else(|| CommandError::Message("DATABASE_URL not set".into()))?;

        // Use service
        let run = ctx.migrations.apply(&ctx.config, ctx.options.dry_run).await?;

        // Format output
        let message = match ctx.format {
            ResponseFormat::Human => format!("Applied {} migrations", run.applied.len()),
            ResponseFormat::Json => "executed migrate".to_string(),
        };

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(json!({
                "run": run,
                "dry_run": ctx.options.dry_run,
            })),
            error: None,
        })
    }
}
```

---

## 10. Summary Table

| Aspect | Component | Location |
|--------|-----------|----------|
| **Entry Point** | CLI main() | `/crates/foundry-cli/src/main.rs` |
| **Kernel** | FoundryApp::dispatch() | `/crates/foundry-application/src/lib.rs` |
| **Registry** | CommandRegistry | `/crates/foundry-application/src/registry.rs` |
| **Command Trait** | FoundryCommand | `/crates/foundry-plugins/src/lib.rs` |
| **Command Metadata** | CommandDescriptor | `/crates/foundry-domain/src/lib.rs` |
| **Context** | CommandContext | `/crates/foundry-plugins/src/lib.rs` |
| **Result** | CommandResult | `/crates/foundry-plugins/src/lib.rs` |
| **Errors** | CommandError, AppError | `/crates/foundry-plugins/src/error.rs` |
| **Output Rendering** | render_result() | `/crates/foundry-cli/src/main.rs` |
| **HTTP Gateway** | HttpServer, invoke() | `/crates/foundry-api/src/http.rs` |
| **Programmatic API** | FoundryInvoker | `/crates/foundry-api/src/invocation.rs` |
| **Service Container** | Container | Re-exported from `foundry_service_container` |
| **Bootstrap** | BootstrapCommands::register_all() | `/crates/foundry-application/src/commands/mod.rs` |

---

## Implementation Ready for Programmatic Execution

The architecture is designed for programmatic execution:

1. **Use FoundryInvoker** - Wraps FoundryApp for async command invocation
2. **Use InvocationRequest** - Builder pattern for flexible request construction
3. **Parse InvocationResponse** - Strongly typed CommandResult with error handling
4. **HTTP/REST** - POST /invoke endpoint for external systems
5. **Access Metadata** - Get command catalog via FoundryInvoker::descriptors()
6. **Custom Ports** - Implement service traits for custom integrations

