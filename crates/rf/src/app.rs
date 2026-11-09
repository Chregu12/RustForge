//! Instance-owned routing and database scope for Laravel-style handlers.

use axum::{handler::Handler, middleware, Router};
use rf_orm::DatabaseManager;
use rf_routing::GlobalRouter;

/// Build an independent application without mutating the global route registry.
/// The supplied database is also usable by explicit services and extractors.
#[derive(Debug, Default)]
pub struct App {
    routes: GlobalRouter,
    database: Option<DatabaseManager>,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn database(mut self, database: DatabaseManager) -> Self {
        self.database = Some(database);
        self
    }

    pub fn get<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.routes.add_get(path, handler);
        self
    }

    pub fn post<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.routes.add_post(path, handler);
        self
    }

    pub fn put<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.routes.add_put(path, handler);
        self
    }

    pub fn patch<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.routes.add_patch(path, handler);
        self
    }

    pub fn delete<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.routes.add_delete(path, handler);
        self
    }

    /// Inspect the HTTP routes registered on this application before booting it.
    pub fn routes(&self) -> Vec<(axum::http::Method, String)> {
        self.routes.registered_handlers()
    }

    /// Install the request context once, plus the optional application's pool.
    /// The pool is also exposed as `Extension<DatabaseManager>` for typed code.
    pub fn build(self) -> Router {
        let router = self
            .routes
            .build_router()
            .layer(middleware::from_fn(rf_request::capture_request));
        if let Some(database) = self.database {
            let scope_database = database.clone();
            router
                .layer(axum::Extension(database))
                .layer(middleware::from_fn(
                    move |request: axum::extract::Request, next: middleware::Next| {
                        let database = scope_database.clone();
                        async move { database.scope(next.run(request)).await }
                    },
                ))
        } else {
            router
        }
    }
}
