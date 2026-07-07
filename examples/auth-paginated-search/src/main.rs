//! RustForge canonical AUTH + PAGINATION + SEARCH slice: a single protected
//! endpoint that searches the authenticated user's own posts by title, paginated,
//! written with ONLY the framework's real high-level primitives.
//!
//! It ties together, end to end:
//!   * `rf_auth`'s REAL `with_auth_scope` per-request scope primitive (the same
//!     one that backs the `rf_auth::middleware::auth_scope` middleware) that gives
//!     every request its own isolated authentication scope (so one request's login
//!     can never leak into another),
//!   * `Auth::login_using_id` / `Auth::check` / `Auth::id` — the REAL auth facade:
//!     a tiny bearer-token bridge logs the caller into the request's auth scope,
//!     the handler guards with `Auth::check()` (401 for guests) and scopes the
//!     search to `Auth::id()`,
//!   * the implicit-request globals `input("field")` (behind `capture_request`)
//!     to read the `?q=` search term and `?page=` page number off the query
//!     string with no `Request` argument,
//!   * the REAL `QueryBuilder` search + pagination:
//!     `DB::table("posts").where_eq("user_id", ..).where_like("title", "%q%")
//!     .order_by("id","asc").paginate(per_page, page)`, and
//!   * `json(..)` responses carrying the correct status codes: 401 when not
//!     authenticated, 200 with the filtered + paginated page otherwise.
//!
//! Note on the search columns: the request term is matched with `where_like` on
//! `title`, scoped to the caller via `where_eq("user_id", ..)`. Searching
//! `title OR body` *within a user scope* would require a grouped
//! `user_id = ? AND (title LIKE ? OR body LIKE ?)`, but the current `QueryBuilder`
//! renders OR conditions without parentheses (`a AND b OR c`), which would change
//! the precedence and leak other users' rows — so the search stays on `title` to
//! keep the user scoping correct rather than fake a broken OR.
//!
//! Run it:  `cargo run -p auth-paginated-search`  (serves on http://127.0.0.1:3002)
//!   GET /posts/search?q=rust&page=1     Authorization: Bearer <user_id>
use axum::http::StatusCode;
use rf::prelude::*;
use serde_json::json;

// The resource under test. `user_id` is the owner FK the search scopes on.
Model!(Post {
    title: String,
    body: String,
    user_id: i64,
});

/// GET /posts/search?q=..&page=.. — protected, paginated title search over the
/// authenticated user's own posts.
///
/// * 401 if the request carries no authenticated user (real `Auth::check()`),
/// * otherwise 200 with the page: the caller's posts whose title matches `q`,
///   ordered by id, sliced to `PER_PAGE` per `page`, plus the pagination
///   metadata from the real `QueryBuilder::paginate`.
async fn search() -> impl axum::response::IntoResponse {
    // Route-level protection: guests are rejected before any query runs.
    if !Auth::check() {
        return json(json!({ "error": "unauthenticated" })).status(StatusCode::UNAUTHORIZED);
    }
    // Scope the search to the caller. `Auth::check()` above guarantees `Some`.
    let user_id = Auth::id().unwrap_or_default() as i64;

    // Read the search term + page off the query string via the implicit-request
    // globals. Query values arrive as strings, so `page` is parsed explicitly.
    let q: String = input("q").unwrap_or_default();
    let page: usize = input::<String>("page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    const PER_PAGE: usize = 2;

    match DB::table("posts")
        .where_eq("user_id", user_id)
        .where_like("title", format!("%{}%", q))
        .order_by("id", "asc")
        .paginate(PER_PAGE, page)
        .await
    {
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

/// Per-request auth middleware: opens a fresh isolated authentication scope with
/// the framework's real `with_auth_scope` primitive (the exact one that backs
/// `rf_auth::middleware::auth_scope`), then, *inside* that scope, bridges an
/// `Authorization: Bearer <user_id>` header into a real `Auth::login_using_id`.
/// Requests without a valid bearer token stay guests (rejected by `Auth::check`),
/// and because the login lives in this request's scope it never leaks to another.
///
/// (This is hand-written against the example's axum 0.7 rather than layering
/// `rf_auth::middleware::auth_scope`, which is compiled against the workspace's
/// axum 0.8 and so cannot be attached to this 0.7 router. The scope + login
/// primitives it calls are the identical, real framework ones.)
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
            let _ = Auth::login_using_id(id, false);
        }
        next.run(req).await
    })
    .await
}

/// Ensure the backing table exists (real SQLite DDL on the global manager).
fn migrate() {
    DB::statement(
        "CREATE TABLE IF NOT EXISTS posts (\
             id INTEGER PRIMARY KEY, title TEXT, body TEXT, user_id INTEGER)",
    )
    .expect("create posts table");
}

/// Wire the single protected route and return the served router. The layer order
/// (outermost last) is: `capture_request` (parse query/body) -> `auth_scope_login`
/// (fresh isolated auth scope + bearer -> scope login) -> handler.
fn build_app() -> axum::Router {
    get("/posts/search", search);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(auth_scope_login))
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    migrate();
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3002")
        .await
        .expect("bind");
    println!("auth-paginated-search listening on http://127.0.0.1:3002");
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

    fn get_req(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    fn auth_get(uri: &str, user_id: u64) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("Authorization", format!("Bearer {user_id}"))
            .body(Body::empty())
            .unwrap()
    }

    /// Drive the protected, paginated search against the real framework layers:
    /// (1) no auth -> 401, (2) authenticated search -> 200 with the caller's own
    /// posts, correctly filtered by the term, paginated, and never leaking a
    /// different user's rows.
    #[tokio::test]
    async fn auth_protected_paginated_search_is_real() {
        migrate();
        let app = build_app();

        // Seed: user 1 owns four "Rust*" posts + one "Python" post; user 2 owns a
        // "Rust" post that must NEVER appear in user 1's results.
        for (title, body, uid) in [
            ("Rust ownership", "borrow checker", 1),
            ("Rust lifetimes", "elision rules", 1),
            ("Rust async", "futures and tasks", 1),
            ("Rust macros", "declarative and proc", 1),
            ("Python basics", "not rust at all", 1),
            ("Rust for user two", "should stay hidden", 2),
        ] {
            create!(Post, title = title, body = body, user_id = uid).expect("seed post");
        }

        // 1. WITHOUT auth -> 401, no data leaked.
        let (status, body) = call(&app, get_req("/posts/search?q=Rust&page=1")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "guest is rejected");
        assert_eq!(body["error"], "unauthenticated");

        // 2. WITH auth as user 1, page 1 of the "Rust" title search.
        //    User 1 has FOUR matching "Rust*" posts (the "Python" one excluded,
        //    user 2's "Rust" post excluded by the owner scope). PER_PAGE = 2, so
        //    page 1 has 2 items, total 4, last_page 2.
        let (status, page1) = call(&app, auth_get("/posts/search?q=Rust&page=1", 1)).await;
        assert_eq!(status, StatusCode::OK, "authenticated search returns 200");
        assert_eq!(page1["total"], 4, "four of user 1's posts match 'Rust'");
        assert_eq!(page1["per_page"], 2);
        assert_eq!(page1["current_page"], 1);
        assert_eq!(page1["last_page"], 2);
        let d1 = page1["data"].as_array().expect("data array");
        assert_eq!(d1.len(), 2, "page 1 holds PER_PAGE items");
        assert_eq!(d1[0]["title"], "Rust ownership");
        assert_eq!(d1[1]["title"], "Rust lifetimes");
        // No row from another user, and no non-matching row, leaked onto the page.
        for row in d1 {
            assert_eq!(row["user_id"].as_i64(), Some(1), "only the caller's rows");
            assert!(
                row["title"].as_str().unwrap().contains("Rust"),
                "only rows matching the search term"
            );
        }

        // 3. Page 2 holds the remaining two matches.
        let (status, page2) = call(&app, auth_get("/posts/search?q=Rust&page=2", 1)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page2["current_page"], 2);
        let d2 = page2["data"].as_array().expect("data array");
        assert_eq!(d2.len(), 2, "page 2 holds the last two matches");
        assert_eq!(d2[0]["title"], "Rust async");
        assert_eq!(d2[1]["title"], "Rust macros");

        // 4. A narrower term filters correctly (real LIKE, not a fake match).
        let (status, one) = call(&app, auth_get("/posts/search?q=lifetimes&page=1", 1)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(one["total"], 1, "'lifetimes' matches exactly one title");
        assert_eq!(one["data"][0]["title"], "Rust lifetimes");

        // 5. User 2's isolated scope sees ONLY its own single "Rust" post,
        //    proving the per-request auth scope + owner filter really isolate data.
        let (status, u2) = call(&app, auth_get("/posts/search?q=Rust&page=1", 2)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(u2["total"], 1, "user 2 owns exactly one matching post");
        assert_eq!(u2["data"][0]["title"], "Rust for user two");
    }
}
