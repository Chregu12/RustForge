//! # rf-routing
//!
//! Complete routing system for Rust web applications with Laravel-like features.
//!
//! ## Features
//!
//! - **Named Routes**: Named routes with parameters and URL generation
//! - **Signed URLs**: Signed URLs with expiration support
//! - **Route Groups**: Organize routes with shared prefixes, middleware, and configuration
//! - **Middleware Pipeline**: Named middleware registration and execution
//! - **Controllers**: RESTful controller traits and routing
//! - **Resource Routing**: Automatic RESTful route generation
//! - **Convenient Macros**: Laravel-like macros for route definition
//!
//! ## Quick Start
//!
//! ```rust
//! use rf_routing::{NamedRoute, RouteRegistry, route_params};
//!
//! let mut registry = RouteRegistry::new();
//!
//! // Register routes
//! let route = NamedRoute::new("users.show", "/users/{id}");
//! registry.register(route);
//!
//! // Generate URL
//! let params = route_params! {
//!     "id" => 123
//! };
//! let url = registry.url("users.show", &params);
//! assert_eq!(url, Some("/users/123".to_string()));
//! ```
//!
//! ## Route Groups
//!
//! ```rust,no_run
//! use rf_routing::RouteGroup;
//! use axum::{Router, routing::get};
//!
//! async fn handler() -> &'static str { "Hello!" }
//!
//! let group = RouteGroup::new()
//!     .prefix("/api")
//!     .middleware("auth")
//!     .name("api.");
//!
//! let router = Router::new()
//!     .route("/users", get(handler));
//!
//! let router = group.apply(router);
//! ```
//!
//! ## Resource Routing
//!
//! ```rust
//! use rf_routing::{ResourceRouter, ControllerAction};
//!
//! let posts = ResourceRouter::new("posts")
//!     .only(vec![ControllerAction::Index, ControllerAction::Show]);
//!
//! // Generates routes for: index, show
//! ```
//!
//! ## Middleware Pipeline
//!
//! ```rust,no_run
//! use rf_routing::{register_middleware, pipeline};
//! use axum::{extract::Request, middleware::Next};
//! use futures::future::BoxFuture;
//!
//! register_middleware("auth", |req: Request, next: Next| {
//!     Box::pin(async move {
//!         // Auth logic
//!         Ok(next.run(req).await)
//!     })
//! });
//!
//! let pipe = pipeline()
//!     .push("auth")
//!     .push("throttle");
//! ```

pub mod named_routes;
pub mod signed_urls;
pub mod url_generation;
pub mod groups;
pub mod middleware_pipeline;
pub mod middleware_stack;
pub mod route;
pub mod controller;
pub mod resource;
pub mod model_binding;
pub mod versioning;
pub mod versioned_router;

#[macro_use]
pub mod macros;

pub use named_routes::{NamedRoute, ParamValue, RouteRegistry, RouteUrlBuilder};
pub use signed_urls::{SignedUrl, SignedUrlBuilder, parse_signed_url};
pub use url_generation::{UrlGenerator, QueryStringBuilder, UrlBuilder};
pub use groups::{RouteGroup, RouteGroupBuilder, RouteGroupRegistry};
pub use middleware_pipeline::{
    MiddlewareRegistry, MiddlewarePipeline, MiddlewareGroup, MiddlewareGroupRegistry,
    global_registry, register_middleware, pipeline,
};
pub use middleware_stack::{MiddlewareStack, MiddlewareStackBuilder};
pub use route::{Route, RouteBuilder, HttpMethod};
pub use controller::{Controller, ControllerAction, ControllerRegistry, ControllerRouteBuilder};
pub use resource::{ResourceRouter, ResourceCollection, api_resource, resource_only, resource_except};
pub use model_binding::{Bindable, ModelBinding, ModelBindingRegistry, ModelBindingError};
pub use versioning::{
    ApiVersion, VersionConfig, VersionError, VersionNegotiator, DefaultNegotiator,
    extract_from_accept, extract_from_header, extract_from_path,
};
pub use versioned_router::VersionedRouterBuilder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_named_routes() {
        let mut registry = RouteRegistry::new();

        let route1 = NamedRoute::new("users.index", "/users");
        let route2 = NamedRoute::new("users.show", "/users/{id}");
        let route3 = NamedRoute::new("posts.show", "/posts/{id}/comments/{comment_id}");

        registry.register(route1);
        registry.register(route2);
        registry.register(route3);

        // Simple route
        let url = registry.url("users.index", &std::collections::HashMap::new());
        assert_eq!(url, Some("/users".to_string()));

        // Route with single parameter
        let params = route_params! {
            "id" => 123
        };
        let url = registry.url("users.show", &params);
        assert_eq!(url, Some("/users/123".to_string()));

        // Route with multiple parameters
        let params = route_params! {
            "id" => 456,
            "comment_id" => 789
        };
        let url = registry.url("posts.show", &params);
        assert_eq!(url, Some("/posts/456/comments/789".to_string()));
    }

    #[test]
    fn test_integration_signed_urls() {
        const SECRET: &str = "test-secret-key";

        // Create signed URL
        let signed = SignedUrlBuilder::new("/users/123", SECRET)
            .expires_in_hours(1)
            .build();

        assert!(!signed.is_expired());
        assert!(signed.verify(SECRET));

        // Verify URL string format
        let url_string = signed.to_string();
        assert!(url_string.contains("signature="));
        assert!(url_string.contains("expires="));

        // Parse and verify
        let parsed = parse_signed_url(&url_string, SECRET);
        assert!(parsed.is_some());
        assert!(parsed.unwrap().verify(SECRET));
    }

    #[test]
    fn test_integration_url_generation() {
        let mut generator = UrlGenerator::new("https://example.com", "secret");

        let route = NamedRoute::new("api.users.show", "/api/users/{id}");
        generator.register(route);

        // Generate regular URL
        let params = route_params! {
            "id" => 123
        };
        let url = generator.route("api.users.show", params.clone());
        assert_eq!(url, Some("/api/users/123".to_string()));

        // Generate signed URL
        let signed = generator.signed_route("api.users.show", params, Some(60));
        assert!(signed.is_some());

        let signed = signed.unwrap();
        assert!(signed.to_string().contains("https://example.com/api/users/123"));
    }
}
