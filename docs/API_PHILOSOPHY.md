# RustForge API philosophy

RustForge supports concise Laravel-style handlers and explicit Rust services.
Both are first-class and can use the same application-owned database pool.

## One app, one pool, two interfaces

```rust,ignore
let db = DatabaseManager::connect(config).await?;
let service_db = db.clone(); // same SeaORM connection pool
let app = App::new().database(db)
    .post("/posts", store)
    .build();
```

`App` owns a fresh route registry, installs `capture_request`, and exposes the
pool as `Extension<DatabaseManager>`. It also scopes async model/DB calls to that
pool. Explicit services may use `db.connection()` with SeaORM or
`db.scope(async { /* Model!/DB calls */ }).await` with the same facade executor.
Two apps can register the same paths and use different databases concurrently.
No reset of a process-global registry or database is needed.

## Concise handlers

```rust,ignore
async fn store() -> AppResult<impl IntoResponse> {
    let data = validate! { title: string.max(200), body: string }?;
    let post = Post::create(data).await?;
    Ok(json(post).status(StatusCode::CREATED))
}
```

`validate!` returns a filtered `ValidatedData` map. Only fields with rules are
returned. It is a runtime validator, not a generated Rust DTO. For a concrete
input type use `ValidatedJson<CreatePost>` or `data.into_typed::<CreatePost>()`.
Boolean rules accept JSON `true`/`false`; strings and numeric substitutes fail.

Typed ORM reads return mapping errors. They do not omit incompatible rows from
`all_typed`, pagination, or eager reads. Untyped `all()` remains `Vec<Value>` for
compatibility; choose the typed method when the domain requires a model struct.

## Async execution is the HTTP default

Models, the async query builder and database validation rules use the shared
async executor. A scoped pool uses native SeaORM I/O. Legacy global rusqlite/
SQLx configuration is supported through a blocking worker adapter. The adapter
must not be mixed with synchronous thread-local transactions.

HTTP facade methods include:

```rust,ignore
let cached: Option<String> = Cache::get_async("key").await?;
Cache::put_async("key", "value", 60).await?;
Mail::raw_async("user@example.com", "Ready", "Your report is ready").await?;
Storage::put_async("reports/report.txt", bytes).await?;
```

These methods share their corresponding facade's configured service. File and
legacy sync transport work runs in `spawn_blocking`; native async drivers remain
available for injected services. `AsyncBridge::run` provides non-blocking waits
for drivers tied to a bridge runtime.

**Compatibility:** synchronous `DB::select`, `Cache::get`, `Mail::send`,
`Storage::put`, older helper macros and `AsyncBridge::block_on` remain blocking
APIs for existing code. They must not be called directly on Tokio HTTP workers.
Synchronous DB calls inside an app-owned async database scope return an error,
preventing accidental queries against a different global database.

## Transactions and task scopes

```rust,ignore
db.scope(DB::transaction(async {
    create!(Post, title = "Hello", body = "World")?;
    Ok(())
})).await?;
```

A scoped async transaction commits on success and rolls back on error. Nested
transactions use savepoints. Cancellation drops the transaction and queues a
rollback through the database driver. All operations in the transaction must be
awaited within its scope. Direct SeaORM queries through `db.connection()` use
the pool, not the facade transaction; use the facade's scoped operations inside
`DB::transaction`, or use an explicit SeaORM transaction consistently.

Request, auth and database scopes are task-local. `tokio::spawn` does not
implicitly inherit them. Background work should receive owned input and an
explicit cloned service/database handle, then establish its own scope.

## Compatibility routing

Free functions such as `get("/posts", index)` retain their global registry for
older applications. Prefer `App::new().get(...).build()` in new applications and
tests. The lower-level Axum router, `State<AppState>`, traits and injected
`Arc<dyn Service>` remain available without any facade dependency.
