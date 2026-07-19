//! # RustForge Full (`rf-full`)
//!
//! The full-surface umbrella crate: everything in [`rf`] plus every extension
//! crate that was previously bundled into `rf` before the cycle-24 umbrella
//! split. Swap your `rf` dependency for `rf-full` to restore the pre-rc.3
//! behaviour.
//!
//! ```toml
//! [dependencies]
//! rf-full = { path = "..." }   # or version = "1.0.0-rc.3"
//! ```
//!
//! ## What rf-full adds on top of rf
//!
//! | Surface | Crate |
//! |---------|-------|
//! | Blade template engine | `rf-blade` |
//! | Inertia.js adapter | `rf-inertia` |
//! | Server-Sent Events | `rf-sse` |
//! | API resources / JSON:API | `rf-api-resources` |
//! | Form requests | `rf-requests` |
//! | Pagination | `rf-pagination` |
//! | File uploads | `rf-upload` |
//! | Gate / Policy authorization | `rf-authorization` |
//! | Testing helpers | `rf-testing` |
//! | Pest-style tests | `rf-pest` |
//! | Stripe billing (Cashier) | `rf-cashier` |
//! | AI integration (MCP) | `rf-mcp` |
//! | Application monitoring | `rf-nightwatch` |
//! | String / array helpers | `rf-helpers` |
//! | Passport OAuth2 | `rf-passport` |
//! | Nova admin panel | `rf-nova` |
//! | Horizon queue dashboard | `rf-horizon` |

// ============================================================================
// CORE — re-export everything the `rf` crate provides
// ============================================================================

// Top-level items from rf (Route, Auth, DB, Cache, …) are re-exported here.
// Module names from rf (prelude, web, data, background, services, facades,
// helpers, core) are shadowed by the explicit module definitions below, which
// extend them with the extension surfaces.
pub use rf::*;

// ============================================================================
// EXTENSION TOP-LEVEL ITEMS (backward compatibility with old rf prelude)
// ============================================================================

/// Pest-style testing facade (moved from rf prelude to rf-full in rc.3).
pub use rf_pest::Pest;

/// Stripe billing integration (moved from rf prelude to rf-full in rc.3).
pub use rf_cashier::Cashier;

/// AI/MCP integration (moved from rf prelude to rf-full in rc.3).
pub use rf_mcp::Mcp;

/// Application monitoring (moved from rf prelude to rf-full in rc.3).
pub use rf_nightwatch::Nightwatch;

/// Passport OAuth2 facade (moved from rustforge and rf-full in rc.3).
pub use rf_passport_facade::Passport;

/// Nova admin panel (moved from rustforge to rf-full in rc.3).
pub use rf_nova::Nova;

/// Horizon queue dashboard (moved from rustforge to rf-full in rc.3).
pub use rf_horizon::Horizon;

// ============================================================================
// EXTENSION MODULES
// ============================================================================

/// Web extension surfaces: API resources, Inertia, Blade, file uploads.
pub mod web_ext {
    /// JSON:API resources, pagination, and form-request validation.
    pub mod api {
        pub use rf_api_resources::*;
        pub use rf_requests::*;
        pub use rf_pagination::*;
        // resolve ambiguous PaginationLinks/Meta from the two crates
        pub use rf_pagination::{PaginationLinks, PaginationMeta};
    }

    /// Blade template engine (Laravel-compatible `.blade.html` files).
    pub mod blade {
        pub use rf_blade::*;
    }

    /// Inertia.js adapter for building SPAs with server-side routing.
    pub mod inertia {
        pub use rf_inertia::*;
    }

    /// File upload utilities (multipart, validation, storage integration).
    pub mod upload {
        pub use rf_upload::*;
    }
}

/// Background extension surfaces: Server-Sent Events.
pub mod background_ext {
    /// Server-Sent Events streaming.
    pub mod sse {
        pub use rf_sse::*;
    }
}

/// Service extension surfaces: authorization, testing, payments, AI, monitoring.
pub mod services_ext {
    /// Gate / Policy authorization (moved from rf::services::auth in rc.3).
    pub mod auth {
        pub use rf_authorization::*;
    }

    /// Testing helpers and Pest-style test runner.
    pub mod testing {
        pub use rf_testing::*;
        pub use rf_pest::*;
    }

    /// Stripe billing / subscription management (Cashier).
    pub mod payments {
        pub use rf_cashier::*;
    }

    /// AI integration via Model Context Protocol (MCP).
    pub mod ai {
        pub use rf_mcp::*;
    }

    /// Application monitoring and health checks (Nightwatch).
    pub mod monitoring {
        pub use rf_nightwatch::*;
    }
}

/// String / array / path / URL helpers (Laravel-style).
pub mod helpers_ext {
    pub use rf_helpers::*;
}

/// Passport OAuth2 server surfaces.
pub mod passport {
    // rf_passport_facade re-exports everything from rf_passport under the
    // facade wrappers; glob-merging both produces ambiguous-glob-reexport
    // warnings, so we use rf_passport_facade as the authoritative glob and
    // re-export only items from rf_passport that the facade does not cover.
    pub use rf_passport_facade::*;
    // Underlying OAuth2 types not wrapped by the facade:
    pub use rf_passport::{
        PassportAuth, PassportConfig, PassportError, PassportResult,
        HasApiTokens, OAuthClient, OAuthAccessToken, OAuthRefreshToken,
        Scope, ScopeRepository,
        AuthorizationCodeGrant, PasswordGrant, ClientCredentialsGrant,
        RefreshTokenGrant, ImplicitGrant,
        AuthorizationRequest, AuthorizationResponse, TokenResponse,
        generate_code_verifier, generate_code_challenge, verify_code_challenge,
    };
}

/// Nova admin panel (experimental).
pub mod nova {
    pub use rf_nova::*;
}

/// Horizon queue monitoring dashboard (experimental).
pub mod horizon {
    pub use rf_horizon::*;
}

// ============================================================================
// FULL PRELUDE — everything in one glob import
// ============================================================================

/// Full prelude: all of `rf::prelude` plus extension surfaces.
///
/// ```ignore
/// use rf_full::full_prelude::*;
/// ```
pub mod full_prelude {
    pub use rf::prelude::*;

    // Extension top-level items
    pub use rf_pest::Pest;
    pub use rf_cashier::Cashier;
    pub use rf_mcp::Mcp;
    pub use rf_nightwatch::Nightwatch;
    pub use rf_passport_facade::Passport;
}
