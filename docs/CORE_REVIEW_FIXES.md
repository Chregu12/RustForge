# Core review fixes and migration

This changes the development snapshot of 1.0.0-rc.3. It does not publish a release
or retroactively modify existing tags.

| Review finding | Change | Regression evidence |
|---|---|---|
| Blocking async query methods | Shared async executor; legacy drivers offloaded; scoped SeaORM pool executes natively | Bridge liveness and facade/typed-pool tests |
| Typed ORM silently omitted rows | Decode errors propagate from all_typed, paginate and eager get | Incompatible SQLite row test |
| Boolean DSL lacked a type rule | BooleanRule wired into validate! | true/false pass; strings, integers, null and missing values reject |
| ValidatedData included unchecked fields | Filter by rule keys; add into_typed DTO conversion | Unexpected admin field excluded |
| Global routes and separate database paths | Instance-owned App and DatabaseManager::scope share one pool | Concurrent app isolation, transaction/savepoint tests |
| Wiki/README drift | Example/dependencies/status generated from one compiled source and pinned implementation | sync-getting-started.py --check and blog-slice HTTP tests |
| MSRV gate used the wrong compiler | Declare supported Rust 1.96.0 and invoke cargo +1.96.0 explicitly on rf/rustforge | msrv CI job |

## Behavior changes

- Wrong database row types now fail the query. Fix incompatible schema/model
  definitions instead of relying on silent row omission.
- `validate! { enabled: bool }` accepts JSON booleans; clients sending string or
  integer booleans must update their payloads. `.optional` permits absence.
- A successful `ValidatedData` contains only fields with rules. Fields needed
  for persistence must be listed explicitly. `ValidatedData::new` remains a
  low-level constructor; application code should consume validator results.
- For HTTP I/O use the async facade methods. Sync facade methods and older
  helper macros remain blocking compatibility APIs.
- In an app-owned database scope, sync DB calls return a diagnostic error rather
  than accessing the legacy global pool. Use `_async` variants. Synchronous
  derive-based database checks also require an explicit strategy; use the
  asynchronous validate! database rules inside these scopes.
- Use `DB::transaction(async { ... })` within a database scope. Do not split a
  synchronous begin/commit session across await points or spawned tasks.
- New applications should build routes through `App`. Global route registration
  is still available for compatibility, but is not an app-isolation mechanism.
- The supported MSRV is 1.96.0. The previous 1.79 declaration was not proven and
  is withdrawn. Existing RC tags keep their original metadata.

## Scope of the shared executor

`App::database` and `DatabaseManager::scope` cover async model/query/validation
operations. Explicit SeaORM services share that pool through the manager.
The legacy synchronous database implementation remains a compatibility adapter;
it is not used for app-scoped async requests. Cache/mail/storage async methods
reuse their existing configured managers and do not create alternative stores.

The canonical getting-started program uses temporary SQLite storage, creates its
schema, and checks 201/400/404/422 responses. Production deployments still need
persistent storage and versioned migrations. PostgreSQL uses backend-specific
DDL and an auto-generated primary key named `id` for the facade's insert API.
