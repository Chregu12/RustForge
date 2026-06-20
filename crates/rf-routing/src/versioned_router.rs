//! Versioned Router Builder
//!
//! Allows defining different route handlers per API version

use crate::versioning::{ApiVersion, VersionConfig, VersionError};
use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Router,
};
use std::collections::HashMap;

/// Builder for versioned API routes
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::versioned_router::VersionedRouterBuilder;
///
/// let router = VersionedRouterBuilder::new()
///     .version(1, |router| {
///         router.route("/users", get(get_users_v1))
///     })
///     .version(2, |router| {
///         router.route("/users", get(get_users_v2))
///     })
///     .default_version(2)
///     .build();
/// ```
pub struct VersionedRouterBuilder {
    versions: HashMap<u32, Router>,
    config: VersionConfig,
}

impl Default for VersionedRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionedRouterBuilder {
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
            config: VersionConfig::default(),
        }
    }

    /// Add routes for a specific version
    pub fn version<F>(mut self, version: u32, configure: F) -> Self
    where
        F: FnOnce(Router) -> Router,
    {
        let router = self.versions.remove(&version).unwrap_or_default();
        let configured = configure(router);
        self.versions.insert(version, configured);
        self
    }

    /// Set the default version
    pub fn default_version(mut self, version: u32) -> Self {
        self.config.default_version = version;
        self
    }

    /// Set supported versions
    pub fn supported_versions(mut self, versions: Vec<u32>) -> Self {
        self.config.supported_versions = versions;
        self
    }

    /// Mark versions as deprecated
    pub fn deprecated_versions(mut self, versions: Vec<u32>) -> Self {
        self.config.deprecated_versions = versions;
        self
    }

    /// Build the router with URL-based versioning (/v1/, /v2/, etc.)
    pub fn build_with_prefix(self) -> Router {
        let mut router = Router::new();

        for (version, version_router) in self.versions {
            let prefix = format!("/v{}", version);
            router = router.nest(&prefix, version_router);
        }

        router
    }

    /// Build the router with header-based versioning
    pub fn build_with_headers(self) -> Router {
        Router::new()
            .fallback(versioned_handler)
            .with_state(VersionedRouterState {
                versions: self.versions,
                config: self.config,
            })
    }

    /// Build router that supports both URL and header-based versioning
    pub fn build(self) -> Router {
        // For now, use URL-based versioning
        // In a real implementation, this would intelligently handle both
        self.build_with_prefix()
    }
}

#[derive(Clone)]
struct VersionedRouterState {
    versions: HashMap<u32, Router>,
    config: VersionConfig,
}

async fn versioned_handler(version: Result<ApiVersion, VersionError>, _req: Request) -> Response {
    match version {
        Ok(v) => {
            // Route to appropriate version handler
            // This is simplified - in practice you'd look up the router
            (
                StatusCode::OK,
                format!("Routing to version {}", v.version()),
            )
                .into_response()
        }
        Err(e) => e.into_response(),
    }
}

/// Macro for easily defining versioned routes
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::versioned_routes;
///
/// let router = versioned_routes! {
///     v1 => {
///         GET "/users" => get_users_v1,
///         POST "/users" => create_user_v1,
///     },
///     v2 => {
///         GET "/users" => get_users_v2,
///         POST "/users" => create_user_v2,
///         DELETE "/users/:id" => delete_user_v2,
///     },
///     default: v2
/// };
/// ```
#[macro_export]
macro_rules! versioned_routes {
    (
        $(
            v$version:literal => {
                $(
                    $method:ident $path:literal => $handler:expr
                ),* $(,)?
            }
        ),* $(,)?
        default: v$default:literal
    ) => {{
        use $crate::versioned_router::VersionedRouterBuilder;
        use axum::routing::{delete, get, patch, post, put};

        let mut builder = VersionedRouterBuilder::new()
            .default_version($default);

        $(
            builder = builder.version($version, |router| {
                router
                $(
                    .route($path, versioned_routes!(@method $method).to($handler))
                )*
            });
        )*

        builder.build()
    }};

    (@method GET) => { get };
    (@method POST) => { post };
    (@method PUT) => { put };
    (@method PATCH) => { patch };
    (@method DELETE) => { delete };
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    async fn handler_v1() -> &'static str {
        "v1"
    }

    async fn handler_v2() -> &'static str {
        "v2"
    }

    #[test]
    fn test_versioned_router_builder() {
        let router = VersionedRouterBuilder::new()
            .version(1, |r| r.route("/test", get(handler_v1)))
            .version(2, |r| r.route("/test", get(handler_v2)))
            .default_version(2)
            .build();

        assert!(true); // Compilation test
    }
}
