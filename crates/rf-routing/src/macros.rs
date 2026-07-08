//! Convenient macros for route definition.
//!
//! This module provides Laravel-like macros for:
//! - Route definition
//! - Route groups
//! - Resource routing
//!
//! Note: The `route_params!` macro is defined in the url_generation module.

/// Create routes with a fluent syntax.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::routes;
/// use axum::routing::{get, post};
///
/// let router = routes! {
///     GET "/" => home_handler,
///     GET "/users" => users_index,
///     POST "/users" => users_store,
///     GET "/users/{id}" => users_show,
/// };
/// ```
#[macro_export]
macro_rules! routes {
    ($($method:tt $path:expr => $handler:expr),* $(,)?) => {{
        let mut router = ::axum::Router::new();
        $(
            router = router.route($path, $crate::__route_method!($method, $handler));
        )*
        router
    }};
}

/// Helper macro for route methods (internal use).
#[macro_export]
#[doc(hidden)]
macro_rules! __route_method {
    (GET, $handler:expr) => {
        ::axum::routing::get($handler)
    };
    (POST, $handler:expr) => {
        ::axum::routing::post($handler)
    };
    (PUT, $handler:expr) => {
        ::axum::routing::put($handler)
    };
    (PATCH, $handler:expr) => {
        ::axum::routing::patch($handler)
    };
    (DELETE, $handler:expr) => {
        ::axum::routing::delete($handler)
    };
    (HEAD, $handler:expr) => {
        ::axum::routing::head($handler)
    };
    (OPTIONS, $handler:expr) => {
        ::axum::routing::options($handler)
    };
}

/// Create a route group with shared configuration.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::group;
///
/// let api_routes = group! {
///     prefix: "/api",
///     middleware: ["auth", "throttle"],
///     name: "api.",
///     routes: {
///         GET "/users" => api_users,
///         GET "/posts" => api_posts,
///     }
/// };
/// ```
#[macro_export]
macro_rules! group {
    (
        $(prefix: $prefix:expr,)?
        $(middleware: [$($mw:expr),* $(,)?],)?
        $(name: $name:expr,)?
        $(domain: $domain:expr,)?
        routes: { $($routes:tt)* }
    ) => {{
        let mut group = $crate::RouteGroup::new();

        $(
            group = group.prefix($prefix);
        )?

        $(
            $(
                group = group.middleware($mw);
            )*
        )?

        $(
            group = group.name($name);
        )?

        $(
            group = group.domain($domain);
        )?

        group
    }};
}

/// Create a nested route group.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::nested_group;
///
/// let routes = nested_group! {
///     parent: {
///         prefix: "/api",
///         middleware: ["auth"],
///     },
///     child: {
///         prefix: "/v1",
///         middleware: ["throttle"],
///     }
/// };
/// ```
#[macro_export]
macro_rules! nested_group {
    (
        parent: {
            $(prefix: $parent_prefix:expr,)?
            $(middleware: [$($parent_mw:expr),* $(,)?],)?
            $(name: $parent_name:expr,)?
        },
        child: {
            $(prefix: $child_prefix:expr,)?
            $(middleware: [$($child_mw:expr),* $(,)?],)?
            $(name: $child_name:expr,)?
        }
    ) => {{
        let mut parent = $crate::RouteGroup::new();
        $(
            parent = parent.prefix($parent_prefix);
        )?
        $(
            $(
                parent = parent.middleware($parent_mw);
            )*
        )?
        $(
            parent = parent.name($parent_name);
        )?

        let mut child = $crate::RouteGroup::new();
        $(
            child = child.prefix($child_prefix);
        )?
        $(
            $(
                child = child.middleware($child_mw);
            )*
        )?
        $(
            child = child.name($child_name);
        )?

        parent.nest(child)
    }};
}

/// Define a resource route.
///
/// This macro has two families of forms:
///
/// ## 1. Metadata builder (returns a [`ResourceRouter`](crate::ResourceRouter))
///
/// ```rust,ignore
/// use rf_routing::resource;
///
/// let posts = resource!("posts");
/// let users = resource!("users", only: [Index, Show]);
/// let comments = resource!("comments", except: [Destroy]);
/// let api_posts = resource!("posts", api: true);
/// ```
///
/// ## 2. RESTful handler registration (wires a controller to the global router)
///
/// Given a controller type (e.g. one produced by `controller_block!`) whose
/// associated functions are async, argument-less handlers, this maps the
/// standard RESTful routes onto the corresponding handler in one call, reusing
/// the real [`get`](crate::get)/[`post`](crate::post)/[`put`](crate::put)/
/// [`patch`](crate::patch)/[`delete`](crate::delete) registration on the global
/// router:
///
/// | Action    | Route                      |
/// |-----------|----------------------------|
/// | `index`   | `GET    {prefix}`          |
/// | `create`  | `GET    {prefix}/create`   |
/// | `store`   | `POST   {prefix}`          |
/// | `show`    | `GET    {prefix}/{id}`      |
/// | `edit`    | `GET    {prefix}/{id}/edit` |
/// | `update`  | `PUT`+`PATCH {prefix}/{id}` |
/// | `destroy` | `DELETE {prefix}/{id}`      |
///
/// ```rust,ignore
/// use rf_routing::resource;
///
/// // Register every RESTful action (all five must be defined on the controller):
/// resource!("/posts", PostController);
///
/// // Or register only the actions the controller actually defines:
/// resource!("/posts", PostController { index, show, store });
/// ```
///
/// Build the served router afterwards with
/// `rf_routing::global_router().build_router()`.
#[macro_export]
macro_rules! resource {
    // --- internal per-action registration arms -----------------------------
    // (the leading `@` token makes these unambiguous vs. the public forms)
    (@action $prefix:expr, $controller:path, index) => {
        $crate::get($prefix, <$controller>::index);
    };
    (@action $prefix:expr, $controller:path, create) => {
        $crate::get(::std::format!("{}/create", $prefix), <$controller>::create);
    };
    (@action $prefix:expr, $controller:path, store) => {
        $crate::post($prefix, <$controller>::store);
    };
    (@action $prefix:expr, $controller:path, show) => {
        $crate::get(::std::format!("{}/{{id}}", $prefix), <$controller>::show);
    };
    (@action $prefix:expr, $controller:path, edit) => {
        $crate::get(::std::format!("{}/{{id}}/edit", $prefix), <$controller>::edit);
    };
    (@action $prefix:expr, $controller:path, update) => {
        $crate::put(::std::format!("{}/{{id}}", $prefix), <$controller>::update);
        $crate::patch(::std::format!("{}/{{id}}", $prefix), <$controller>::update);
    };
    (@action $prefix:expr, $controller:path, destroy) => {
        $crate::delete(::std::format!("{}/{{id}}", $prefix), <$controller>::destroy);
    };

    // --- metadata builder forms (return a `ResourceRouter`) -----------------
    ($name:expr, only: [$($action:ident),* $(,)?]) => {
        $crate::ResourceRouter::new($name)
            .only(vec![$($crate::ControllerAction::$action),*])
    };

    ($name:expr, except: [$($action:ident),* $(,)?]) => {
        $crate::ResourceRouter::new($name)
            .except(vec![$($crate::ControllerAction::$action),*])
    };

    ($name:expr, api: true) => {
        $crate::ResourceRouter::new($name).api_resource()
    };

    ($name:expr, shallow: true) => {
        $crate::ResourceRouter::new($name).shallow()
    };

    // --- RESTful handler registration forms ---------------------------------
    // Explicit subset: only register the listed actions.
    ($prefix:expr, $controller:path { $($action:ident),+ $(,)? }) => {
        $(
            $crate::resource!(@action $prefix, $controller, $action);
        )+
    };

    // Full RESTful set: index, show, store, update, destroy (all must exist).
    ($prefix:expr, $controller:path) => {
        $crate::resource!($prefix, $controller { index, show, store, update, destroy });
    };

    // Metadata builder: bare name only.
    ($name:expr) => {
        $crate::ResourceRouter::new($name)
    };
}

/// Define multiple resources at once.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::resources;
///
/// let collection = resources! {
///     "posts" => {},
///     "users" => { only: [Index, Show] },
///     "comments" => { api: true }
/// };
/// ```
#[macro_export]
macro_rules! resources {
    ($($name:expr => { $($config:tt)* }),* $(,)?) => {{
        let mut collection = $crate::ResourceCollection::new();
        $(
            collection = collection.add($crate::resource!($name $(, $($config)*)?));
        )*
        collection
    }};
}

/// Register middleware globally.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::middleware;
///
/// middleware! {
///     "auth" => auth_handler,
///     "throttle" => throttle_handler,
///     "cors" => cors_handler,
/// }
/// ```
#[macro_export]
macro_rules! middleware {
    ($($name:expr => $handler:expr),* $(,)?) => {{
        $(
            $crate::register_middleware($name, $handler);
        )*
    }};
}

/// Create a middleware group.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::middleware_group;
///
/// let web = middleware_group! {
///     "web" => ["session", "csrf", "errors"]
/// };
///
/// let api = middleware_group! {
///     "api" => ["auth:api", "throttle:60,1"]
/// };
/// ```
#[macro_export]
macro_rules! middleware_group {
    ($name:expr => [$($mw:expr),* $(,)?]) => {{
        let mut group = $crate::MiddlewareGroup::new($name);
        $(
            group = group.add($mw);
        )*
        group
    }};
}

#[cfg(test)]
mod tests {
    // Note: Tests for route_params! are in url_generation module.
    //
    // The RESTful handler-registration forms of `resource!` (e.g.
    // `resource!("/posts", PostController { index, show, store })`) are proven
    // end-to-end — real requests served through `build_router()` — by the
    // `resource_routing` sandbox probe, since they depend on the global router
    // singleton and async serving.
    use crate::ControllerAction;

    // Regression guard: the pre-existing metadata builder forms must keep working
    // now that the RESTful handler-registration arms have been added.
    #[test]
    fn resource_metadata_forms_still_build() {
        let posts = resource!("posts");
        assert_eq!(posts.name(), "posts");

        let only = resource!("posts", only: [Index, Show]);
        assert!(only.should_include(&ControllerAction::Index));
        assert!(only.should_include(&ControllerAction::Show));
        assert!(!only.should_include(&ControllerAction::Store));

        let except = resource!("posts", except: [Destroy]);
        assert!(!except.should_include(&ControllerAction::Destroy));

        let api = resource!("posts", api: true);
        assert!(api.is_api_only());

        let shallow = resource!("comments", shallow: true);
        assert!(shallow.is_shallow());
    }
}
