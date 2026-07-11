# rf-i18n

Internationalization (i18n) for RustForge.

## Status

**Stable core.** The translation core (JSON catalogs, dot-key lookup,
Handlebars interpolation, pluralization, locale fallback) is production-ready
and covered by tests. The axum extractor (`AcceptLanguage`) is behind the
optional `axum` feature and is experimental — the API may evolve.

Date/number/currency formatting is simplified (not locale-aware beyond
German decimal separator); for production formatting use `icu4x` or `chrono`.

## Features

- **JSON catalogs** — load translations from a JSON string or build them with
  the builder API.
- **Dot-key lookup** — `i18n.t("messages.welcome", …)` resolves nested keys.
- **Handlebars interpolation** — `"Hello, {{name}}!"` with typed `serde_json`
  data. Missing variables resolve to empty string, never leaking `{{…}}` into
  responses.
- **Pluralization** — English, German, French, Slavic (ru/uk/be), Arabic rules
  built-in; unknown locales fall back to English rules with a `tracing::warn`.
- **Locale fallback** — missing keys in the active locale fall back to the
  configured fallback locale.
- **Cheap clone** — `I18n` is `Clone`; catalogs are behind `Arc`. Use
  `Arc<I18n>` + `I18n::for_locale(locale)` for per-request instances without
  rebuilding the catalog.
- **axum extractor** (opt-in `axum` feature) — `AcceptLanguage` parses the
  `Accept-Language` header and/or `?locale=` query param into a best-match
  locale tag.

## Installation

```toml
[dependencies]
rf-i18n = { path = "../rf-i18n" }

# With axum per-request locale negotiation:
rf-i18n = { path = "../rf-i18n", features = ["axum"] }
```

## Usage

### Basic

```rust
use rf_i18n::{I18n, TranslationCatalog};
use serde_json::json;

let catalog = TranslationCatalog::new("en")
    .add("greeting", serde_json::Value::String("Hello, {{name}}!".into()));

let i18n = I18n::new("en").add_catalog(catalog);

// Interpolated translation
let msg = i18n.t("greeting", Some(json!({ "name": "Alice" })))?;
assert_eq!(msg, "Hello, Alice!");

// Missing variable → empty string, not {{name}}
let safe = i18n.t("greeting", None)?;
assert_eq!(safe, "Hello, !");

// Pluralization
let items_en = i18n.t_plural("items", 5)?;
```

### Arc<I18n> + per-request locale (no axum feature needed)

```rust
use std::sync::Arc;

let shared: Arc<I18n> = Arc::new(i18n);
let de = shared.for_locale("de"); // cheap clone, shared catalog
let msg = de.t("greeting", Some(json!({ "name": "Hans" })))?;
```

### axum integration (requires `axum` feature)

```rust
use std::sync::Arc;
use axum::{routing::get, Router, Extension};
use rf_i18n::{I18n, AcceptLanguage};

async fn greet(
    AcceptLanguage(locale): AcceptLanguage,
    Extension(i18n): Extension<Arc<I18n>>,
) -> String {
    i18n.for_locale(&locale).t("greeting", None).unwrap_or_default()
}

let i18n: Arc<I18n> = Arc::new(/* … */);
let app = Router::new()
    .route("/hello", get(greet))
    .layer(Extension(i18n));
```

The `AcceptLanguage` extractor checks `?locale=<tag>` first, then the
`Accept-Language` header, then defaults to `"en"`. It returns only the primary
subtag (`"de-DE"` → `"de"`).

## Documentation

```bash
cargo doc --package rf-i18n --open
```

## License

MIT OR Apache-2.0

## Part of RustForge

This crate is part of [RustForge](https://github.com/Chregu12/RustForge),
a comprehensive full-stack application framework for Rust.
