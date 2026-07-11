//! RustForge showcase: Accept-Language-negotiated localized API endpoints.
//!
//! Demonstrates the `rf-i18n` axum integration:
//!
//! - [`rf_i18n::AcceptLanguage`] extracts the best-match locale for each request
//!   from (1) a `?locale=` query parameter, (2) the `Accept-Language` header,
//!   or (3) falls back to `"en"`. Only the primary language subtag is kept
//!   (`"de-DE"` → `"de"`).
//! - A shared `Arc<I18n>` is injected via axum `Extension`; per-request views
//!   are obtained with `I18n::for_locale()`, which is a cheap arc-clone.
//! - Plural-sensitive translations via `I18n::t_plural()` cover the German
//!   `one / other` rule and the English `zero / one / other` rule out of the box.
//!
//! Routes (served on http://127.0.0.1:3009):
//!   GET /greet           — {"locale":"de","message":"Willkommen!"}
//!   GET /items?count=N   — {"locale":"de","count":3,"summary":"3 Artikel"}
//!
//! Run:
//!   cargo run -p i18n-localized-api
//!   curl -H 'Accept-Language: de' http://127.0.0.1:3009/greet
//!   curl -H 'Accept-Language: fr' 'http://127.0.0.1:3009/items?count=5'
//!   curl 'http://127.0.0.1:3009/greet?locale=de'

use std::sync::Arc;

use axum::{
    extract::{Extension, Query},
    routing::get,
    Json, Router,
};
use rf_i18n::{AcceptLanguage, I18n, TranslationCatalog};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// I18n setup
// ---------------------------------------------------------------------------

/// Build a shared `I18n` instance with English, German, and French catalogs.
fn build_i18n() -> Arc<I18n> {
    let en = TranslationCatalog::new("en")
        .add(
            "greeting",
            serde_json::Value::String("Welcome!".into()),
        )
        .add(
            "items",
            serde_json::json!({
                "zero":  "No items",
                "one":   "1 item",
                "other": "{{count}} items"
            }),
        );

    let de = TranslationCatalog::new("de")
        .add(
            "greeting",
            serde_json::Value::String("Willkommen!".into()),
        )
        .add(
            "items",
            serde_json::json!({
                "one":   "1 Artikel",
                "other": "{{count}} Artikel"
            }),
        );

    let fr = TranslationCatalog::new("fr")
        .add(
            "greeting",
            serde_json::Value::String("Bienvenue !".into()),
        )
        .add(
            "items",
            serde_json::json!({
                "one":   "1 article",
                "other": "{{count}} articles"
            }),
        );

    Arc::new(
        I18n::new("en")
            .fallback("en")
            .add_catalog(en)
            .add_catalog(de)
            .add_catalog(fr),
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /greet — returns a locale-aware greeting.
///
/// `AcceptLanguage` resolves the locale from `?locale=` or `Accept-Language`.
/// `Extension(i18n)` is the shared catalog injected at router-build time.
async fn greet(
    AcceptLanguage(locale): AcceptLanguage,
    Extension(i18n): Extension<Arc<I18n>>,
) -> Json<serde_json::Value> {
    let local = i18n.for_locale(&locale);
    let message = local
        .t("greeting", None)
        .unwrap_or_else(|_| "Welcome!".into());
    Json(serde_json::json!({ "locale": locale, "message": message }))
}

/// Query params for `/items`.
#[derive(Deserialize)]
struct ItemsQuery {
    count: Option<i64>,
}

/// GET /items?count=N — returns a locale-aware, plural-sensitive item summary.
async fn items(
    AcceptLanguage(locale): AcceptLanguage,
    Extension(i18n): Extension<Arc<I18n>>,
    Query(params): Query<ItemsQuery>,
) -> Json<serde_json::Value> {
    let count = params.count.unwrap_or(0);
    let local = i18n.for_locale(&locale);
    let summary = local
        .t_plural("items", count)
        .unwrap_or_else(|_| format!("{count} items"));
    Json(serde_json::json!({ "locale": locale, "count": count, "summary": summary }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the axum app.  The `Arc<I18n>` is shared across all requests via an
/// axum `Extension` layer — no `State<…>` wrapper needed for this pattern.
fn build_app(i18n: Arc<I18n>) -> Router {
    Router::new()
        .route("/greet", get(greet))
        .route("/items", get(items))
        .layer(Extension(i18n))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let i18n = build_i18n();
    let app = build_app(i18n);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3009")
        .await
        .expect("bind");
    println!("i18n-localized-api listening on http://127.0.0.1:3009");
    axum::serve(listener, app).await.expect("serve");
}

// ---------------------------------------------------------------------------
// Tests — drive the real axum router via tower's `oneshot`
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `.oneshot()`

    /// Send `req` through a fresh app; return (status, parsed JSON body).
    async fn call(req: Request<Body>) -> (StatusCode, serde_json::Value) {
        let resp = build_app(build_i18n())
            .oneshot(req)
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    // -- /greet ---------------------------------------------------------------

    /// No header and no query param → falls back to English.
    #[tokio::test]
    async fn greet_defaults_to_english() {
        let req = Request::builder().uri("/greet").body(Body::empty()).unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["locale"], "en");
        assert_eq!(body["message"], "Welcome!");
    }

    /// `Accept-Language: de-DE,de;q=0.9,en;q=0.8` → primary tag "de".
    #[tokio::test]
    async fn greet_german_via_accept_language() {
        let req = Request::builder()
            .uri("/greet")
            .header("accept-language", "de-DE,de;q=0.9,en;q=0.8")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["locale"], "de");
        assert_eq!(body["message"], "Willkommen!");
    }

    /// French greeting via `Accept-Language`.
    #[tokio::test]
    async fn greet_french_via_accept_language() {
        let req = Request::builder()
            .uri("/greet")
            .header("accept-language", "fr")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["locale"], "fr");
        assert_eq!(body["message"], "Bienvenue !");
    }

    /// `?locale=fr` overrides `Accept-Language: de`.
    #[tokio::test]
    async fn greet_locale_query_param_overrides_header() {
        let req = Request::builder()
            .uri("/greet?locale=fr")
            .header("accept-language", "de")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["locale"], "fr");
        assert_eq!(body["message"], "Bienvenue !");
    }

    // -- /items ---------------------------------------------------------------

    /// English plural: singular "1 item".
    #[tokio::test]
    async fn items_singular_english() {
        let req = Request::builder()
            .uri("/items?count=1")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["summary"], "1 item");
        assert_eq!(body["count"], 1);
    }

    /// English plural: multiple "5 items".
    #[tokio::test]
    async fn items_plural_english() {
        let req = Request::builder()
            .uri("/items?count=5")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["summary"], "5 items");
    }

    /// English plural: zero uses "zero" catalog key → "No items".
    #[tokio::test]
    async fn items_zero_english() {
        let req = Request::builder()
            .uri("/items?count=0")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["summary"], "No items");
    }

    /// German plural (one/other rules) with count=3 → "3 Artikel".
    #[tokio::test]
    async fn items_plural_german() {
        let req = Request::builder()
            .uri("/items?count=3")
            .header("accept-language", "de")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["locale"], "de");
        assert_eq!(body["summary"], "3 Artikel");
    }

    /// German singular: count=1 → "1 Artikel".
    #[tokio::test]
    async fn items_singular_german() {
        let req = Request::builder()
            .uri("/items?count=1")
            .header("accept-language", "de")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["summary"], "1 Artikel");
    }

    /// Unknown locale falls back to English translations.
    #[tokio::test]
    async fn greet_unknown_locale_falls_back_to_english() {
        let req = Request::builder()
            .uri("/greet")
            .header("accept-language", "xx")
            .body(Body::empty())
            .unwrap();
        let (status, body) = call(req).await;
        assert_eq!(status, StatusCode::OK);
        // I18n::for_locale("xx") + fallback("en") → English message
        assert_eq!(body["message"], "Welcome!");
    }
}
