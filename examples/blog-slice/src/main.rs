//! RustForge flagship vertical slice: request -> validate -> model -> response.
//!
//! A tiny blog API written with ONLY the framework's high-level primitives —
//! `get`/`post` routing, the `capture_request` middleware, the `validate!` DSL,
//! the `Model!`/`create!` ORM macros and `DB` — with NO `Request` argument
//! threaded through handlers and no visible `.await?` in the validation path.
//!
//! Run it:  `cargo run -p blog-slice`  (serves on http://127.0.0.1:3000)
//!   POST /posts       {"title":"Hello","body":"World"}
//!   GET  /posts
//!   GET  /posts/{id}    (the `:id` path param reaches the handler via `input`)
use rf::prelude::*;

// A model backed by the real (SQLite) DB.
Model!(Post: title, body);

/// POST /posts — validate the request, persist a real row, return it as JSON.
///
/// Returns a real typed response via the `json(..)` global helper (an
/// `application/json` `ResponseBuilder`) rather than a hand-built JSON string.
async fn create_post() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(100), body: string }.is_err() {
        return json(serde_json::json!({"error": "validation failed"}));
    }
    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(created) => json(created),
        Err(e) => json(serde_json::json!({"error": e.to_string()})),
    }
}

/// GET /posts — list every post as JSON (a real SELECT).
async fn list_posts() -> impl axum::response::IntoResponse {
    match Post::all().await {
        Ok(posts) => json(posts),
        Err(e) => json(serde_json::json!({"error": e.to_string()})),
    }
}

/// GET /posts/{id} — show a single post. The `:id` path param is available to the
/// implicit-request globals (no `Request` argument, no explicit `Path` extractor).
async fn show_post() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => return json(serde_json::json!({"error": "invalid id"})),
    };
    match Post::find(id).await {
        Ok(Some(post)) => json(post),
        Ok(None) => json(serde_json::json!({"error": "not found"})),
        Err(e) => json(serde_json::json!({"error": e.to_string()})),
    }
}

/// Wire the routes and return the served router (registers on the global router).
fn build_app() -> axum::Router {
    post("/posts", create_post);
    get("/posts", list_posts);
    get("/posts/{id}", show_post);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    DB::statement("CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT, body TEXT)")
        .expect("create table");
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind");
    println!("blog-slice listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    async fn call(app: &axum::Router, req: Request<Body>) -> String {
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
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
    async fn request_validate_model_response_slice_is_real() {
        DB::statement("CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT, body TEXT)")
            .unwrap();
        let app = build_app();

        // Valid create -> persisted with a real id.
        let out = call(&app, post_json("/posts", r#"{"title":"Hello","body":"World"}"#)).await;
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["title"], "Hello");
        let created_id = v["id"].as_i64().unwrap();
        assert!(created_id >= 1);

        // GET /posts/{id} -> the `:id` path param reaches the argument-less handler
        // via the implicit-request globals (`input::<i64>("id")`), and it returns
        // the very row we just created.
        let out = call(
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
        assert_eq!(shown["body"], "World");

        // Invalid create (title too long) -> validation rejects it.
        let long = "x".repeat(200);
        let out = call(&app, post_json("/posts", &format!(r#"{{"title":"{long}","body":"b"}}"#))).await;
        assert!(out.contains("validation failed"));

        // List -> exactly the one valid post.
        let out = call(&app, Request::builder().uri("/posts").body(Body::empty()).unwrap()).await;
        let list: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["body"], "World");

        // GET /posts/{id} for a missing row -> the path param is really read and
        // resolved to "not found" (same app instance; the global router is
        // process-wide, so we can't call build_app() twice in another test).
        let out = call(
            &app,
            Request::builder()
                .uri("/posts/999999")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert!(out.contains("not found"), "expected not-found, got: {out}");
    }
}
