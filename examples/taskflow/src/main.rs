//! TaskFlow — a small, canonical project/task manager built ENTIRELY on the
//! RustForge high-level primitives, and the PERMANENT regression test for six
//! framework edges that a real-app build once exposed and that are now FIXED.
//! Each edge below is genuinely exercised, so a regression re-breaks this
//! example's `cargo test -p taskflow`.
//!
//! The FIXED edges this example guards (see the tests at the bottom):
//!
//!  1. BIDIRECTIONAL relations that now COMPILE. `Project hasMany tasks: Task`
//!     AND `Task belongsTo project: Project` are both declared — the two models
//!     reference each other. This used to be an E0391 opaque-type inference
//!     cycle ("cycle detected when computing type of opaque
//!     `with_relations::{opaque#0}`"); the generated `with_relations` now returns
//!     a CONCRETE boxed future, so the cycle no longer forms. Its mere
//!     compilation is a regression guard, and both directions hydrate:
//!     `Project::with(&["tasks"])` → `project.tasks`, and
//!     `Task::with(&["project"])` → `task.project`. (`User hasMany tasks` +
//!     `Task belongsTo assignee` closes a SECOND bidirectional cycle for good
//!     measure.)
//!
//!  2. A relation using a foreign_key OVERRIDE. The tasks table stores the
//!     assignee in the Laravel-conventional `user_id` column, but the relation
//!     is named `assignee`, whose default FK would be `assignee_id`. The
//!     `(foreign_key = "user_id")` override is therefore load-bearing: drop it
//!     and `task.assignee` stops hydrating. `Task::with(&["assignee"])` →
//!     `task.assignee`.
//!
//!  3. A SINGLE `build_router` where `capture_request` globals (argument-less
//!     handlers reading `input()` / path params) AND `ValidatedJson<CreateX>`
//!     body handlers COEXIST — no two-router split. `capture_request` now
//!     buffers the body to parse the globals and RE-INSERTS it downstream, so a
//!     `ValidatedJson` extractor on the same router still sees the body. Both
//!     handler styles live on one router here.
//!
//!  4. `validate!` AND `ValidatedJson` producing a real structured 422 with a
//!     per-field `errors` map, including a custom `@ message(...)` override on
//!     the `User::email` field.
//!
//!  5. Nested eager `with(&["tasks.assignee"])`, a constrained
//!     `with_where("tasks", "status", "open")`, and the two COMBINED — where the
//!     constraint applies to the FIRST path segment (`tasks`), so a nested
//!     `tasks.assignee` load returns only OPEN tasks, each with its assignee.
//!
//!  6. Auth-protected mutations using the VERIFYING login-by-id path
//!     (`Auth::login_using_id_verified` against a real DB-backed `UserProvider`),
//!     NOT the trusting `login_using_id`. A bearer for a non-existent user id is
//!     rejected with 401.
//!
//!  7. A query scope (`Task::open()`), typed `paginate()`, a `where_like`
//!     search, and `create!`/`find!`/`update!`/`delete!` against the real
//!     rusqlite DB, with `json(..)` responses carrying 201/200/204/404/422/401.
//!
//! Run:  cargo run -p taskflow   (serves on http://127.0.0.1:3007)

use axum::http::StatusCode;
use axum::response::IntoResponse;
use rf::prelude::*;
use rf_validation::ValidatedJson;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Models — bidirectional relations + a foreign_key override, all declared with
// the real `Model!` DSL. That these COMPILE at all is regression guard #1.
// ---------------------------------------------------------------------------

// A user who can be assigned tasks. `hasMany tasks: Task` is the inverse of
// `Task belongsTo assignee` (its default FK `user_id` matches the tasks column),
// closing a User <-> Task cycle that the boxed-future fix now compiles.
Model!(User {
    validated,
    name: String @ min(1) max(80),
    email: String @ email message("A valid email address is required"),

    hasMany tasks: Task,

    timestamps = false,
});

// A project owns many tasks. `hasMany tasks: Task` (default FK `project_id`,
// matching the column) is the inverse of `Task belongsTo project` — the
// canonical Laravel bidirectional pair (regression guard #1).
Model!(Project {
    validated,
    name: String @ min(1) max(120),
    description: String,

    hasMany tasks: Task,

    timestamps = false,
});

// A task belongs to a project AND to an assignee.
//
// * `belongsTo project: Project` closes the Project <-> Task cycle (edge #1).
// * `belongsTo assignee: User (foreign_key = "user_id")` is the foreign_key
//   OVERRIDE (edge #2): the relation is named `assignee` (default FK would be
//   `assignee_id`) but the real column is `user_id`, so the override is what
//   makes `task.assignee` resolve.
// * `scope open` generates `Task::open()` returning a real QueryBuilder already
//   filtered to open tasks (edge #7).
Model!(Task {
    validated,
    title: String @ min(1) max(120),
    status: String,
    project_id: i64,
    user_id: i64,

    belongsTo project: Project,
    belongsTo assignee: User (foreign_key = "user_id"),

    scope open: where("status", "open"),

    timestamps = false,
});

// ---------------------------------------------------------------------------
// Migrations (real SQLite DDL on the global manager). No mirror columns, no
// generated-column hacks — the FK override makes the natural schema resolvable.
// ---------------------------------------------------------------------------

fn migrate() {
    DB::statement("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
        .expect("create users");
    DB::statement(
        "CREATE TABLE IF NOT EXISTS projects (id INTEGER PRIMARY KEY, name TEXT, description TEXT)",
    )
    .expect("create projects");
    DB::statement(
        "CREATE TABLE IF NOT EXISTS tasks (\
             id INTEGER PRIMARY KEY, title TEXT, status TEXT, \
             project_id INTEGER, user_id INTEGER)",
    )
    .expect("create tasks");
}

// ---------------------------------------------------------------------------
// Auth: a real DB-backed UserProvider + the VERIFYING login-by-id path (edge #6).
// ---------------------------------------------------------------------------

/// Resolves user identities against the real `users` table. `retrieve_by_id` is
/// what the VERIFYING `Auth::login_using_id_verified` calls to confirm a bearer's
/// id maps to a stored user before establishing any identity — so a phantom id
/// is never authorized. Uses the synchronous `DB::select` facade (a real
/// rusqlite query), which is safe to call from this sync trait method.
struct DbUserProvider;

impl rf_auth::UserProvider for DbUserProvider {
    fn retrieve_by_id(&self, id: u64) -> Option<Value> {
        DB::select("SELECT * FROM users WHERE id = ?", &[Value::from(id)])
            .ok()
            .and_then(|mut rows| rows.pop())
    }

    fn retrieve_by_credentials(&self, credentials: &Value) -> Option<Value> {
        // Only the id lookup is used by this example; support it for completeness.
        let id = credentials.get("id")?.as_i64()?;
        self.retrieve_by_id(id as u64)
    }
}

/// Per-request auth middleware: opens a fresh isolated auth scope, then bridges
/// an `Authorization: Bearer <user_id>` header into a VERIFYING login. Because
/// the login is verified against the DB-backed `UserProvider`, a bearer for a
/// non-existent user leaves the request a guest (rejected downstream with 401).
async fn auth_scope_login(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    rf_auth::auth_manager::with_auth_scope(async move {
        if let Some(id) = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .and_then(|t| t.trim().parse::<u64>().ok())
        {
            // VERIFYING path: only logs in if `id` resolves to a real user.
            let _ = Auth::login_using_id_verified(id, false);
        }
        next.run(req).await
    })
    .await
}

/// Guard helper: `Some(401 response)` for a guest, `None` for an authenticated
/// caller. `Auth::check()` reads the per-request scope set by `auth_scope_login`.
fn require_auth() -> Option<axum::response::Response> {
    if Auth::check() {
        None
    } else {
        Some(
            json(json!({ "error": "unauthenticated" }))
                .status(StatusCode::UNAUTHORIZED)
                .into_response(),
        )
    }
}

/// Read the `{id}` path param (merged into the request context by the router's
/// path-param layer), or a 400 response when it is missing/non-numeric.
///
/// (Fully-qualified `std::result::Result` because `rf::prelude` re-exports its
/// own single-parameter `Result<T, RustForgeError>` alias, which would otherwise
/// shadow the standard two-parameter `Result` used here.)
fn path_id() -> std::result::Result<i64, axum::response::Response> {
    input::<i64>("id").ok_or_else(|| {
        json(json!({ "error": "invalid id" }))
            .status(StatusCode::BAD_REQUEST)
            .into_response()
    })
}

// ---------------------------------------------------------------------------
// Users — a ValidatedJson<CreateUser> create endpoint. Exercises the custom
// `@ message` in a structured 422 (edge #4) and coexists on the ONE router with
// the input()-style handlers below (edge #3).
// ---------------------------------------------------------------------------

/// POST /users — auth-protected. The body is deserialized AND validated by the
/// `ValidatedJson` extractor before the handler runs; an invalid body is
/// rejected with a structured 422 (`errors` map) carrying `User::email`'s custom
/// `@ message`. On success (valid body): 201 with the persisted row, or 401 for
/// a guest.
async fn users_store(ValidatedJson(user): ValidatedJson<CreateUser>) -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    match create!(User, name = user.name, email = user.email) {
        Ok(created) => json(created).status(StatusCode::CREATED).into_response(),
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// GET /users/{id} — one user WITH the `hasMany tasks` inverse eager-loaded
/// (`user.tasks`), proving the User <-> Task bidirectional pair hydrates.
async fn users_show() -> impl IntoResponse {
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match User::with(&["tasks"]).r#where("id", id).get().await {
        Ok(mut rows) => match rows.pop() {
            Some(u) => json(u).status(StatusCode::OK).into_response(),
            None => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Projects — full REST, input()/validate! style (edge #3, #7), plus the eager /
// nested / constrained loads (edge #1, #5).
// ---------------------------------------------------------------------------

/// GET /projects — every project WITH its tasks eager-loaded (`hasMany`,
/// N+1-free). One direction of the bidirectional pair (edge #1).
async fn projects_index() -> impl IntoResponse {
    match Project::with(&["tasks"]).get().await {
        Ok(projects) => json(projects).status(StatusCode::OK),
        Err(e) => json(json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /projects/{id} — one project or 404.
async fn projects_show() -> impl IntoResponse {
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match find!(Project, id) {
        Ok(Some(p)) => json(p).status(StatusCode::OK).into_response(),
        Ok(None) => json(json!({ "error": "not found" }))
            .status(StatusCode::NOT_FOUND)
            .into_response(),
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// GET /projects/{id}/board — NESTED eager load: tasks + each task's assignee
/// (`with(&["tasks.assignee"])`). Returns ALL of the project's tasks, each with
/// its `assignee` hydrated (edge #5).
async fn projects_board() -> impl IntoResponse {
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match Project::with(&["tasks.assignee"]).r#where("id", id).get().await {
        Ok(mut rows) => match rows.pop() {
            Some(p) => json(p).status(StatusCode::OK).into_response(),
            None => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// GET /projects/{id}/open — CONSTRAINED eager load: only the project's OPEN
/// tasks, via `with(&["tasks"]).with_where("tasks", "status", "open")` (edge #5).
async fn projects_open() -> impl IntoResponse {
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match Project::with(&["tasks"])
        .r#where("id", id)
        .with_where("tasks", "status", "open")
        .get()
        .await
    {
        Ok(mut rows) => match rows.pop() {
            Some(p) => json(p).status(StatusCode::OK).into_response(),
            None => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// GET /projects/{id}/open-board — the NESTED and CONSTRAINED loads COMBINED:
/// `with(&["tasks.assignee"]).with_where("tasks", "status", "open")`. The
/// constraint applies to the FIRST path segment (`tasks`), so this returns only
/// the OPEN tasks, each still hydrated with its `assignee`. (Constraining the
/// deeper `assignee` segment is a documented framework follow-up.)
async fn projects_open_board() -> impl IntoResponse {
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match Project::with(&["tasks.assignee"])
        .r#where("id", id)
        .with_where("tasks", "status", "open")
        .get()
        .await
    {
        Ok(mut rows) => match rows.pop() {
            Some(p) => json(p).status(StatusCode::OK).into_response(),
            None => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// POST /projects — auth-protected, validated with the `validate!` DSL (an
/// input()-style handler, edge #3), 201 / 422 / 401.
async fn projects_store() -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    if validate! { name: string.max(120), description: string }.is_err() {
        return json(json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .into_response();
    }
    let name: String = input("name").unwrap_or_default();
    let description: String = input("description").unwrap_or_default();
    match create!(Project, name = name, description = description) {
        Ok(created) => json(created).status(StatusCode::CREATED).into_response(),
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// PUT /projects/{id} — auth-protected, validated, 200 / 404 / 422 / 401.
async fn projects_update() -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if validate! { name: string.max(120), description: string }.is_err() {
        return json(json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .into_response();
    }
    let name: String = input("name").unwrap_or_default();
    let description: String = input("description").unwrap_or_default();
    match update!(Project, id, name = name, description = description) {
        Ok(0) => json(json!({ "error": "not found" }))
            .status(StatusCode::NOT_FOUND)
            .into_response(),
        Ok(_) => match find!(Project, id) {
            Ok(Some(p)) => json(p).status(StatusCode::OK).into_response(),
            _ => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// DELETE /projects/{id} — auth-protected, 204 / 404 / 401.
async fn projects_destroy() -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match delete!(Project, id) {
        Ok(0) => json(json!({ "error": "not found" }))
            .status(StatusCode::NOT_FOUND)
            .into_response(),
        Ok(_) => Response::no_content().into_response(),
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Tasks — a ValidatedJson<CreateTask> create (edge #3 body handler) + input()
// reads, paginate(), the `open` scope, where_like search, and the two-way
// relation show (edge #1 + #2 + #7).
// ---------------------------------------------------------------------------

/// GET /tasks — typed paginated list (`Task::paginate`, edge #7). `?page=N`.
async fn tasks_index() -> impl IntoResponse {
    let page: usize = input::<String>("page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    const PER_PAGE: usize = 3;
    match Task::paginate(PER_PAGE, page).await {
        Ok(p) => json(json!({
            "data": p.data,
            "total": p.total,
            "per_page": p.per_page,
            "current_page": p.current_page,
            "last_page": p.last_page,
        }))
        .status(StatusCode::OK),
        Err(e) => json(json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /tasks/open — the generated `Task::open()` query scope (edge #7).
async fn tasks_open() -> impl IntoResponse {
    match Task::open().order_by("id", "asc").get().await {
        Ok(rows) => json(rows).status(StatusCode::OK),
        Err(e) => json(json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /tasks/search?q=.. — a real `where_like` title search (edge #7).
async fn tasks_search() -> impl IntoResponse {
    let q: String = input("q").unwrap_or_default();
    match DB::table("tasks")
        .where_like("title", format!("%{}%", q))
        .order_by("id", "asc")
        .get()
        .await
    {
        Ok(rows) => json(rows).status(StatusCode::OK),
        Err(e) => json(json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /tasks/{id} — show one task with BOTH relations eager-loaded:
/// `task.project` (bidirectional inverse, edge #1) and `task.assignee` (via the
/// foreign_key override, edge #2). 200 / 404.
async fn tasks_show() -> impl IntoResponse {
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match Task::with(&["project", "assignee"]).r#where("id", id).get().await {
        Ok(mut rows) => match rows.pop() {
            Some(t) => json(t).status(StatusCode::OK).into_response(),
            None => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// POST /tasks — auth-protected. Body deserialized AND validated by the
/// `ValidatedJson` extractor on the SAME router as the input()-style handlers
/// (edge #3). 201 / 422 / 401.
async fn tasks_store(ValidatedJson(task): ValidatedJson<CreateTask>) -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    match create!(
        Task,
        title = task.title,
        status = task.status,
        project_id = task.project_id,
        user_id = task.user_id
    ) {
        Ok(created) => json(created).status(StatusCode::CREATED).into_response(),
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// PUT /tasks/{id} — auth-protected, validated with `validate!`, 200 / 404 / 422 / 401.
async fn tasks_update() -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if validate! {
        title: string.max(120),
        status: string,
        project_id: int.min(1),
        user_id: int.min(1)
    }
    .is_err()
    {
        return json(json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .into_response();
    }
    let title: String = input("title").unwrap_or_default();
    let status: String = input("status").unwrap_or_default();
    let project_id: i64 = input("project_id").unwrap_or_default();
    let user_id: i64 = input("user_id").unwrap_or_default();
    match update!(Task, id, title = title, status = status, project_id = project_id, user_id = user_id)
    {
        Ok(0) => json(json!({ "error": "not found" }))
            .status(StatusCode::NOT_FOUND)
            .into_response(),
        Ok(_) => match find!(Task, id) {
            Ok(Some(t)) => json(t).status(StatusCode::OK).into_response(),
            _ => json(json!({ "error": "not found" }))
                .status(StatusCode::NOT_FOUND)
                .into_response(),
        },
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

/// DELETE /tasks/{id} — auth-protected, 204 / 404 / 401.
async fn tasks_destroy() -> impl IntoResponse {
    if let Some(resp) = require_auth() {
        return resp;
    }
    let id = match path_id() {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match delete!(Task, id) {
        Ok(0) => json(json!({ "error": "not found" }))
            .status(StatusCode::NOT_FOUND)
            .into_response(),
        Ok(_) => Response::no_content().into_response(),
        Err(e) => json(json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Router — ONE build_router hosting BOTH the input()/validate! handlers AND the
// ValidatedJson body handlers, behind `capture_request` (which now re-inserts
// the body so both can read it) and `auth_scope_login` (edge #3 + #6).
// ---------------------------------------------------------------------------

fn build_app() -> axum::Router {
    // Register the verifying auth provider (idempotent; replaces on re-build so
    // repeated build_app() calls across tests are safe).
    Auth::set_provider(std::sync::Arc::new(DbUserProvider));

    rf::global_router().clear();

    // Users (ValidatedJson create + eager-inverse show).
    post("/users", users_store);
    get("/users/{id}", users_show);

    // Projects (input()/validate! REST + eager/nested/constrained loads).
    get("/projects", projects_index);
    post("/projects", projects_store);
    get("/projects/{id}", projects_show);
    get("/projects/{id}/board", projects_board);
    get("/projects/{id}/open", projects_open);
    get("/projects/{id}/open-board", projects_open_board);
    put("/projects/{id}", projects_update);
    delete("/projects/{id}", projects_destroy);

    // Tasks (ValidatedJson create + input() reads; specific routes before {id}).
    get("/tasks", tasks_index);
    post("/tasks", tasks_store);
    get("/tasks/open", tasks_open);
    get("/tasks/search", tasks_search);
    get("/tasks/{id}", tasks_show);
    put("/tasks/{id}", tasks_update);
    delete("/tasks/{id}", tasks_destroy);

    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(auth_scope_login))
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    migrate();
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3007")
        .await
        .expect("bind");
    println!("taskflow listening on http://127.0.0.1:3007");
    axum::serve(listener, app).await.expect("serve");
}

// ---------------------------------------------------------------------------
// Integration test: the FULL lifecycle end-to-end (tower oneshot), explicitly
// asserting every fixed edge.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn call(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, value)
    }

    fn get_req(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    fn json_req(method: &str, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn auth_json(method: &str, uri: &str, body: &str, uid: u64) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header("Authorization", format!("Bearer {uid}"))
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    /// The generated Create DTOs really implement `rf_validation::Validate`; if
    /// they did not, `ValidatedJson<CreateX>` would not even compile.
    #[test]
    fn create_dtos_are_validate() {
        fn assert_is_validate<T: rf_validation::Validate>() {}
        assert_is_validate::<CreateUser>();
        assert_is_validate::<CreateTask>();
    }

    #[tokio::test]
    async fn taskflow_full_lifecycle_asserts_every_fixed_edge() {
        migrate();
        let app = build_app();

        // Seed two real users directly (assignees). Their ids back the verifying
        // bearer login.
        let ada = create!(User, name = "Ada", email = "ada@example.com").expect("user ada");
        let lin = create!(User, name = "Linus", email = "linus@example.com").expect("user linus");
        let ada_id = ada["id"].as_i64().unwrap();
        let lin_id = lin["id"].as_i64().unwrap();

        // --- Edge #6: a bearer for a NON-EXISTENT user is rejected (verifying
        //     login-by-id). A guest is likewise rejected. Both -> 401.
        let (status, body) = call(
            &app,
            json_req("POST", "/projects", r#"{"name":"P","description":"d"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "guest blocked");
        assert_eq!(body["error"], "unauthenticated");

        let (status, _) = call(
            &app,
            auth_json("POST", "/projects", r#"{"name":"Ghost","description":"z"}"#, 999_999),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "bearer for a non-existent user must be rejected (verifying login)"
        );

        // --- Edge #7: authenticated create -> 201 (create! against real DB).
        let (status, proj) = call(
            &app,
            auth_json("POST", "/projects", r#"{"name":"Apollo","description":"moon"}"#, ada_id as u64),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "auth create 201");
        let project_id = proj["id"].as_i64().expect("project id");

        // Seed four tasks through the ValidatedJson body handler on the SAME
        // router as the input() handlers (edge #3).
        for (title, status_v, assignee) in [
            ("Design rocket", "open", ada_id),
            ("Build engine", "open", lin_id),
            ("Test fuel", "done", ada_id),
            ("Write manual", "open", lin_id),
        ] {
            let (st, _) = call(
                &app,
                auth_json(
                    "POST",
                    "/tasks",
                    &format!(
                        r#"{{"title":"{title}","status":"{status_v}","project_id":{project_id},"user_id":{assignee}}}"#
                    ),
                    ada_id as u64,
                ),
            )
            .await;
            assert_eq!(st, StatusCode::CREATED, "ValidatedJson task create 201");
        }

        // --- Edge #1 (direction A): Project::with(["tasks"]) -> project.tasks.
        let (status, list) = call(&app, get_req("/projects")).await;
        assert_eq!(status, StatusCode::OK);
        let p = list
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"].as_i64() == Some(project_id))
            .expect("project present");
        assert_eq!(
            p["tasks"].as_array().unwrap().len(),
            4,
            "bidirectional Project hasMany tasks hydrated"
        );

        // --- Edge #1 (direction B) + #2: Task::with(["project","assignee"]) ->
        //     task.project (bidirectional inverse) AND task.assignee (FK override).
        let (status, one) = call(&app, get_req("/tasks/1")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            one["project"]["name"], "Apollo",
            "bidirectional Task belongsTo project hydrated"
        );
        assert_eq!(
            one["assignee"]["name"], "Ada",
            "FK-override Task belongsTo assignee (foreign_key = user_id) hydrated"
        );

        // --- Edge #1 (second bidirectional pair): User::with(["tasks"]) ->
        //     user.tasks. Ada is the assignee of two tasks.
        let (status, ada_full) = call(&app, get_req(&format!("/users/{ada_id}"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            ada_full["tasks"].as_array().unwrap().len(),
            2,
            "User hasMany tasks (inverse of the FK-override belongsTo) hydrated"
        );

        // --- Edge #5: nested with(["tasks.assignee"]) -> all 4 tasks, each with
        //     its assignee.
        let (status, board) = call(&app, get_req(&format!("/projects/{project_id}/board"))).await;
        assert_eq!(status, StatusCode::OK);
        let all_tasks = board["tasks"].as_array().unwrap();
        assert_eq!(all_tasks.len(), 4, "nested load returns all tasks");
        for t in all_tasks {
            assert!(
                t["assignee"]["name"].is_string(),
                "nested tasks.assignee hydrated: {t}"
            );
        }

        // --- Edge #5: constrained with_where (open only), non-nested -> 3 tasks.
        let (status, openp) = call(&app, get_req(&format!("/projects/{project_id}/open"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            openp["tasks"].as_array().unwrap().len(),
            3,
            "with_where constrained the tasks to the 3 open ones"
        );

        // --- Edge #5: nested AND constrained COMBINED -> only the 3 open tasks,
        //     each still with its assignee (first-segment constraint).
        let (status, ob) = call(&app, get_req(&format!("/projects/{project_id}/open-board"))).await;
        assert_eq!(status, StatusCode::OK);
        let ob_tasks = ob["tasks"].as_array().unwrap();
        assert_eq!(ob_tasks.len(), 3, "combined nested+with_where: only open tasks");
        for t in ob_tasks {
            assert_eq!(t["status"], "open", "constraint applied to first segment");
            assert!(
                t["assignee"]["name"].is_string(),
                "assignee still hydrated in the combined load: {t}"
            );
        }

        // --- Edge #7: typed paginate().
        let (status, page1) = call(&app, get_req("/tasks?page=1")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page1["total"], 4);
        assert_eq!(page1["per_page"], 3);
        assert_eq!(page1["last_page"], 2);
        assert_eq!(page1["data"].as_array().unwrap().len(), 3);
        let (_, page2) = call(&app, get_req("/tasks?page=2")).await;
        assert_eq!(page2["data"].as_array().unwrap().len(), 1);

        // --- Edge #7: query scope Task::open() -> the 3 open tasks.
        let (status, open) = call(&app, get_req("/tasks/open")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(open.as_array().unwrap().len(), 3, "3 open tasks via scope");

        // --- Edge #7: where_like search.
        let (status, found) = call(&app, get_req("/tasks/search?q=engine")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(found.as_array().unwrap().len(), 1);
        assert_eq!(found[0]["title"], "Build engine");

        // --- Edge #7: auth-protected update -> 200 + persisted (update! + find!).
        let (status, upd) = call(
            &app,
            auth_json(
                "PUT",
                "/tasks/1",
                &format!(
                    r#"{{"title":"Design rocket v2","status":"done","project_id":{project_id},"user_id":{ada_id}}}"#
                ),
                ada_id as u64,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "auth update 200");
        assert_eq!(upd["title"], "Design rocket v2");

        // --- Edge #4: input()-style validate! rejects an oversized title -> 422.
        let long = "x".repeat(300);
        let (status, _) = call(
            &app,
            auth_json(
                "PUT",
                "/tasks/1",
                &format!(
                    r#"{{"title":"{long}","status":"open","project_id":{project_id},"user_id":{ada_id}}}"#
                ),
                ada_id as u64,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "validate! -> 422");

        // --- Edge #4: ValidatedJson produces a STRUCTURED 422 with a per-field
        //     `errors` map carrying User::email's CUSTOM @ message.
        let (status, body) = call(
            &app,
            auth_json("POST", "/users", r#"{"name":"Bob","email":"not-an-email"}"#, ada_id as u64),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "ValidatedJson -> 422");
        assert_eq!(
            body["errors"]["email"][0]["message"], "A valid email address is required",
            "422 body carries the custom @ message on the email field"
        );

        // --- Edge #7: 404 on a missing row; 401 for a guest delete; 204 for an
        //     authenticated delete; then 404 for the now-deleted row.
        let (status, _) = call(&app, get_req("/projects/999999")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "missing project -> 404");

        let (status, _) = call(
            &app,
            Request::builder()
                .method("DELETE")
                .uri("/tasks/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "guest delete -> 401");

        let (status, body) = call(&app, auth_json("DELETE", "/tasks/1", "", ada_id as u64)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "auth delete -> 204");
        assert_eq!(body, Value::Null, "204 carries no body");

        let (status, _) = call(&app, get_req("/tasks/1")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "deleted task -> 404");
    }
}
