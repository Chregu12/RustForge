//! Route facade providing Laravel-style static routing API.
//!
//! This module provides the main `Route` struct with static-like methods
//! for defining routes in a Laravel-style syntax.

use crate::facade::builder::FacadeRouteBuilder;
use crate::facade::group::GroupBuilder;
use crate::HttpMethod;

/// The Route facade providing a static-like API for defining routes.
///
/// This is the main entry point for defining routes in your application.
/// It provides methods like `get()`, `post()`, `put()`, etc. that return
/// a builder for further configuration.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_routing::RouteFacade as Route;
///
/// // Simple GET route
/// Route::get("/users", "UserController@index");
///
/// // POST route with middleware and name
/// Route::post("/users", "UserController@store")
///     .name("users.store")
///     .middleware("auth");
///
/// // Route with multiple middleware
/// Route::put("/users/:id", "UserController@update")
///     .name("users.update")
///     .middleware("auth")
///     .middleware("validate");
/// ```
pub struct Route;

impl Route {
    /// Register a GET route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::get("/users", "UserController@index")
    ///     .name("users.index");
    /// ```
    pub fn get(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Get])
    }

    /// Register a POST route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::post("/users", "UserController@store")
    ///     .name("users.store")
    ///     .middleware("auth");
    /// ```
    pub fn post(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Post])
    }

    /// Register a PUT route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::put("/users/:id", "UserController@update")
    ///     .name("users.update");
    /// ```
    pub fn put(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Put])
    }

    /// Register a PATCH route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::patch("/users/:id", "UserController@patch")
    ///     .name("users.patch");
    /// ```
    pub fn patch(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Patch])
    }

    /// Register a DELETE route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::delete("/users/:id", "UserController@destroy")
    ///     .name("users.destroy")
    ///     .middleware("auth");
    /// ```
    pub fn delete(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Delete])
    }

    /// Register an OPTIONS route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::options("/users", "OptionsController@handle");
    /// ```
    pub fn options(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Options])
    }

    /// Register a route that responds to multiple HTTP methods.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    /// use rf_routing::HttpMethod;
    ///
    /// Route::match_methods(
    ///     vec![HttpMethod::Get, HttpMethod::Post],
    ///     "/users",
    ///     "UserController@handle"
    /// );
    /// ```
    pub fn match_methods(
        methods: Vec<HttpMethod>,
        path: impl Into<String>,
        _handler: impl Into<String>,
    ) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, methods)
    }

    /// Register a route that responds to any HTTP method.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::any("/fallback", "FallbackController@handle");
    /// ```
    pub fn any(path: impl Into<String>, _handler: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(
            path,
            vec![
                HttpMethod::Get,
                HttpMethod::Post,
                HttpMethod::Put,
                HttpMethod::Patch,
                HttpMethod::Delete,
                HttpMethod::Options,
            ],
        )
    }

    /// Create a route group for organizing routes with shared configuration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::group()
    ///     .prefix("/api")
    ///     .middleware("auth")
    ///     .routes(|group| {
    ///         group.get("/users", "UserController@index");
    ///         group.post("/users", "UserController@store");
    ///     });
    /// ```
    pub fn group() -> GroupBuilder {
        GroupBuilder::new()
    }

    /// Register RESTful resource routes for a controller.
    ///
    /// This generates the standard RESTful routes:
    /// - GET /resource - index
    /// - GET /resource/create - create
    /// - POST /resource - store
    /// - GET /resource/:id - show
    /// - GET /resource/:id/edit - edit
    /// - PUT/PATCH /resource/:id - update
    /// - DELETE /resource/:id - destroy
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::resource("posts", "PostController");
    /// ```
    pub fn resource(resource: impl Into<String>, _controller: impl Into<String>) {
        let resource = resource.into();

        // Index
        Self::get(format!("/{}", resource), "index".to_string())
            .name(format!("{}.index", resource));

        // Create
        Self::get(format!("/{}/create", resource), "create".to_string())
            .name(format!("{}.create", resource));

        // Store
        Self::post(format!("/{}", resource), "store".to_string())
            .name(format!("{}.store", resource));

        // Show
        Self::get(format!("/{}/:id", resource), "show".to_string())
            .name(format!("{}.show", resource));

        // Edit
        Self::get(format!("/{}/:id/edit", resource), "edit".to_string())
            .name(format!("{}.edit", resource));

        // Update
        Self::put(format!("/{}/:id", resource), "update".to_string())
            .name(format!("{}.update", resource));
        Self::patch(format!("/{}/:id", resource), "update".to_string())
            .name(format!("{}.update.patch", resource));

        // Destroy
        Self::delete(format!("/{}/:id", resource), "destroy".to_string())
            .name(format!("{}.destroy", resource));
    }

    /// Register API resource routes (without create/edit).
    ///
    /// This generates routes for API usage (no HTML forms):
    /// - GET /resource - index
    /// - POST /resource - store
    /// - GET /resource/:id - show
    /// - PUT/PATCH /resource/:id - update
    /// - DELETE /resource/:id - destroy
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::api_resource("posts", "PostController");
    /// ```
    pub fn api_resource(resource: impl Into<String>, _controller: impl Into<String>) {
        let resource = resource.into();

        // Index
        Self::get(format!("/{}", resource), "index".to_string())
            .name(format!("{}.index", resource));

        // Store
        Self::post(format!("/{}", resource), "store".to_string())
            .name(format!("{}.store", resource));

        // Show
        Self::get(format!("/{}/:id", resource), "show".to_string())
            .name(format!("{}.show", resource));

        // Update
        Self::put(format!("/{}/:id", resource), "update".to_string())
            .name(format!("{}.update", resource));
        Self::patch(format!("/{}/:id", resource), "update".to_string())
            .name(format!("{}.update.patch", resource));

        // Destroy
        Self::delete(format!("/{}/:id", resource), "destroy".to_string())
            .name(format!("{}.destroy", resource));
    }

    /// Register a redirect route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::redirect("/old-path", "/new-path");
    /// ```
    pub fn redirect(from: impl Into<String>, to: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(from, vec![HttpMethod::Get])
            .metadata("redirect", to.into())
    }

    /// Register a permanent redirect route (301).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::permanent_redirect("/old-path", "/new-path");
    /// ```
    pub fn permanent_redirect(
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(from, vec![HttpMethod::Get])
            .metadata("redirect", to.into())
            .metadata("status", "301")
    }

    /// Register a view route (renders a view without a controller).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::view("/about", "about");
    /// ```
    pub fn view(path: impl Into<String>, view: impl Into<String>) -> FacadeRouteBuilder {
        FacadeRouteBuilder::new(path, vec![HttpMethod::Get]).metadata("view", view.into())
    }

    /// Start a middleware group without closures.
    ///
    /// This is an alternative syntax that avoids the `||` closure syntax
    /// which is difficult to type on German keyboards.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// // Instead of: Route::middleware(&["auth"]).group(|| { ... });
    /// // Use this:
    /// Route::middleware(&["auth"])
    ///     .add(Route::post("/posts", "PostController@store"))
    ///     .add(Route::put("/posts/:id", "PostController@update"))
    ///     .add(Route::delete("/posts/:id", "PostController@destroy"));
    /// ```
    pub fn middleware(middleware: &[&str]) -> MiddlewareGroupBuilder {
        MiddlewareGroupBuilder::new(middleware)
    }

    /// Create a protected route group (shortcut for auth middleware).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::protected()
    ///     .add(Route::post("/posts", "PostController@store"))
    ///     .add(Route::delete("/posts/:id", "PostController@destroy"));
    /// ```
    pub fn protected() -> MiddlewareGroupBuilder {
        MiddlewareGroupBuilder::new(&["auth"])
    }

    /// Create an API route group (with api middleware and /api prefix).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::api()
    ///     .add(Route::get("/users", "UserController@index"))
    ///     .add(Route::post("/users", "UserController@store"));
    /// // Routes will be: /api/users
    /// ```
    pub fn api() -> MiddlewareGroupBuilder {
        MiddlewareGroupBuilder::new(&["api"]).with_prefix("/api")
    }
}

/// Builder for middleware groups without closure syntax.
///
/// This provides an alternative to the closure-based `group()` method,
/// avoiding the `||` syntax which is difficult on German keyboards.
pub struct MiddlewareGroupBuilder {
    middleware: Vec<String>,
    prefix: Option<String>,
    routes: Vec<FacadeRouteBuilder>,
}

impl MiddlewareGroupBuilder {
    /// Create a new middleware group builder.
    pub fn new(middleware: &[&str]) -> Self {
        Self {
            middleware: middleware.iter().map(|s| s.to_string()).collect(),
            prefix: None,
            routes: Vec::new(),
        }
    }

    /// Set a prefix for all routes in this group.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Add a route to this middleware group.
    ///
    /// The middleware will be automatically applied to the route.
    pub fn add(mut self, mut route: FacadeRouteBuilder) -> Self {
        // Apply prefix if set
        if let Some(ref prefix) = self.prefix {
            route = route.with_prefix(prefix);
        }

        // Apply middleware
        for mw in &self.middleware {
            route = route.middleware(mw);
        }

        // Route will auto-register on drop, just push to track
        self.routes.push(route);
        self
    }

    /// Add multiple routes at once.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteFacade as Route;
    ///
    /// Route::middleware(&["auth"]).add_all(vec![
    ///     Route::post("/posts", "PostController@store"),
    ///     Route::put("/posts/:id", "PostController@update"),
    /// ]);
    /// ```
    pub fn add_all(mut self, routes: Vec<FacadeRouteBuilder>) -> Self {
        for route in routes {
            self = self.add(route);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facade::registry::global_router;

    #[test]
    fn test_route_get() {
        let builder = Route::get("/users", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.uri, "/users");
        assert_eq!(route.methods.len(), 1);
        assert_eq!(route.methods[0], HttpMethod::Get);
    }

    #[test]
    fn test_route_post() {
        let builder = Route::post("/users", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.uri, "/users");
        assert_eq!(route.methods[0], HttpMethod::Post);
    }

    #[test]
    fn test_route_put() {
        let builder = Route::put("/users/:id", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.methods[0], HttpMethod::Put);
    }

    #[test]
    fn test_route_patch() {
        let builder = Route::patch("/users/:id", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.methods[0], HttpMethod::Patch);
    }

    #[test]
    fn test_route_delete() {
        let builder = Route::delete("/users/:id", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.methods[0], HttpMethod::Delete);
    }

    #[test]
    fn test_route_options() {
        let builder = Route::options("/users", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.methods[0], HttpMethod::Options);
    }

    #[test]
    fn test_route_match_methods() {
        let builder = Route::match_methods(
            vec![HttpMethod::Get, HttpMethod::Post],
            "/users",
            "handler".to_string(),
        );
        let route = builder.into_route();

        assert_eq!(route.methods.len(), 2);
        assert!(route.methods.contains(&HttpMethod::Get));
        assert!(route.methods.contains(&HttpMethod::Post));
    }

    #[test]
    fn test_route_any() {
        let builder = Route::any("/fallback", "handler".to_string());
        let route = builder.into_route();

        assert_eq!(route.methods.len(), 6);
    }

    #[test]
    fn test_route_with_name() {
        let builder = Route::get("/users", "handler".to_string()).name("users.index");
        let route = builder.into_route();

        assert_eq!(route.name, Some("users.index".to_string()));
    }

    #[test]
    fn test_route_with_middleware() {
        let builder = Route::get("/users", "handler".to_string())
            .middleware("auth")
            .middleware("throttle");
        let route = builder.into_route();

        assert_eq!(route.middleware.len(), 2);
    }

    #[test]
    fn test_route_redirect() {
        let builder = Route::redirect("/old", "/new");
        let route = builder.into_route();

        assert_eq!(route.metadata("redirect"), Some(&"/new".to_string()));
    }

    #[test]
    fn test_route_permanent_redirect() {
        let builder = Route::permanent_redirect("/old", "/new");
        let route = builder.into_route();

        assert_eq!(route.metadata("redirect"), Some(&"/new".to_string()));
        assert_eq!(route.metadata("status"), Some(&"301".to_string()));
    }

    #[test]
    fn test_route_view() {
        let builder = Route::view("/about", "about");
        let route = builder.into_route();

        assert_eq!(route.metadata("view"), Some(&"about".to_string()));
    }

    #[test]
    fn test_route_resource() {
        // Clear any existing routes
        global_router().clear();

        Route::resource("posts", "PostController");

        let routes = global_router().routes();

        // Should have 8 routes (index, create, store, show, edit, update, patch update, destroy)
        assert!(routes.len() >= 8);

        // Check some route names
        let names: Vec<Option<String>> = routes.iter().map(|r| r.name.clone()).collect();
        assert!(names.contains(&Some("posts.index".to_string())));
        assert!(names.contains(&Some("posts.store".to_string())));
        assert!(names.contains(&Some("posts.show".to_string())));
    }

    #[test]
    fn test_route_api_resource() {
        global_router().clear();

        Route::api_resource("users", "UserController");

        let routes = global_router().routes();

        // Should have 6 routes (index, store, show, update, patch update, destroy)
        // No create or edit routes
        assert!(routes.len() >= 6);

        let names: Vec<Option<String>> = routes.iter().map(|r| r.name.clone()).collect();
        assert!(names.contains(&Some("users.index".to_string())));
        assert!(!names.contains(&Some("users.create".to_string())));
        assert!(!names.contains(&Some("users.edit".to_string())));
    }
}
