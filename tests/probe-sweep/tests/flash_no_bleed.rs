// Integration probe: flash_no_bleed
// Adapted from sandbox/probes/flash_no_bleed/src/main.rs
// Proves ViewEngine flash does not bleed across concurrent clients.

use axum::{
    Router,
    body::Body,
    extract::{Query, State},
    http::{Request, header},
    middleware,
    response::Html,
    routing::{get, post},
};
use rf_views::{Context, ViewEngine, helpers::redirect_with_success};
use rf_web::session_scope;
use serde::Deserialize;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt; // oneshot

const TEMPLATE: &str = r#"{% set msg = flash(key="success") %}{% if msg %}{{ msg }}{% else %}NO_FLASH{% endif %}"#;

fn create_engine(tmp: &TempDir) -> Arc<ViewEngine> {
    let views = tmp.path().join("views");
    std::fs::create_dir_all(&views).unwrap();
    std::fs::write(views.join("page.tera"), TEMPLATE).unwrap();
    Arc::new(ViewEngine::new(views.to_str().unwrap()).unwrap())
}

#[derive(Deserialize)]
struct FlashParams {
    msg: String,
}

async fn action_handler(
    State(eng): State<Arc<ViewEngine>>,
    Query(p): Query<FlashParams>,
) -> axum::response::Redirect {
    redirect_with_success(&eng, "/page", p.msg)
}

async fn page_handler(State(eng): State<Arc<ViewEngine>>) -> Html<String> {
    match eng.render("page", &Context::new()) {
        Ok(html) => Html(html),
        Err(e) => Html(format!("ERROR: {e}")),
    }
}

fn app(engine: Arc<ViewEngine>) -> Router {
    Router::new()
        .route("/action", post(action_handler))
        .route("/page", get(page_handler))
        .with_state(engine)
        .layer(middleware::from_fn(session_scope))
}

async fn call(
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    engine: Arc<ViewEngine>,
) -> (String, String) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, format!("rf_session={c}"));
    }
    let req = builder.body(Body::empty()).unwrap();
    let resp = app(engine).oneshot(req).await.unwrap();

    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';').next().and_then(|kv| {
                let mut p = kv.trim().splitn(2, '=');
                (p.next()? == "rf_session").then(|| p.next().map(|x| x.to_string()))?
            })
        })
        .expect("server must set an rf_session cookie");

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (String::from_utf8_lossy(&bytes).to_string(), set_cookie)
}

#[tokio::test]
async fn test_flash_no_bleed() {
    let tmp = TempDir::new().unwrap();
    let engine = create_engine(&tmp);

    // 1. Client A flashes "FLASH_A", reads its own page.
    let (_, cookie_a) = call("POST", "/action?msg=FLASH_A", None, engine.clone()).await;
    let (page_a, _) = call("GET", "/page", Some(&cookie_a), engine.clone()).await;
    assert_eq!(page_a, "FLASH_A", "client A must see its own flash; got: {page_a:?}");

    // 2. Client B flashes "FLASH_B", reads its own page.
    let (_, cookie_b) = call("POST", "/action?msg=FLASH_B", None, engine.clone()).await;
    let (page_b, _) = call("GET", "/page", Some(&cookie_b), engine.clone()).await;
    assert_eq!(page_b, "FLASH_B", "client B must see its own flash; got: {page_b:?}");

    // 3. No bleed: A and B each set a flash on the SAME engine instance.
    let (_, cookie_a2) = call("POST", "/action?msg=ONLY_A", None, engine.clone()).await;
    let (_, cookie_b2) = call("POST", "/action?msg=ONLY_B", None, engine.clone()).await;

    let (a_reads, _) = call("GET", "/page", Some(&cookie_a2), engine.clone()).await;
    assert_eq!(a_reads, "ONLY_A", "BLEED: client A saw {a_reads:?} instead of \"ONLY_A\"");

    let (b_reads, _) = call("GET", "/page", Some(&cookie_b2), engine.clone()).await;
    assert_eq!(b_reads, "ONLY_B", "BLEED: client B saw {b_reads:?} instead of \"ONLY_B\"");

    // 4. Flash has a one-request lifetime.
    let (_, cookie_c) = call("POST", "/action?msg=ONCE", None, engine.clone()).await;
    let (first, _) = call("GET", "/page", Some(&cookie_c), engine.clone()).await;
    assert_eq!(first, "ONCE", "flash must be readable on the next request");
    let (second, _) = call("GET", "/page", Some(&cookie_c), engine.clone()).await;
    assert_eq!(second, "NO_FLASH", "flash must be cleared after one request; got: {second:?}");
}
