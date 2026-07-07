//! RustForge canonical REST resource: the full five-verb CRUD lifecycle for a
//! single `Article` resource, written with ONLY the framework's high-level
//! primitives and NO plumbing leaking into the handlers.
//!
//! It ties together, end to end:
//!   * `get`/`post`/`put`/`delete` routing + `build_router` + the
//!     `capture_request` middleware,
//!   * argument-less `impl IntoResponse` handlers (no `Request` arg, no
//!     `Result<_, StatusCode>`, no visible `.await?`) that read the request via
//!     the implicit-request globals `input("field")` / `input::<i64>("id")`,
//!   * the `validate!` typed DSL for 422 rejection,
//!   * the `Model!` / `create!` / `find!` / `update!` / `delete!` ORM macros for
//!     real (SQLite-backed) persistence,
//!   * the `Article belongsTo Author` relation, eager-loaded in the index via
//!     the typed `Article::with(&["author"]).get()` builder (one fetch + one
//!     batched loader query, no N+1), and
//!   * `json(..)` responses carrying the correct REST status codes:
//!     201 on create, 200 on read/update, 204 on delete, 404 on a missing row,
//!     422 on validation failure.
//!
//! Run it:  `cargo run -p rest-crud-resource`  (serves on http://127.0.0.1:3001)
//!   POST   /articles        {"title":"Hello","body":"World","author_id":1}
//!   GET    /articles
//!   GET    /articles/:id
//!   PUT    /articles/:id     {"title":"Edited","body":"Changed","author_id":1}
//!   DELETE /articles/:id
use axum::http::StatusCode;
use rf::prelude::*;

// The parent resource (belongsTo target). Backed by the real `authors` table.
Model!(Author {
    name: String,
});

// The resource under test. `author_id` is the FK on the article row; the
// `belongsTo author` relation resolves it against `authors.id`.
Model!(Article {
    title: String,
    body: String,
    author_id: i64,

    belongsTo author: Author,
});

/// GET /articles — list every article as JSON, eager-loading each one's
/// `author` via the typed `with(..).get()` builder (one SELECT + one batched
/// loader query, no N+1). Returns a real typed `Vec<Article>` with the `author`
/// field populated, serialized with `json(..)`.
async fn index() -> impl axum::response::IntoResponse {
    match Article::with(&["author"]).get().await {
        Ok(articles) => json(articles),
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /articles/:id — show one article (200) or 404 if it does not exist. The
/// `:id` path param reaches this argument-less handler via `input::<i64>("id")`.
async fn show() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => {
            return json(serde_json::json!({ "error": "invalid id" }))
                .status(StatusCode::BAD_REQUEST)
        }
    };
    match find!(Article, id) {
        Ok(Some(article)) => json(article).status(StatusCode::OK),
        Ok(None) => json(serde_json::json!({ "error": "not found" })).status(StatusCode::NOT_FOUND),
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /articles — validate the request, persist a real row, return it (201).
/// A validation failure short-circuits to 422 before any DB write.
async fn store() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(200), body: string, author_id: int.min(1) }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();
    let author_id: i64 = input("author_id").unwrap_or_default();
    match create!(Article, title = title, body = body, author_id = author_id) {
        Ok(created) => json(created).status(StatusCode::CREATED),
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// PUT /articles/:id — validate, then update an existing row (200 with the
/// changed row), 404 if the id does not exist, 422 if validation fails.
async fn update() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => {
            return json(serde_json::json!({ "error": "invalid id" }))
                .status(StatusCode::BAD_REQUEST)
        }
    };
    if validate! { title: string.max(200), body: string, author_id: int.min(1) }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();
    let author_id: i64 = input("author_id").unwrap_or_default();
    match update!(Article, id, title = title, body = body, author_id = author_id) {
        // `update_by_id` reports the number of affected rows: 0 => no such id.
        Ok(0) => json(serde_json::json!({ "error": "not found" })).status(StatusCode::NOT_FOUND),
        Ok(_) => match find!(Article, id) {
            Ok(Some(article)) => json(article).status(StatusCode::OK),
            Ok(None) => {
                json(serde_json::json!({ "error": "not found" })).status(StatusCode::NOT_FOUND)
            }
            Err(e) => {
                json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /articles/:id — destroy the row (204 No Content) or 404 if missing.
async fn destroy() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => {
            return json(serde_json::json!({ "error": "invalid id" }))
                .status(StatusCode::BAD_REQUEST)
        }
    };
    match delete!(Article, id) {
        // `destroy` reports the number of deleted rows: 0 => no such id.
        Ok(0) => json(serde_json::json!({ "error": "not found" })).status(StatusCode::NOT_FOUND),
        Ok(_) => Response::no_content(),
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Ensure the backing tables exist (real SQLite DDL on the global manager).
fn migrate() {
    DB::statement("CREATE TABLE IF NOT EXISTS authors (id INTEGER PRIMARY KEY, name TEXT)")
        .expect("create authors table");
    DB::statement(
        "CREATE TABLE IF NOT EXISTS articles (\
             id INTEGER PRIMARY KEY, title TEXT, body TEXT, author_id INTEGER)",
    )
    .expect("create articles table");
}

/// Wire the five REST routes and return the served router.
fn build_app() -> axum::Router {
    get("/articles", index);
    post("/articles", store);
    get("/articles/:id", show);
    put("/articles/:id", update);
    delete("/articles/:id", destroy);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    migrate();
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .expect("bind");
    println!("rest-crud-resource listening on http://127.0.0.1:3001");
    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    /// Send `req` through the router; return (status, parsed-JSON-or-Null).
    async fn call(app: &axum::Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let value = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
        (status, value)
    }

    fn json_req(method: &str, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn get_req(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    /// Drive the FULL REST lifecycle against the real framework layers:
    /// create -> show -> list(+relation) -> update -> reject-bad-update ->
    /// show-missing -> delete -> show-deleted.
    #[tokio::test]
    async fn full_rest_lifecycle_is_real() {
        migrate();
        // One author to hang the belongsTo relation off of.
        let author = create!(Author, name = "Ada Lovelace")
            .expect("seed author");
        let author_id = author["id"].as_i64().expect("author id");
        let app = build_app();

        // 1. CREATE -> 201 + a real persisted id, fields echoed back.
        let (status, created) = call(
            &app,
            json_req(
                "POST",
                "/articles",
                &format!(
                    r#"{{"title":"First","body":"Body one","author_id":{author_id}}}"#
                ),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "create returns 201");
        assert_eq!(created["title"], "First");
        assert_eq!(created["body"], "Body one");
        let id = created["id"].as_i64().expect("persisted id");
        assert!(id >= 1, "real auto-increment id");

        // 2. SHOW it -> 200, fields match the created row.
        let (status, shown) = call(&app, get_req(&format!("/articles/{id}"))).await;
        assert_eq!(status, StatusCode::OK, "show returns 200");
        assert_eq!(shown["id"].as_i64().unwrap(), id);
        assert_eq!(shown["title"], "First");
        assert_eq!(shown["body"], "Body one");

        // 3. LIST -> 200, contains our article WITH the eager-loaded `author`
        //    relation populated (typed `with(..).get()`), proving the relation
        //    was really loaded, not faked.
        let (status, list) = call(&app, get_req("/articles")).await;
        assert_eq!(status, StatusCode::OK, "index returns 200");
        let arr = list.as_array().expect("index is a JSON array");
        let ours = arr
            .iter()
            .find(|a| a["id"].as_i64() == Some(id))
            .expect("index contains the created article");
        assert_eq!(ours["title"], "First");
        assert_eq!(
            ours["author"]["name"], "Ada Lovelace",
            "belongsTo author eager-loaded in the index via with(..).get()"
        );

        // 4. UPDATE -> 200, the row really changed.
        let (status, updated) = call(
            &app,
            json_req(
                "PUT",
                &format!("/articles/{id}"),
                &format!(
                    r#"{{"title":"Edited","body":"Changed","author_id":{author_id}}}"#
                ),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "update returns 200");
        assert_eq!(updated["title"], "Edited");
        assert_eq!(updated["body"], "Changed");
        // Confirm the change is persisted, not just echoed.
        let (_, reshown) = call(&app, get_req(&format!("/articles/{id}"))).await;
        assert_eq!(reshown["title"], "Edited");

        // 5. Bad UPDATE (title over the 200-char max) -> 422, row unchanged.
        let long = "x".repeat(300);
        let (status, _) = call(
            &app,
            json_req(
                "PUT",
                &format!("/articles/{id}"),
                &format!(
                    r#"{{"title":"{long}","body":"Changed","author_id":{author_id}}}"#
                ),
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation failure returns 422"
        );
        let (_, still) = call(&app, get_req(&format!("/articles/{id}"))).await;
        assert_eq!(still["title"], "Edited", "rejected update did not persist");

        // 6. SHOW a missing id -> 404.
        let (status, _) = call(&app, get_req("/articles/999999")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "missing row returns 404");

        // 7. DELETE -> 204 No Content.
        let (status, body) = call(
            &app,
            Request::builder()
                .method("DELETE")
                .uri(format!("/articles/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT, "delete returns 204");
        assert_eq!(body, serde_json::Value::Null, "204 carries no body");

        // 8. SHOW the deleted id -> 404 (it is really gone).
        let (status, _) = call(&app, get_req(&format!("/articles/{id}"))).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "deleted row is gone -> 404"
        );
    }
}
