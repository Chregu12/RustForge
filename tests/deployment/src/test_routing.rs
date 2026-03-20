//! Deployment tests for rf-routing

#[cfg(test)]
mod tests {
    use rf_routing::route::RouteBuilder;
    use rf_routing::groups::{RouteGroup, RouteGroupBuilder};
    use rf_routing::named_routes::{NamedRoute, RouteRegistry, RouteUrlBuilder, ParamValue};
    use rf_routing::resource::{ResourceRouter, ResourceCollection, api_resource};
    use rf_routing::controller::ControllerAction;
    use rf_routing::signed_urls::{SignedUrl, SignedUrlBuilder};
    use rf_routing::url_generation::{UrlGenerator, UrlBuilder, QueryStringBuilder};
    use rf_routing::middleware_pipeline::{MiddlewareRegistry, MiddlewarePipeline, MiddlewareGroup};
    use rf_routing::middleware_stack::MiddlewareStackBuilder;
    use rf_routing::versioning::{ApiVersion as RoutingApiVersion, VersionConfig, DefaultNegotiator, VersionNegotiator};
    use std::collections::HashMap;
    use std::sync::Arc;

    // ── Route Builder ────────────────────────────────────────────

    #[test]
    fn route_builder_get() {
        let route = RouteBuilder::get("/users")
            .name("users.index")
            .add_middleware("auth")
            .build();
        assert!(route.has_middleware("auth"));
    }

    #[test]
    fn route_builder_post() {
        let route = RouteBuilder::post("/users")
            .name("users.store")
            .build();
        let _ = route;
    }

    #[test]
    fn route_builder_all_methods() {
        let _ = RouteBuilder::put("/users/{id}").build();
        let _ = RouteBuilder::patch("/users/{id}").build();
        let _ = RouteBuilder::delete("/users/{id}").build();
    }

    #[test]
    fn route_metadata() {
        let route = RouteBuilder::get("/api")
            .metadata("rate_limit", "100")
            .build();
        assert_eq!(route.metadata("rate_limit"), Some(&"100".to_string()));
        assert!(route.metadata("nonexistent").is_none());
    }

    // ── Route Groups ─────────────────────────────────────────────

    #[test]
    fn route_group_builder() {
        let group = RouteGroupBuilder::new()
            .prefix("/api/v1")
            .middleware("auth")
            .middleware("throttle")
            .name("api.")
            .domain("api.example.com")
            .build();

        assert_eq!(group.get_prefix(), Some("/api/v1"));
        assert_eq!(group.get_middleware().len(), 2);
        assert_eq!(group.get_name(), Some("api."));
        assert_eq!(group.get_domain(), Some("api.example.com"));
    }

    #[test]
    fn route_group_nesting() {
        let parent = RouteGroup::new().prefix("/api").middleware("auth");
        let child = RouteGroup::new().prefix("/v1").middleware("throttle");
        let nested = parent.nest(child);
        assert_eq!(nested.get_prefix(), Some("/api/v1"));
        assert_eq!(nested.get_middleware().len(), 2);
    }

    // ── Named Routes ─────────────────────────────────────────────

    #[test]
    fn named_route_creation() {
        let route = NamedRoute::new("users.show", "/users/{id}");
        assert_eq!(route.name(), "users.show");
        assert_eq!(route.pattern(), "/users/{id}");
    }

    #[test]
    fn named_route_url_generation() {
        let route = NamedRoute::new("users.show", "/users/{id}");
        let mut params = HashMap::new();
        params.insert("id".into(), ParamValue::Number(42));
        let url = route.url(&params);
        assert_eq!(url, "/users/42");
    }

    #[test]
    fn route_registry() {
        let mut registry = RouteRegistry::new();
        registry.register(NamedRoute::new("home", "/"));
        registry.register(NamedRoute::new("users.index", "/users"));

        assert!(registry.has("home"));
        assert!(registry.has("users.index"));
        assert!(!registry.has("nonexistent"));

        let names = registry.names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn route_url_builder() {
        let route = NamedRoute::new("post", "/posts/{id}/comments/{comment_id}");
        let url = RouteUrlBuilder::new(route)
            .param("id", ParamValue::Number(1))
            .param("comment_id", ParamValue::Number(5))
            .build();
        assert_eq!(url, "/posts/1/comments/5");
    }

    // ── Resource Routes ──────────────────────────────────────────

    #[test]
    fn resource_router() {
        let resource = ResourceRouter::new("posts");
        assert_eq!(resource.name(), "posts");
        assert!(!resource.actions().is_empty());
    }

    #[test]
    fn api_resource_excludes_html() {
        let resource = api_resource("posts");
        assert!(resource.is_api_only());
        let actions = resource.actions();
        assert!(!actions.contains(&ControllerAction::Create));
        assert!(!actions.contains(&ControllerAction::Edit));
    }

    #[test]
    fn resource_only_except() {
        let resource = ResourceRouter::new("posts")
            .only(vec![ControllerAction::Index, ControllerAction::Show]);
        assert_eq!(resource.actions().len(), 2);

        let resource2 = ResourceRouter::new("posts")
            .except(vec![ControllerAction::Destroy]);
        assert!(!resource2.actions().contains(&ControllerAction::Destroy));
    }

    #[test]
    fn resource_nested() {
        let resource = ResourceRouter::new("posts")
            .nest(ResourceRouter::new("comments"));
        assert_eq!(resource.nested_resources().len(), 1);
    }

    #[test]
    fn resource_collection() {
        let collection = ResourceCollection::new()
            .add(ResourceRouter::new("posts"))
            .add(ResourceRouter::new("comments"));
        assert_eq!(collection.resources().len(), 2);
        assert!(collection.find("posts").is_some());
        assert!(collection.find("tags").is_none());
    }

    #[test]
    fn resource_paths() {
        let resource = ResourceRouter::new("posts");
        let paths = resource.paths(None);
        assert!(!paths.is_empty());
    }

    // ── Controller Actions ───────────────────────────────────────

    #[test]
    fn controller_action_methods() {
        assert_eq!(ControllerAction::Index.method(), "GET");
        assert_eq!(ControllerAction::Store.method(), "POST");
        assert_eq!(ControllerAction::Update.method(), "PUT");
        assert_eq!(ControllerAction::Destroy.method(), "DELETE");
    }

    #[test]
    fn controller_action_all() {
        let all = ControllerAction::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn controller_action_from_str() {
        assert_eq!(ControllerAction::from_str("index"), Some(ControllerAction::Index));
        assert_eq!(ControllerAction::from_str("store"), Some(ControllerAction::Store));
        assert!(ControllerAction::from_str("invalid").is_none());
    }

    // ── Signed URLs ──────────────────────────────────────────────

    #[test]
    fn signed_url_creation_and_verification() {
        let signed = SignedUrl::new("https://example.com/download/123", "my-secret", None);
        assert!(signed.verify("my-secret"));
        assert!(!signed.verify("wrong-secret"));
        assert!(!signed.is_expired());
    }

    #[test]
    fn signed_url_with_expiry() {
        use chrono::Utc;
        let expires = Utc::now() + chrono::Duration::hours(1);
        let signed = SignedUrl::new("https://example.com/file", "secret", Some(expires));
        assert!(!signed.is_expired());
        assert!(signed.verify("secret"));
    }

    #[test]
    fn signed_url_builder() {
        let signed = SignedUrlBuilder::new("https://example.com/share", "secret")
            .expires_in_hours(24)
            .build();
        assert!(signed.verify("secret"));
        assert!(!signed.is_expired());
    }

    // ── URL Generation ───────────────────────────────────────────

    #[test]
    fn url_builder() {
        let url = UrlBuilder::new("https://example.com")
            .segment("api")
            .segment("users")
            .query("page", "1")
            .query("per_page", "10")
            .fragment("results")
            .build();
        assert!(url.contains("/api/users"));
        assert!(url.contains("page=1"));
        assert!(url.contains("#results"));
    }

    #[test]
    fn query_string_builder() {
        let qs = QueryStringBuilder::new()
            .add("search", "rust")
            .add("page", "1")
            .build();
        assert!(qs.contains("search=rust"));
        assert!(qs.contains("page=1"));
    }

    #[test]
    fn url_generator() {
        let mut gen = UrlGenerator::new("https://myapp.com", "signing-secret");
        gen.register(NamedRoute::new("users.show", "/users/{id}"));

        let url = gen.route("users.show", {
            let mut p = HashMap::new();
            p.insert("id".into(), ParamValue::Number(42));
            p
        });
        assert!(url.is_some());
        assert!(url.unwrap().contains("/users/42"));
    }

    // ── Middleware Pipeline ───────────────────────────────────────

    #[test]
    fn middleware_registry() {
        let registry = MiddlewareRegistry::new();
        registry.register("auth", |req, next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });
        assert!(registry.has("auth"));
        assert!(!registry.has("nonexistent"));
        assert!(registry.names().contains(&"auth".to_string()));
    }

    #[test]
    fn middleware_pipeline_building() {
        let registry = Arc::new(MiddlewareRegistry::new());
        registry.register("auth", |req, next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });
        registry.register("throttle", |req, next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });

        let pipeline = MiddlewarePipeline::new(registry)
            .push("auth")
            .push("throttle");
        assert_eq!(pipeline.len(), 2);
        assert!(!pipeline.is_empty());
    }

    #[test]
    fn middleware_group() {
        let group = MiddlewareGroup::new("web")
            .add("csrf")
            .add("session");
        assert_eq!(group.name(), "web");
        assert_eq!(group.middleware().len(), 2);
    }

    // ── Middleware Stack ──────────────────────────────────────────

    #[test]
    fn middleware_stack_builder() {
        let stack = MiddlewareStackBuilder::new()
            .global("request_id")
            .group("web", vec!["csrf".into(), "session".into()])
            .group("api", vec!["auth".into(), "throttle".into()])
            .route("admin.users", vec!["admin".into()])
            .build();

        assert_eq!(stack.global().len(), 1);
        assert!(stack.group("web").is_some());
        assert!(stack.group("api").is_some());
        assert!(stack.route("admin.users").is_some());
    }

    #[test]
    fn middleware_stack_resolve() {
        let stack = MiddlewareStackBuilder::new()
            .global("request_id")
            .group("web", vec!["csrf".into(), "session".into()])
            .route("users.index", vec!["cache".into()])
            .build();

        let resolved = stack.resolve("users.index", &["web".into()]);
        assert!(resolved.contains(&"request_id".to_string()));
        assert!(resolved.contains(&"csrf".to_string()));
        assert!(resolved.contains(&"cache".to_string()));
    }

    // ── API Versioning (routing crate) ───────────────────────────

    #[test]
    fn routing_api_version() {
        let v = RoutingApiVersion::new(2);
        assert_eq!(v.version(), 2);
        assert!(v.is(2));
        assert!(!v.is(1));
        assert!(v.at_least(1));
        assert!(v.at_least(2));
        assert!(!v.at_least(3));
    }

    #[test]
    fn version_negotiator() {
        let config = VersionConfig {
            default_version: 2,
            supported_versions: vec![1, 2, 3],
            deprecated_versions: vec![1],
        };
        let negotiator = DefaultNegotiator::new(config);
        assert!(negotiator.is_supported(2));
        assert!(negotiator.is_deprecated(1));
        assert!(!negotiator.is_supported(99));
        assert_eq!(negotiator.negotiate(None).unwrap(), 2);
        assert_eq!(negotiator.negotiate(Some(3)).unwrap(), 3);
    }
}
