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
//!
//! ## Canonical entry points
//!
//! Use `#[auto_await]` (or its string-free alias `#[await_calls(..)]`) to write
//! await-free async code, and `#[model]` to define models. The PascalCase
//! `Model!` macro is kept for the short inline form, but `#[model]` is preferred
//! for new code. Opt a single item out of auto-await with `#[no_auto_await]`
//! (alias `#[sync]`). The Laravel-style helper macros (`now!`, `view!`,
//! `cache!`, `create!`, `find!`, `routes!`, `dispatch!`, ...) are available from
//! both `rf` and `rustforge` so the public macro surface stays consistent.

// ============================================================================
// DIRECT RE-EXPORTS (Most Common - Laravel-style)
// ============================================================================

// Standard library re-exports
pub use std::time::Duration;

// Facades (from merged locations in main crates)
pub use rf_routing::{RouteFacade as Route, global_router, GlobalRouter};
pub use rf_auth::Auth;
/// Low-level `DB` string facade. **Not wired to a live database**: its
/// `DB::table()` terminal methods return a clear error rather than fabricating
/// results. For real persistence use the typed, SeaORM-backed model API.
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

// Await-handling attribute macros (aligned with `rustforge`)
pub use rf_macros::auto_await;
pub use rf_macros::await_calls;
pub use rf_macros::sync;
pub use rf_macros::no_auto_await;

// Eloquent CRUD Macros (Laravel-style without json! or .await!)
pub use rf_macros::update;
pub use rf_macros::create;
pub use rf_macros::find;
pub use rf_macros::delete;

// Laravel Helper Macros (Phase 23)
pub use rf_macros::routes;        // Route definition without || (German keyboard friendly)
pub use rf_macros::resource;      // RESTful resource routes
pub use rf_macros::migration;     // Database migration DSL
pub use rf_macros::request;       // Form request validation
pub use rf_macros::send_mail;     // Email sending
pub use rf_macros::dispatch;      // Event dispatching
pub use rf_macros::job;           // Background job definition

// Laravel-style helper macros (aligned with `rustforge`).
// These are function-like macros from rf_macros; the `redirect`/`csrf` macros
// live in the macro namespace and coexist with the same-named helper functions
// re-exported above from rf_global_helpers.
pub use rf_macros::{now, view, bcrypt, redirect, cache, auth, csrf};
pub use rf_macros::{dd, dump, event, logger, session, storage, asset};
pub use rf_macros::{back, old, url, collect, config, env_var};
pub use rf_macros::{abort, abort_if, response, rescue, report, function};
pub use rf_macros::{mail, mailable, markdown, notification, blade, html};
pub use rf_macros::{section, push, stack, form_request, validated};
pub use rf_macros::{exception_handler, handle_exceptions};

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

    // Helpers
    pub use rf_global_helpers::{Hash, redirect, csrf_token, csrf_field};
    pub use rf_collections::{Collection, collect};

    // Macros
    pub use rf_macros::{rules, route, controller, Model};
    pub use rf_macros::{auto_await, await_calls, sync, no_auto_await};  // Await-handling attributes
    pub use rf_macros::{update, create, find, delete};  // Eloquent CRUD macros
    pub use rf_macros::{routes, resource, migration, request, send_mail, dispatch, job};  // Laravel helper macros
    // Laravel-style helper macros (aligned with `rustforge`)
    pub use rf_macros::{now, view, bcrypt, redirect, cache, auth, csrf};
    pub use rf_macros::{dd, dump, event, logger, session, storage, asset};
    pub use rf_macros::{back, old, url, collect, config, env_var};
    pub use rf_macros::{abort, abort_if, response, rescue, report, function};
    pub use rf_macros::{mail, mailable, markdown, notification, blade, html};
    pub use rf_macros::{section, push, stack, form_request, validated};
    pub use rf_macros::{exception_handler, handle_exceptions};
    pub use rf_validation_derive::Validate;

    // Core Types
    pub use rf_response::Response;
    pub use rf_errors::{RustForgeError, Result};

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
