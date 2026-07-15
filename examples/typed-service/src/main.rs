//! RustForge typed-service example — cycle 15 / item 1
//!
//! Demonstrates the FULLY-TYPED, DI-visible path recommended for larger /
//! testable applications.  Zero global facades are used anywhere in this file.
//!
//! # Pattern summary
//!
//! ```text
//! AppState { post_service: Arc<dyn PostService> }
//!            ↑ injected at startup, swappable for mocks in tests
//!
//! handler(State(state): State<AppState>, ValidatedJson(input): ValidatedJson<CreatePost>)
//!         ↑ all dependencies are explicit in the function signature
//!         ↑ no DB::, Cache::, Mail:: static calls
//!
//! state.post_service.create(input).await?
//!         ↑ business logic called through the trait; concrete impl is hidden
//! ```
//!
//! # ORM status — instance-based path is REAL
//!
//! `rf_orm::DatabaseManager` IS instance-based:
//!   - `DatabaseManager::connect(config).await?` → owned struct holding a SeaORM pool
//!   - `db.connection() -> &DatabaseConnection` exposes the pool for queries
//!   - wrap in `Arc<DatabaseManager>` and inject into AppState / services
//!
//! The `DB` / `GLOBAL_DB` constants in `rf_orm::facade` are a SEPARATE convenience
//! layer backed by a `Mutex<DBManager>` (rusqlite).  They are NOT required by
//! `DatabaseManager`, which uses `sea_orm::DatabaseConnection` (sqlx pool).
//!
//! # Run
//!
//! ```
//! cargo run -p typed-service
//! # POST /posts → create; GET /posts → list
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rf_orm::{ConnectionTrait, DatabaseConfig, DatabaseManager};
use rf_validation::ValidatedJson;
use sea_orm::{DbBackend, Statement, Value};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ── Domain types ───────────────────────────────────────────────────────────────

/// A blog post returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
}

/// Input for creating a post.  `Validate` derive enforces field constraints
/// inside `ValidatedJson`; invalid requests are rejected with 422 before the
/// handler body runs.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePost {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(min = 1, max = 10_000))]
    pub body: String,
}

// ── AppError ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(String),
    #[error("not found")]
    NotFound,
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(serde_json::json!({ "error": self.to_string() }))).into_response()
    }
}

// ── PostService trait — the explicit DI boundary ───────────────────────────────
//
// Handlers depend on this trait, NOT on a concrete DB struct.  Swapping the
// implementation (real DB vs mock) requires no change to any handler.

#[async_trait]
pub trait PostService: Send + Sync {
    async fn create(&self, input: CreatePost) -> Result<Post, AppError>;
    async fn list(&self) -> Result<Vec<Post>, AppError>;
}

// ── Concrete implementation: DbPostService ────────────────────────────────────
//
// Uses `Arc<DatabaseManager>` (instance-based; no global state).
// In a real application migrations would manage the schema; here we call
// CREATE TABLE IF NOT EXISTS once per operation to keep the example
// self-contained.

pub struct DbPostService {
    db: Arc<DatabaseManager>,
}

impl DbPostService {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }

    async fn ensure_schema(&self) -> Result<(), AppError> {
        let conn = self.db.connection();
        conn.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE IF NOT EXISTS posts \
             (id INTEGER PRIMARY KEY AUTOINCREMENT, \
              title TEXT NOT NULL, \
              body  TEXT NOT NULL)",
        ))
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl PostService for DbPostService {
    async fn create(&self, input: CreatePost) -> Result<Post, AppError> {
        self.ensure_schema().await?;

        let conn = self.db.connection();
        let result = conn
            .execute(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO posts (title, body) VALUES (?, ?)",
                [
                    Value::String(Some(Box::new(input.title.clone()))),
                    Value::String(Some(Box::new(input.body.clone()))),
                ],
            ))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Post {
            id: result.last_insert_id() as i64,
            title: input.title,
            body: input.body,
        })
    }

    async fn list(&self) -> Result<Vec<Post>, AppError> {
        self.ensure_schema().await?;

        let conn = self.db.connection();
        let rows = conn
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT id, title, body FROM posts ORDER BY id",
            ))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let posts = rows
            .into_iter()
            .map(|row| Post {
                id: row.try_get::<i64>("", "id").unwrap_or(0),
                title: row.try_get::<String>("", "title").unwrap_or_default(),
                body: row.try_get::<String>("", "body").unwrap_or_default(),
            })
            .collect();

        Ok(posts)
    }
}

// ── AppState — the DI container ───────────────────────────────────────────────
//
// Clone is derived so axum can hand a copy to every request handler.
// The inner Arc<dyn PostService> is cheap to clone (reference-counted pointer).

#[derive(Clone)]
pub struct AppState {
    /// Injected at startup; swapped for a mock in tests.
    pub post_service: Arc<dyn PostService>,
}

// ── Handlers — EXPLICIT signatures, no global calls ───────────────────────────
//
// Every dependency is visible in the function signature:
//   State(state)                — pulls AppState from the request extensions
//   ValidatedJson(input)        — deserializes + validates; 422 on bad input
//
// The handler body calls `state.post_service.create(input).await?` — the
// dependency is injected, fully mockable, and auditable from the signature.

pub async fn create_post(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<CreatePost>,
) -> Result<Json<Post>, AppError> {
    let post = state.post_service.create(input).await?;
    Ok(Json(post))
}

pub async fn list_posts(State(state): State<AppState>) -> Result<Json<Vec<Post>>, AppError> {
    let posts = state.post_service.list().await?;
    Ok(Json(posts))
}

// ── Router factory — accepts state so tests can inject mocks ──────────────────

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/posts", get(list_posts).post(create_post))
        .with_state(state)
}

// ── Main — wires real dependencies ────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // DatabaseManager is INSTANCE-BASED: connect() returns an owned struct.
    // No global DB singleton is touched here.
    let db = Arc::new(DatabaseManager::connect(DatabaseConfig::default()).await?);

    let post_service: Arc<dyn PostService> = Arc::new(DbPostService::new(db));
    let state = AppState { post_service };

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3010").await?;
    println!("typed-service listening on http://127.0.0.1:3010");
    axum::serve(listener, app).await?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// Three independent test cases demonstrate the testability benefit of the typed
// DI path:
//
// 1. Unit test with MockPostService — NO database, NO server, NO global state.
//    The mock is a plain struct implementing the trait.  This is possible ONLY
//    because the handler depends on the trait, not on a concrete DB type.
//
// 2. AppState wiring test — verifies that injecting a mock into AppState and
//    calling through `state.post_service` works exactly as a handler would.
//
// 3. Integration test with DbPostService and an in-memory SQLite pool —
//    proves that `DatabaseManager` IS instance-based (no `GLOBAL_DB` involved).

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::time::Duration;

    // ── Mock implementation ───────────────────────────────────────────────────
    //
    // A zero-dependency fake: no I/O, no async runtime beyond the executor
    // tokio provides for the test.

    struct MockPostService {
        next_id: AtomicI64,
    }

    impl MockPostService {
        fn new() -> Self {
            Self {
                next_id: AtomicI64::new(1),
            }
        }
    }

    #[async_trait]
    impl PostService for MockPostService {
        async fn create(&self, input: CreatePost) -> Result<Post, AppError> {
            let id = self.next_id.fetch_add(1, Ordering::SeqCst);
            Ok(Post {
                id,
                title: input.title,
                body: input.body,
            })
        }

        async fn list(&self) -> Result<Vec<Post>, AppError> {
            Ok(vec![])
        }
    }

    // ── Test 1: pure unit test, mock service, no DB, no server ───────────────

    #[tokio::test]
    async fn test_mock_service_create_assigns_sequential_ids() {
        let svc = MockPostService::new();

        let p1 = svc
            .create(CreatePost {
                title: "Hello DI".into(),
                body: "no global state was harmed in the making of this test".into(),
            })
            .await
            .unwrap();
        assert_eq!(p1.id, 1, "first post gets id 1");
        assert_eq!(p1.title, "Hello DI");

        let p2 = svc
            .create(CreatePost {
                title: "Second".into(),
                body: "body".into(),
            })
            .await
            .unwrap();
        assert_eq!(p2.id, 2, "second post gets id 2");
    }

    // ── Test 2: AppState wiring — Arc<dyn PostService> is injectable ──────────

    #[tokio::test]
    async fn test_appstate_dependency_injection() {
        // Inject mock through AppState — exactly what build_router() does at
        // startup, except with a mock implementation instead of DbPostService.
        let state = AppState {
            post_service: Arc::new(MockPostService::new()),
        };

        // Call through state exactly as a handler would:
        //   `state.post_service.create(input).await?`
        let input = CreatePost {
            title: "DI test".into(),
            body: "injected, not global".into(),
        };
        let post = state.post_service.create(input).await.unwrap();

        assert_eq!(post.id, 1);
        assert_eq!(post.title, "DI test");
        assert_eq!(post.body, "injected, not global");
    }

    // ── Test 3: DbPostService uses DatabaseManager — instance-based ORM ───────
    //
    // Proves the ORM discovery claim: DatabaseManager IS instance-based.
    // We create a private pool (max_connections=1 for shared in-memory SQLite),
    // wrap it in Arc, pass it to DbPostService — zero interaction with GLOBAL_DB.

    fn single_connection_config() -> DatabaseConfig {
        DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
            min_connections: 1,
            connect_timeout: Duration::from_secs(8),
            idle_timeout: None,
            acquire_timeout: Duration::from_secs(30),
            log_queries: false,
            log_level: "off".to_string(),
        }
    }

    #[tokio::test]
    async fn test_db_post_service_is_instance_based() {
        // Create a private DatabaseManager — no global state, no DB:: calls.
        let db = Arc::new(
            DatabaseManager::connect(single_connection_config())
                .await
                .expect("sqlite::memory: must connect"),
        );

        let svc = DbPostService::new(Arc::clone(&db));

        // Create
        let input = CreatePost {
            title: "Instance-based ORM".into(),
            body: "DatabaseManager::connect gives an owned pool, no GLOBAL_DB needed".into(),
        };
        let created = svc.create(input.clone()).await.unwrap();
        assert!(created.id > 0, "id from AUTOINCREMENT must be positive, got {}", created.id);
        assert_eq!(created.title, input.title);
        assert_eq!(created.body, input.body);

        // List — same pool, same in-memory SQLite database
        let posts = svc.list().await.unwrap();
        assert_eq!(posts.len(), 1, "expected 1 post, got {}", posts.len());
        assert_eq!(posts[0].title, "Instance-based ORM");
    }
}
