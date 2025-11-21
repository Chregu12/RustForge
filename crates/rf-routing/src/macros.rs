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
///     GET "/users/:id" => users_show,
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
/// # Example
///
/// ```rust,ignore
/// use rf_routing::resource;
///
/// let posts = resource!("posts");
/// let users = resource!("users", only: [Index, Show]);
/// let comments = resource!("comments", except: [Destroy]);
/// let api_posts = resource!("posts", api: true);
/// ```
#[macro_export]
macro_rules! resource {
    ($name:expr) => {
        $crate::ResourceRouter::new($name)
    };

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
    // Note: Tests for route_params! are in url_generation module
    // Other macro tests would go here if needed
}
