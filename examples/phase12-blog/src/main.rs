//! # Phase 12 Blog — migrated onto RustForge's high-level primitives
//!
//! This example used to hand-write ~18 raw `.await`/`Result<>` lines against
//! bare axum (manual `Router::new().route(..)`, `State`/`Path`/`Multipart`
//! extractors, `Html`/`StatusCode` plumbing). That predates the framework's
//! high-level surface and contradicts the vision (hide async/`Result`, offer a
//! Laravel-like DX). It is now written with ONLY that surface — exactly the set
//! `examples/blog-slice` uses:
//!
//!   * `get`/`post` routing + `global_router().build_router()`
//!   * the `capture_request` middleware
//!   * the `input`/`file` implicit-request globals (no `Request` argument, no
//!     explicit `State`/`Path`/`Multipart` extractors threaded into handlers)
//!   * the `validate!` typed validation DSL
//!   * the `Model!`/`create!`/`find!` ORM macros backed by the real (SQLite) DB
//!   * the `json`/`view` response helpers (no hand-built `Html`/JSON strings)
//!
//! Every handler is argument-less and returns `impl IntoResponse`; there is no
//! visible axum plumbing and no `Result<_, StatusCode>` in a handler signature.
//!
//! Run it:  `cargo run -p phase12-blog --bin blog-server`
//!   (serves on http://127.0.0.1:3000)
//!   GET  /                 HTML index (rendered via `view`)
//!   GET  /posts            JSON list
//!   GET  /posts/{id}        JSON single post (`:id` reaches the handler via `input`)
//!   POST /posts            {"title":"Hello","content":"World"} -> validate + persist
//!   POST /media/upload     multipart `image=@file` -> parsed via the `file` global
use rf::prelude::*;

// A model backed by the real (SQLite) DB.
Model!(Post: title, content);

/// GET / — render the blog index as HTML through the `view` helper, which reads
/// a real on-disk template (`resources/views/home.blade.html`) and interpolates
/// the posts with `@foreach`. No `axum::response::Html`, no manual templating.
async fn home() -> impl axum::response::IntoResponse {
    let posts = Post::all().await.unwrap_or_default();
    view("home", serde_json::json!({ "posts": posts }))
}

/// GET /posts — list every post as JSON (a real SELECT), returned via the
/// `json` global helper rather than a hand-built JSON string.
async fn list_posts() -> impl axum::response::IntoResponse {
    match Post::all().await {
        Ok(posts) => json(posts),
        Err(e) => json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /posts/{id} — show a single post as JSON. The `:id` path param is read via
/// the implicit-request `input` global (no `Request` argument, no `Path`
/// extractor threaded through the handler).
async fn show_post() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => return json(serde_json::json!({ "error": "invalid id" })),
    };
    match Post::find(id).await {
        Ok(Some(post)) => json(post),
        Ok(None) => json(serde_json::json!({ "error": "not found" })),
        Err(e) => json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// POST /posts — validate the request with the `validate!` DSL, persist a real
/// row with `create!`, and return it as JSON. No `.await?`, no `Result` in the
/// signature, no hand-rolled body parsing.
async fn create_post() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(100), content: string }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }));
    }
    let title: String = input("title").unwrap_or_default();
    let content: String = input("content").unwrap_or_default();
    match create!(Post, title = title, content = content) {
        Ok(created) => json(created),
        Err(e) => json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// POST /media/upload — accept a multipart upload through the `file` global,
/// which `capture_request` populates from the request's `multipart/form-data`
/// body. This replaces the old hand-rolled `while multipart.next_field().await`
/// loop with a single implicit-request lookup.
async fn upload_media() -> impl axum::response::IntoResponse {
    match file("image") {
        Some(f) => json(serde_json::json!({
            "success": true,
            "filename": f.filename(),
            "size": f.size(),
            "content_type": f.content_type(),
        })),
        None => json(serde_json::json!({ "success": false, "error": "no file uploaded" })),
    }
}

/// Wire the routes and return the served router (registers on the global router,
/// then attaches the `capture_request` middleware that backs the implicit-request
/// globals).
fn build_app() -> axum::Router {
    get("/", home);
    get("/posts", list_posts);
    get("/posts/{id}", show_post);
    post("/posts", create_post);
    post("/media/upload", upload_media);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

/// Create the posts table (a one-time boot step, not handler logic).
fn migrate() {
    DB::statement(
        "CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT, content TEXT)",
    )
    .expect("create posts table");
}

#[tokio::main]
async fn main() {
    migrate();
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind");
    println!("phase12-blog listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    async fn call(app: &axum::Router, req: Request<Body>) -> (axum::http::StatusCode, String) {
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    fn post_json(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn blog_example_uses_real_high_level_primitives() {
        migrate();
        let app = build_app();

        // POST /posts — validate! + create! persist a real row.
        let (_, out) = call(&app, post_json("/posts", r#"{"title":"Hello","content":"World"}"#)).await;
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["title"], "Hello");
        let created_id = v["id"].as_i64().unwrap();
        assert!(created_id >= 1);

        // GET /posts/{id} — the `:id` path param reaches the argument-less handler
        // through `input::<i64>("id")` and returns the very row we just created.
        let (_, out) = call(
            &app,
            Request::builder()
                .uri(format!("/posts/{created_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let shown: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(shown["id"].as_i64().unwrap(), created_id);
        assert_eq!(shown["title"], "Hello");
        assert_eq!(shown["content"], "World");

        // Invalid create (title too long) -> validate! rejects it.
        let long = "x".repeat(200);
        let (_, out) =
            call(&app, post_json("/posts", &format!(r#"{{"title":"{long}","content":"b"}}"#))).await;
        assert!(out.contains("validation failed"), "expected validation failure, got: {out}");

        // GET / — the `view` helper renders the real on-disk template, listing
        // the persisted post (proves `view` + `@foreach` work end to end).
        let (status, html) =
            call(&app, Request::builder().uri("/").body(Body::empty()).unwrap()).await;
        assert_eq!(status, axum::http::StatusCode::OK, "home render failed: {html}");
        assert!(html.contains("<h1>RustForge Blog</h1>"), "home html: {html}");
        assert!(html.contains("Hello"), "home should list the post title, got: {html}");
        assert!(html.contains(&format!("/posts/{created_id}")), "home should link the post: {html}");

        // POST /media/upload — the `file` global parses the multipart body that
        // `capture_request` staged (replacing the old manual `next_field` loop).
        let boundary = "X-RUSTFORGE-BOUNDARY";
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"photo.png\"\r\nContent-Type: image/png\r\n\r\n{data}\r\n--{b}--\r\n",
            b = boundary,
            data = "PNGDATA!"
        );
        let (_, out) = call(
            &app,
            Request::builder()
                .method("POST")
                .uri("/media/upload")
                .header("content-type", format!("multipart/form-data; boundary={boundary}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await;
        let up: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(up["success"], true, "upload response: {out}");
        assert_eq!(up["filename"], "photo.png");
        assert_eq!(up["size"].as_u64().unwrap(), "PNGDATA!".len() as u64);
    }
}
