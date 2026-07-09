//! # RustForge (rf)
//!
//! Simplified imports for the RustForge framework.
//!
//! ## Quick Start
//! ```ignore
//! // Most common - direct imports:
//! use rf::{Route, Auth, DB, Hash, Collection};
//!
//! // Or use prelude for everything common:
//! use rf::prelude::*;
//! ```
//!
//! ## 5 Main Modules
//!
//! | Module | Description | Key Exports |
//! |--------|-------------|-------------|
//! | `rf::prelude` | Common imports | Route, Auth, DB, Hash, etc. |
//! | `rf::web` | HTTP & Views | Request, Response, Blade, Inertia |
//! | `rf::data` | Database & Validation | ORM, Eloquent, Cache, Validator |
//! | `rf::background` | Background Processing | Jobs, Queue, Events, Broadcast |
//! | `rf::services` | Infrastructure | Storage, Mail, Logging, Metrics |

// ============================================================================
// DIRECT RE-EXPORTS (Most Common - Laravel-style)
// ============================================================================

// Standard library re-exports
pub use std::time::Duration;

// Facades (from merged locations in main crates)
pub use rf_routing::{RouteFacade as Route, global_router, GlobalRouter};
pub use rf_auth::Auth;
// Ready-made bearer-auth guard: `route_layer(from_fn(rf::require_auth))` rejects
// unauthenticated requests with a JSON 401 before the handler + its body extractors.
pub use rf_auth::require_auth;
pub use rf_orm::DB;
pub use rf_cache::CacheFacade as Cache;
pub use rf_events::EventFacade as Event;
pub use rf_storage::StorageFacade as Storage;
pub use rf_logging::Log;
pub use rf_mail::MailFacade as Mail;
pub use rf_web::SessionFacade as Session;
pub use rf_config::Config;
pub use rf_view::ViewFacade as View;

// Helpers
pub use rf_global_helpers::Hash;
pub use rf_global_helpers::redirect;
pub use rf_global_helpers::csrf_token;

// Collections
pub use rf_collections::Collection;
pub use rf_collections::collect;

// Macros
pub use rf_macros::rules;
pub use rf_macros::route;
pub use rf_macros::controller;
pub use rf_macros::Model;

// Eloquent CRUD Macros (Laravel-style without json! or .await!)
pub use rf_macros::update;
pub use rf_macros::create;
pub use rf_macros::find;
pub use rf_macros::delete;

// Laravel Helper Macros (Phase 23)
pub use rf_macros::routes;        // Route definition without || (German keyboard friendly)
pub use rf_macros::migration;     // Database migration DSL
pub use rf_macros::request;       // Form request validation
pub use rf_macros::send_mail;     // Email sending
pub use rf_macros::dispatch;      // Event dispatching
pub use rf_macros::job;           // Background job definition

// Validation
pub use rf_validation_derive::Validate;

// Errors
pub use rf_errors::RustForgeError;
pub use rf_errors::Result;

// Response
pub use rf_response::Response;

// New packages (Phase 22)
pub use rf_pest::Pest;
pub use rf_cashier::Cashier;
pub use rf_mcp::Mcp;
pub use rf_nightwatch::Nightwatch;

// ============================================================================
// MODULE 1: prelude - Common imports for 90% of use cases
// ============================================================================

/// Common imports for most RustForge applications.
///
/// ```ignore
/// use rf::prelude::*;
/// ```
///
/// Includes: Route, Auth, DB, Cache, Hash, Collection, Response, etc.
pub mod prelude {
    // Standard library
    pub use std::time::Duration;

    // All Facades (from merged locations in main crates)
    pub use rf_routing::RouteFacade as Route;
    // Real handler-based route registration: `get("/", home_handler)` etc.,
    // served via `rf_routing::global_router().build_router()`.
    pub use rf_routing::{delete, get, patch, post, put};
    // RESTful resource routing sugar: `resource!("/posts", PostController { index, show, store })`
    // maps the standard REST routes onto a controller's handlers in one call,
    // reusing the real `get`/`post`/`put`/`patch`/`delete` registration above.
    pub use rf_routing::resource;
    // Implicit-request global helpers (behind the `capture_request` middleware):
    // `file("image")`, `input("title")`, `has("title")`.
    pub use rf_request::{file, has, input};
    pub use rf_auth::Auth;
    // Ready-made bearer-auth guard middleware for protected routes:
    // `.route_layer(axum::middleware::from_fn(require_auth))` returns a JSON 401
    // before the handler and its body extractors run (so auth precedes any 422).
    pub use rf_auth::require_auth;
    pub use rf_orm::DB;
    pub use rf_cache::CacheFacade as Cache;
    pub use rf_events::EventFacade as Event;
    pub use rf_storage::StorageFacade as Storage;
    pub use rf_logging::Log;
    pub use rf_mail::MailFacade as Mail;
    pub use rf_web::SessionFacade as Session;
    pub use rf_config::Config;
    pub use rf_view::ViewFacade as View;

    // Helpers
    pub use rf_global_helpers::{Hash, redirect, csrf_token, csrf_field};
    pub use rf_collections::{Collection, collect};

    // Real Response global helpers. `json(data)` builds an `application/json`
    // response from any `Serialize` value; `back()` redirects to the previous
    // page (falling back to `/`); `download(path)` serves a file's bytes with a
    // `Content-Disposition` header. Each returns a `rf_response::ResponseBuilder`,
    // which implements `axum::response::IntoResponse`. (`redirect` above already
    // provides the redirect helper, with its richer flash-message API.)
    // `view(name, data)` renders `resources/views/<name>.blade.html`, interpolating
    // `{{ var }}` from the data, and returns a `text/html` response (a
    // `rf_response::ViewResponse`, which implements `IntoResponse`).
    pub use rf_response::{back, download, json, view};

    // Macros
    pub use rf_macros::{rules, route, controller, Model};
    // `controller_block!` — the vision controller syntax (function-like macro that
    // generates a struct + async, argument-less handler methods returning
    // `IntoResponse`). Its expansion names `rf_response::IntoResponse` by path,
    // which the `rf_response` re-export further below resolves.
    pub use rf_macros::controller_block;
    pub use rf_macros::{update, create, find, delete};  // Eloquent CRUD macros
    pub use rf_macros::{routes, migration, request, send_mail, dispatch};  // Laravel helper macros
    pub use rf_macros::validate;  // typed validation DSL (validates the current request)
    // Laravel-style facade helper macros. Only the ones whose expansion actually
    // compiles against an existing facade are re-exported here. `view!` expands to
    // `rf_response::Response::view(name, data)` — a real file+interpolate renderer
    // (loads `resources/views/<name>.blade.html`, fills `{{ var }}`). `event!`
    // expands to `rf_event_facade::event(payload)` — the real type-keyed sync event
    // bus (fires every listener registered for the payload's concrete type). Still
    // NOT advertised until their engines/signatures are fixed (see VISION_GAP.md):
    // `back!` (needs Response::back wiring), `session!` (targets a non-existent
    // `rf_session` crate), `job!` (targets a missing `rf_job_facade`).
    pub use rf_macros::{auth, cache, event, redirect, storage, view};
    pub use rf_validation_derive::Validate;

    // Support for the `Model!`/`create!`/`find!`/`update!`/`delete!` macros above.
    // Those macros expand to code that names `rf_db_facade::Model`/`QueryBuilder`,
    // `serde_json::json!` and `chrono::` by crate name, and whose CRUD calls
    // (`User::create(..)`) need the `Model` trait in scope. Re-exporting the
    // internal facade crate (plus the `Model` trait) and the `serde_json`/`chrono`
    // crates here makes those paths resolvable through the prelude glob alone, so an
    // `rf`-only consumer no longer has to add the internal `rf-db-facade`/`serde_json`
    // crates by hand. (`serde` itself must still be a direct dependency: the
    // `#[derive(serde::Serialize)]` the macro emits expands to code that links the
    // `serde` crate, which a re-export cannot provide.) The macro `Model` and the
    // trait `Model` live in different namespaces, so both names coexist.
    pub use rf_db_facade::{self, Model, QueryBuilder};
    pub use {chrono, serde_json};

    // The Laravel-style helper macros above (event!/send_mail!/cache!/auth!/
    // storage!/redirect!/session!/view!/back!) expand to code that names their
    // facade crate by path (e.g. `rf_event_facade::Event::dispatch(..)`). Re-export
    // those crates here so the macro expansions resolve through the prelude glob
    // for a consumer that only depends on `rf`.
    pub use {
        rf_auth_facade, rf_cache_facade, rf_event_facade, rf_mail_facade, rf_response,
        rf_route_facade, rf_storage_facade,
    };
    // The `validate!` macro expands to code naming `rf_validation::` and
    // `rf_request::` by crate path; re-export those crate names so it resolves
    // through the prelude glob for an `rf`-only consumer.
    pub use {rf_request, rf_validation};

    // Core Types
    pub use rf_response::Response;
    // Export the framework error type, but NOT the bare one-parameter
    // `rf_errors::Result<T>` alias: glob-imported through `use rf::prelude::*`
    // it SHADOWED std's two-parameter `Result<T, E>`, so any real
    // `Result<T, SomeOtherError>` (e.g. a `rf_queue` Job's
    // `handle() -> Result<(), QueueError>`, or a `.map_err(..)?` with a foreign
    // error) failed to compile with a misleading E0107/E0308/E0277 that never
    // mentioned the shadow (the MediaFlow #1 blocker). Bare `Result` in app
    // code now means `std::result::Result` again. The one-parameter rf alias is
    // still available under a distinct, non-shadowing name for those who want
    // it (or use `rf_core::AppResult` / `rf::services::errors::Result`).
    pub use rf_errors::RustForgeError;
    pub use rf_errors::Result as RfResult;

    // Terse `?`-based handler path (the honest, first-class Result story).
    //
    // A handler can now be written as an argument-less `async fn` returning
    // `AppResult<impl IntoResponse>` and use `?` on every fallible step:
    //   - `validate! {..}?`     -> `From<ValidationErrors>` renders a 422
    //   - `create!(Model, ..)?` -> `From<String>` (ORM error) renders a 500
    //   - `find!(Model, id).or_404()?` -> `OrNotFound` renders a 404
    // `AppError` already implements `axum::IntoResponse` (via `rf-core`'s `axum`
    // feature, enabled below), so the error variant of the returned `Result`
    // maps itself to the correct HTTP status + framework JSON envelope with NO
    // hand-written `match`/`.status()`. This does NOT hide `Result`/`Option`
    // (a language ceiling) — it makes the idiomatic `?` path ergonomic.
    pub use rf_core::{AppError, AppResult, OrNotFound};

    // New packages (Phase 22)
    pub use rf_pest::Pest;
    pub use rf_cashier::Cashier;
    pub use rf_mcp::Mcp;
    pub use rf_nightwatch::Nightwatch;
}

// ============================================================================
// MODULE 2: web - HTTP, Routing, Views, API
// ============================================================================

/// HTTP, Routing, Views, and API resources.
///
/// ```ignore
/// use rf::web::{Request, Response, Router};
/// use rf::web::views::{Blade, Template};
/// use rf::web::api::{Resource, Paginator};
/// ```
pub mod web {
    // Request & Response
    pub use rf_request::*;
    pub use rf_response::*;

    // Routing
    pub use rf_routing::*;

    // API Resources & Pagination
    pub mod api {
        pub use rf_api_resources::*;
        pub use rf_requests::*;
        pub use rf_pagination::*;

        // `rf_api_resources` and `rf_pagination` both define `PaginationLinks` and
        // `PaginationMeta`. Prefer the `rf_pagination` versions here so they stay
        // consistent with `Paginator` (also from `rf_pagination`); this explicit
        // re-export shadows the globs above and resolves the ambiguous-glob-reexport
        // warnings.
        pub use rf_pagination::{PaginationLinks, PaginationMeta};
    }

    // Views & Templates
    pub mod views {
        pub use rf_blade::*;
        pub use rf_view::*;
    }

    // Inertia.js
    pub mod inertia {
        pub use rf_inertia::*;
    }
}

// ============================================================================
// MODULE 3: data - Database, ORM, Cache, Validation, Collections
// ============================================================================

/// Database, ORM, Caching, Validation, and Collections.
///
/// ```ignore
/// use rf::data::{Model, Query, Validator};
/// use rf::data::cache::Store;
/// use rf::data::Collection;
/// ```
pub mod data {
    // ORM: rf_orm is the foundational ORM that `Model`, the model macros, and the
    // rest of the framework are built on, so it is glob-re-exported at the
    // `rf::data` level. rf_eloquent shares many item and module names with rf_orm
    // (`events`, `relationships`, `scopes`, `polymorphic`, `prelude`, `HasScopes`,
    // `HasManyThrough`, `ModelEvents`, ...); glob-merging both here produced
    // ambiguous-glob-reexport warnings, so rf_eloquent lives under its own
    // namespace instead.
    pub use rf_orm::*;

    /// The Eloquent ORM (`rf_eloquent`), re-exported under its own namespace so its
    /// types stay reachable (e.g. `rf::data::eloquent::HasScopes`) without colliding
    /// with the `rf_orm` exports above.
    pub mod eloquent {
        pub use rf_eloquent::*;
    }

    // Collections (re-export at data level)
    pub use rf_collections::{Collection, collect, LazyCollection};

    // Validation
    pub mod validation {
        pub use rf_validation::*;
        pub use rf_validation_derive::Validate;
        pub use rf_macros::rules;
    }

    // Cache
    pub mod cache {
        pub use rf_cache::*;
    }
}

// ============================================================================
// MODULE 4: background - Jobs, Queue, Events, Notifications, Broadcast
// ============================================================================

/// Background processing: Jobs, Queues, Events, Notifications, Broadcasting.
///
/// ```ignore
/// use rf::background::{Job, JobPayload, Scheduler};
/// use rf::background::events::Event;
/// use rf::background::broadcast::Channel;
/// ```
pub mod background {
    // Jobs
    pub use rf_jobs::{
        Job, JobPayload, JobContext, JobError, JobResult,
        JobBatch, JobChain, Scheduler, QueueManager,
    };

    // Queue
    pub mod queue {
        pub use rf_queue::*;
    }

    // Events
    pub mod events {
        pub use rf_events::*;
    }

    // Notifications
    pub mod notifications {
        pub use rf_notifications::*;
    }

    // Broadcasting (WebSocket, SSE)
    pub mod broadcast {
        pub use rf_broadcast::*;
    }

    pub mod sse {
        pub use rf_sse::*;
    }
}

// ============================================================================
// MODULE 5: services - Storage, Mail, Logging, Metrics, Auth, Testing
// ============================================================================

/// Infrastructure services: Storage, Mail, Logging, Auth, Testing.
///
/// ```ignore
/// use rf::services::storage::Disk;
/// use rf::services::mail::Mailable;
/// use rf::services::auth::Guard;
/// ```
pub mod services {
    // Storage & Upload
    pub mod storage {
        pub use rf_storage::*;
        pub use rf_upload::*;
    }

    // Mail
    pub mod mail {
        pub use rf_mail::*;
    }

    // Logging & Metrics
    pub mod logging {
        pub use rf_logging::*;
    }

    pub mod metrics {
        pub use rf_metrics::*;
    }

    // Authentication & Authorization
    pub mod auth {
        pub use rf_auth::*;
        pub use rf_authorization::{gate, Gate, Policy, Authorizable};
    }

    // Sanctum (API Tokens)
    pub mod sanctum {
        pub use rf_sanctum::*;
    }

    // Testing
    pub mod testing {
        pub use rf_testing::*;
        pub use rf_pest::*;
    }

    // Payments
    pub mod payments {
        pub use rf_cashier::*;
    }

    // AI Integration
    pub mod ai {
        pub use rf_mcp::*;
    }

    // Monitoring
    pub mod monitoring {
        pub use rf_nightwatch::*;
    }

    // Errors
    pub mod errors {
        pub use rf_errors::*;
    }
}

// ============================================================================
// BONUS: facades - All facades in one place (für Legacy/Compatibility)
// ============================================================================

/// All Laravel-style facades.
///
/// ```ignore
/// use rf::facades::*;
/// ```
pub mod facades {
    pub use rf_routing::RouteFacade as Route;
    pub use rf_auth::Auth;
    pub use rf_orm::DB;
    pub use rf_cache::CacheFacade as Cache;
    pub use rf_events::EventFacade as Event;
    pub use rf_storage::StorageFacade as Storage;
    pub use rf_logging::Log;
    pub use rf_mail::MailFacade as Mail;
    pub use rf_web::SessionFacade as Session;
    pub use rf_config::Config;
    pub use rf_view::ViewFacade as View;

    // Additional facades (Sanctum, Passport)
    pub use rf_sanctum::Sanctum;
    pub use rf_passport::Passport;
}

// ============================================================================
// BONUS: helpers - All helper functions
// ============================================================================

/// All helper functions and utilities.
///
/// ```ignore
/// use rf::helpers::{slug, snake, env_var};
/// ```
pub mod helpers {
    // String helpers
    pub use rf_helpers::str::{
        slug, snake, camel, studly, kebab, title, plural, singular, limit, words,
    };

    // Array helpers
    pub use rf_helpers::arr::{
        only, except, flatten, collapse, divide, first, last, random, shuffle,
    };

    // Path & URL helpers
    pub use rf_helpers::path;
    pub use rf_helpers::url;

    // Global helpers
    pub use rf_global_helpers::*;

    // Env helpers
    pub use rf_helpers::{env_var, env_or, abort, abort_if, abort_unless};
}

// ============================================================================
// BONUS: core - Framework internals
// ============================================================================

/// Core framework types (rarely needed directly).
pub mod core {
    pub use rf_core::*;
    pub use rf_web::*;
    pub use rf_config::*;
}
