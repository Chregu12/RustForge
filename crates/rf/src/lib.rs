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

// Facades
pub use rf_route_facade::Route;
pub use rf_auth_facade::Auth;
pub use rf_db_facade::DB;
pub use rf_cache_facade::Cache;
pub use rf_event_facade::Event;
pub use rf_storage_facade::Storage;
pub use rf_log_facade::Log;
pub use rf_mail_facade::Mail;
pub use rf_session_facade::Session;
pub use rf_config_facade::Config;
pub use rf_view_facade::View;

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

// Validation
pub use rf_validation_derive::Validate;

// Errors
pub use rf_errors::RustForgeError;
pub use rf_errors::Result;

// Response
pub use rf_response::Response;

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
    // All Facades
    pub use rf_route_facade::Route;
    pub use rf_auth_facade::Auth;
    pub use rf_db_facade::DB;
    pub use rf_cache_facade::Cache;
    pub use rf_event_facade::Event;
    pub use rf_storage_facade::Storage;
    pub use rf_log_facade::Log;
    pub use rf_mail_facade::Mail;
    pub use rf_session_facade::Session;
    pub use rf_config_facade::Config;
    pub use rf_view_facade::View;

    // Helpers
    pub use rf_global_helpers::{Hash, redirect, csrf_token, csrf_field};
    pub use rf_collections::{Collection, collect};

    // Macros
    pub use rf_macros::{rules, route, controller, Model};
    pub use rf_validation_derive::Validate;

    // Core Types
    pub use rf_response::Response;
    pub use rf_errors::{RustForgeError, Result};
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
    // ORM & Eloquent
    pub use rf_orm::*;
    pub use rf_eloquent::*;

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
    pub use rf_route_facade::Route;
    pub use rf_auth_facade::Auth;
    pub use rf_db_facade::DB;
    pub use rf_cache_facade::Cache;
    pub use rf_event_facade::Event;
    pub use rf_storage_facade::Storage;
    pub use rf_log_facade::Log;
    pub use rf_mail_facade::Mail;
    pub use rf_session_facade::Session;
    pub use rf_config_facade::Config;
    pub use rf_view_facade::View;
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
