//! # rf-tenancy: Multi-tenancy Support for RustForge
//!
//! Building blocks for multi-tenant SaaS apps: identify the tenant of each
//! request, expose it as a request-scoped "current tenant", and scope your own
//! data access to it.
//!
//! ## What this crate does
//!
//! - **Tenant identification** — resolve the tenant from a request header
//!   ([`HeaderIdentifier`]) or the `Host`/domain ([`DomainIdentifier`]) via a
//!   resolver you register tenants on.
//! - **Request-scoped current tenant** — [`TenantLayer`] is a real Tower/axum
//!   layer (`impl Layer`/`Service`) that resolves the tenant once per request
//!   and establishes it in a task-local, readable anywhere in the request via
//!   [`Tenant::current`] / [`Tenant::current_id`].
//! - **Registerable resolver** — [`TenantResolver`] is a trait object
//!   (`Arc<dyn TenantResolver>`); the bundled [`InMemoryTenantResolver`] is a
//!   populated, registerable implementation, and you can supply your own
//!   (e.g. DB-backed) by implementing the trait.
//! - **Data-isolation helpers** — [`scope_to_current`] filters a set of
//!   [`TenantScoped`] rows down to the current tenant, and [`guard_tenant`]
//!   rejects a cross-tenant access with [`TenantError::CrossTenantAccess`].
//!
//! ## What this crate does NOT do
//!
//! Isolation is **not** automatic or transparent: this crate does not intercept
//! or rewrite your SQL. You opt in per query — either with the helpers above
//! over rows that implement [`TenantScoped`], or by filtering on the current
//! tenant id in your own query builder, e.g.:
//!
//! ```ignore
//! let tid = Tenant::current_id().expect("inside a tenant scope");
//! let rows = DB::table("posts").where_eq("tenant_id", tid).get().await?;
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_tenancy::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Register tenants on a resolver.
//! let resolver = InMemoryTenantResolver::new();
//! resolver
//!     .add_tenant(Tenant::with_domain("1", "Acme", "acme.example.com"))
//!     .await;
//!
//! // Build a real axum layer that sets the current tenant per request.
//! // (Attach it with `Router::new()...layer(layer)`.)
//! let _layer = TenantLayer::by_header("X-Tenant-Id", resolver.clone());
//!
//! // Inside a request handled by that layer, read the current tenant anywhere:
//! if let Some(tenant) = Tenant::current() {
//!     println!("Current tenant: {}", tenant.id());
//! }
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use axum::{
    extract::Request as AxumRequest,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::sync::RwLock;
use tower::{Layer, Service};

tokio::task_local! {
    /// The tenant established for the current request scope, set by
    /// [`TenantLayer`]'s middleware (or [`with_current_tenant`] in tests).
    static CURRENT_TENANT: Arc<Tenant>;
}

/// Run a future with `tenant` established as the current tenant for its scope.
///
/// This is what [`TenantLayer`]'s middleware wraps each request in; it is also
/// public so tests and non-HTTP code (jobs, CLI) can establish a tenant scope.
pub async fn with_current_tenant<F, R>(tenant: Tenant, fut: F) -> R
where
    F: Future<Output = R>,
{
    CURRENT_TENANT.scope(Arc::new(tenant), fut).await
}

/// Tenant errors
#[derive(Debug, Error)]
pub enum TenantError {
    #[error("Tenant not found")]
    NotFound,

    #[error("Invalid tenant identifier: {0}")]
    InvalidIdentifier(String),

    #[error("Cross-tenant access denied")]
    CrossTenantAccess,

    #[error("Tenant identification failed: {0}")]
    IdentificationFailed(String),
}

impl IntoResponse for TenantError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            TenantError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            TenantError::InvalidIdentifier(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            TenantError::CrossTenantAccess => (StatusCode::FORBIDDEN, self.to_string()),
            TenantError::IdentificationFailed(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        (status, message).into_response()
    }
}

/// Result type for tenant operations
pub type TenantResult<T> = Result<T, TenantError>;

/// Tenant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    id: String,
    name: String,
    domain: Option<String>,
}

impl Tenant {
    /// Create new tenant
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            domain: None,
        }
    }

    /// Create tenant with domain
    pub fn with_domain(
        id: impl Into<String>,
        name: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            domain: Some(domain.into()),
        }
    }

    /// Get tenant ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get tenant name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get tenant domain
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// The tenant established for the current request scope (by [`TenantLayer`]
    /// or [`with_current_tenant`]). Returns `None` outside a tenant scope.
    pub fn current() -> Option<Tenant> {
        CURRENT_TENANT.try_with(|t| (**t).clone()).ok()
    }

    /// Convenience: the current tenant's id, if a tenant scope is established.
    ///
    /// Handy for scoping a query, e.g.
    /// `DB::table("posts").where_eq("tenant_id", Tenant::current_id()?)`.
    pub fn current_id() -> Option<String> {
        CURRENT_TENANT.try_with(|t| t.id.clone()).ok()
    }
}

/// A record that belongs to a specific tenant.
///
/// Implement this on the rows/DTOs your queries return so the isolation helpers
/// ([`scope_to_current`], [`guard_tenant`]) can enforce tenant boundaries.
pub trait TenantScoped {
    /// The id of the tenant that owns this record.
    fn tenant_id(&self) -> &str;
}

/// Filter `rows` down to only those owned by the current tenant.
///
/// Returns an empty `Vec` when there is no current tenant scope (fail-closed:
/// no tenant means no rows, never everyone's rows).
pub fn scope_to_current<T: TenantScoped>(rows: impl IntoIterator<Item = T>) -> Vec<T> {
    match Tenant::current_id() {
        Some(id) => rows
            .into_iter()
            .filter(|r| r.tenant_id() == id)
            .collect(),
        None => Vec::new(),
    }
}

/// Guard a single record against cross-tenant access.
///
/// Returns [`TenantError::CrossTenantAccess`] when `row` belongs to a different
/// tenant than the current one, and [`TenantError::IdentificationFailed`] when
/// there is no current tenant scope at all. `Ok(())` only when the row belongs
/// to the current tenant.
pub fn guard_tenant<T: TenantScoped>(row: &T) -> TenantResult<()> {
    match Tenant::current_id() {
        Some(id) if row.tenant_id() == id => Ok(()),
        Some(_) => Err(TenantError::CrossTenantAccess),
        None => Err(TenantError::IdentificationFailed(
            "no current tenant scope".to_string(),
        )),
    }
}

/// Tenant identifier strategy
#[async_trait]
pub trait TenantIdentifier: Send + Sync {
    /// Identify tenant from request parts
    async fn identify(&self, parts: &Parts) -> TenantResult<Tenant>;
}

/// Domain-based tenant identification
#[derive(Clone)]
pub struct DomainIdentifier {
    resolver: Arc<dyn TenantResolver>,
}

impl DomainIdentifier {
    /// Create a domain identifier over any registerable resolver.
    ///
    /// Accepts either a concrete [`InMemoryTenantResolver`] (converted via its
    /// `From` impl) or an `Arc<dyn TenantResolver>` for a custom/DB-backed store.
    pub fn new(resolver: impl Into<Arc<dyn TenantResolver>>) -> Self {
        Self {
            resolver: resolver.into(),
        }
    }
}

#[async_trait]
impl TenantIdentifier for DomainIdentifier {
    async fn identify(&self, parts: &Parts) -> TenantResult<Tenant> {
        let host = parts
            .headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| TenantError::IdentificationFailed("No host header".to_string()))?;

        self.resolver.resolve_by_domain(host).await
    }
}

/// Header-based tenant identification
#[derive(Clone)]
pub struct HeaderIdentifier {
    header_name: String,
    resolver: Arc<dyn TenantResolver>,
}

impl HeaderIdentifier {
    /// Create a header identifier over any registerable resolver.
    ///
    /// Accepts either a concrete [`InMemoryTenantResolver`] (converted via its
    /// `From` impl) or an `Arc<dyn TenantResolver>` for a custom/DB-backed store.
    pub fn new(
        header_name: impl Into<String>,
        resolver: impl Into<Arc<dyn TenantResolver>>,
    ) -> Self {
        Self {
            header_name: header_name.into(),
            resolver: resolver.into(),
        }
    }
}

#[async_trait]
impl TenantIdentifier for HeaderIdentifier {
    async fn identify(&self, parts: &Parts) -> TenantResult<Tenant> {
        let headers = &parts.headers;
        let tenant_id = headers
            .get(&self.header_name)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                TenantError::IdentificationFailed(format!(
                    "Header '{}' not found",
                    self.header_name
                ))
            })?;

        self.resolver.resolve_by_id(tenant_id).await
    }
}

/// Tenant resolver trait
#[async_trait]
pub trait TenantResolver: Send + Sync {
    /// Resolve tenant by ID
    async fn resolve_by_id(&self, id: &str) -> TenantResult<Tenant>;

    /// Resolve tenant by domain
    async fn resolve_by_domain(&self, domain: &str) -> TenantResult<Tenant>;
}

/// In-memory tenant resolver (for testing/development)
#[derive(Clone)]
pub struct InMemoryTenantResolver {
    tenants: Arc<RwLock<Vec<Tenant>>>,
}

impl InMemoryTenantResolver {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_tenant(&self, tenant: Tenant) {
        let mut tenants = self.tenants.write().await;
        tenants.push(tenant);
    }
}

impl Default for InMemoryTenantResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Allow a concrete [`InMemoryTenantResolver`] to be used anywhere an
/// `Arc<dyn TenantResolver>` is expected, so the identifiers/layer stay
/// ergonomic while operating on the trait object.
impl From<InMemoryTenantResolver> for Arc<dyn TenantResolver> {
    fn from(resolver: InMemoryTenantResolver) -> Self {
        Arc::new(resolver)
    }
}

#[async_trait]
impl TenantResolver for InMemoryTenantResolver {
    async fn resolve_by_id(&self, id: &str) -> TenantResult<Tenant> {
        let tenants = self.tenants.read().await;
        tenants
            .iter()
            .find(|t| t.id() == id)
            .cloned()
            .ok_or(TenantError::NotFound)
    }

    async fn resolve_by_domain(&self, domain: &str) -> TenantResult<Tenant> {
        let tenants = self.tenants.read().await;
        tenants
            .iter()
            .find(|t| t.domain() == Some(domain))
            .cloned()
            .ok_or(TenantError::NotFound)
    }
}

/// A real Tower/axum layer that identifies the tenant of each request and
/// establishes it as the [current tenant](Tenant::current) for the duration of
/// the downstream handler.
///
/// Attach it like any other layer:
///
/// ```rust,no_run
/// use axum::{routing::get, Router};
/// use rf_tenancy::{InMemoryTenantResolver, Tenant, TenantLayer};
///
/// # async fn build() -> Router {
/// let resolver = InMemoryTenantResolver::new();
/// resolver.add_tenant(Tenant::new("acme", "Acme Inc")).await;
///
/// async fn handler() -> String {
///     Tenant::current_id().unwrap_or_else(|| "none".into())
/// }
///
/// Router::new()
///     .route("/", get(handler))
///     .layer(TenantLayer::by_header("X-Tenant-Id", resolver))
/// # }
/// ```
///
/// When identification fails (missing/unknown tenant) the request is rejected
/// with the [`TenantError`] response before the handler runs.
#[derive(Clone)]
pub struct TenantLayer {
    identifier: Arc<dyn TenantIdentifier>,
}

impl TenantLayer {
    /// Build a layer from any [`TenantIdentifier`] (custom strategies welcome).
    pub fn new(identifier: Arc<dyn TenantIdentifier>) -> Self {
        Self { identifier }
    }

    /// Create a layer that identifies the tenant from the request's
    /// `Host`/domain via `resolver`.
    pub fn by_domain(resolver: impl Into<Arc<dyn TenantResolver>>) -> Self {
        Self {
            identifier: Arc::new(DomainIdentifier::new(resolver)),
        }
    }

    /// Create a layer that identifies the tenant from a request header via
    /// `resolver` (e.g. `X-Tenant-Id`).
    pub fn by_header(
        header_name: impl Into<String>,
        resolver: impl Into<Arc<dyn TenantResolver>>,
    ) -> Self {
        Self {
            identifier: Arc::new(HeaderIdentifier::new(header_name, resolver)),
        }
    }

    /// Resolve the tenant for a request's [`Parts`] using this layer's
    /// identifier. Called by the middleware; also usable directly.
    pub async fn identify(&self, parts: &Parts) -> TenantResult<Tenant> {
        self.identifier.identify(parts).await
    }
}

impl<S> Layer<S> for TenantLayer {
    type Service = TenantMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TenantMiddleware {
            inner,
            identifier: self.identifier.clone(),
        }
    }
}

/// The [`Service`] produced by [`TenantLayer`]. Resolves the current tenant,
/// establishes the per-request task-local scope, then calls the inner service.
#[derive(Clone)]
pub struct TenantMiddleware<S> {
    inner: S,
    identifier: Arc<dyn TenantIdentifier>,
}

impl<S> Service<AxumRequest> for TenantMiddleware<S>
where
    S: Service<AxumRequest, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: AxumRequest) -> Self::Future {
        let identifier = self.identifier.clone();
        // Move the *ready* inner service into the future (Tower readiness
        // contract), leaving a clone behind for the next call.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let (parts, body) = req.into_parts();
            match identifier.identify(&parts).await {
                Ok(tenant) => {
                    let req = AxumRequest::from_parts(parts, body);
                    // Establish the current tenant for the whole downstream call.
                    with_current_tenant(tenant, inner.call(req)).await
                }
                // Identification failed: reject before the handler runs.
                Err(err) => Ok(err.into_response()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[tokio::test]
    async fn test_tenant_creation() {
        let tenant = Tenant::new("1", "Test Tenant");
        assert_eq!(tenant.id(), "1");
        assert_eq!(tenant.name(), "Test Tenant");
        assert_eq!(tenant.domain(), None);
    }

    #[tokio::test]
    async fn test_tenant_with_domain() {
        let tenant = Tenant::with_domain("1", "Test Tenant", "example.com");
        assert_eq!(tenant.id(), "1");
        assert_eq!(tenant.name(), "Test Tenant");
        assert_eq!(tenant.domain(), Some("example.com"));
    }

    #[tokio::test]
    async fn test_in_memory_resolver_by_id() {
        let resolver = InMemoryTenantResolver::new();
        resolver.add_tenant(Tenant::new("1", "Tenant 1")).await;

        let tenant = resolver.resolve_by_id("1").await.unwrap();
        assert_eq!(tenant.id(), "1");

        let result = resolver.resolve_by_id("999").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_resolver_by_domain() {
        let resolver = InMemoryTenantResolver::new();
        resolver
            .add_tenant(Tenant::with_domain("1", "Tenant 1", "tenant1.example.com"))
            .await;

        let tenant = resolver
            .resolve_by_domain("tenant1.example.com")
            .await
            .unwrap();
        assert_eq!(tenant.id(), "1");

        let result = resolver.resolve_by_domain("nonexistent.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_header_identifier() {
        let resolver = InMemoryTenantResolver::new();
        resolver
            .add_tenant(Tenant::new("tenant-123", "Test Tenant"))
            .await;

        let identifier = HeaderIdentifier::new("X-Tenant-Id", resolver);

        // Create Parts from a Request
        let request = Request::builder()
            .header("X-Tenant-Id", "tenant-123")
            .body(())
            .unwrap();
        let (parts, _body) = request.into_parts();

        let tenant = identifier.identify(&parts).await.unwrap();
        assert_eq!(tenant.id(), "tenant-123");
    }

    #[tokio::test]
    async fn test_header_identifier_missing_header() {
        let resolver = InMemoryTenantResolver::new();
        let identifier = HeaderIdentifier::new("X-Tenant-Id", resolver);

        // Create Parts from a Request (no headers)
        let request = Request::builder().body(()).unwrap();
        let (parts, _body) = request.into_parts();

        let result = identifier.identify(&parts).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_tenants() {
        let resolver = InMemoryTenantResolver::new();
        resolver.add_tenant(Tenant::new("1", "Tenant 1")).await;
        resolver.add_tenant(Tenant::new("2", "Tenant 2")).await;
        resolver.add_tenant(Tenant::new("3", "Tenant 3")).await;

        let tenant1 = resolver.resolve_by_id("1").await.unwrap();
        let tenant2 = resolver.resolve_by_id("2").await.unwrap();
        let tenant3 = resolver.resolve_by_id("3").await.unwrap();

        assert_eq!(tenant1.id(), "1");
        assert_eq!(tenant2.id(), "2");
        assert_eq!(tenant3.id(), "3");
    }

    #[tokio::test]
    async fn test_tenant_error_responses() {
        let err = TenantError::NotFound;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let err = TenantError::InvalidIdentifier("test".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = TenantError::CrossTenantAccess;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_concurrent_tenant_access() {
        let resolver = InMemoryTenantResolver::new();
        resolver.add_tenant(Tenant::new("1", "Tenant 1")).await;

        // Simulate concurrent access
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let resolver = resolver.clone();
                tokio::spawn(async move { resolver.resolve_by_id("1").await })
            })
            .collect();

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    // --- Current-tenant scope + isolation helpers ---

    struct Row {
        tenant_id: String,
        val: i32,
    }

    impl TenantScoped for Row {
        fn tenant_id(&self) -> &str {
            &self.tenant_id
        }
    }

    #[tokio::test]
    async fn test_current_tenant_scope() {
        // No tenant outside a scope.
        assert!(Tenant::current().is_none());
        assert!(Tenant::current_id().is_none());

        with_current_tenant(Tenant::new("acme", "Acme"), async {
            assert_eq!(Tenant::current_id().as_deref(), Some("acme"));
            assert_eq!(Tenant::current().unwrap().name(), "Acme");
        })
        .await;

        // Scope is gone again once the future completes.
        assert!(Tenant::current().is_none());
    }

    #[tokio::test]
    async fn test_scope_to_current_filters_rows() {
        let rows = || {
            vec![
                Row { tenant_id: "a".into(), val: 1 },
                Row { tenant_id: "b".into(), val: 2 },
                Row { tenant_id: "a".into(), val: 3 },
            ]
        };

        with_current_tenant(Tenant::new("a", "A"), async {
            let mine = scope_to_current(rows());
            assert_eq!(mine.len(), 2);
            assert!(mine.iter().all(|r| r.tenant_id == "a"));
            let vals: Vec<i32> = mine.iter().map(|r| r.val).collect();
            assert_eq!(vals, vec![1, 3]);
        })
        .await;

        // Fail-closed: no current tenant -> no rows, never everyone's rows.
        assert!(scope_to_current(rows()).is_empty());
    }

    #[tokio::test]
    async fn test_guard_tenant_constructs_cross_tenant_access() {
        with_current_tenant(Tenant::new("a", "A"), async {
            let own = Row { tenant_id: "a".into(), val: 1 };
            assert!(guard_tenant(&own).is_ok());

            let foreign = Row { tenant_id: "b".into(), val: 2 };
            assert!(matches!(
                guard_tenant(&foreign),
                Err(TenantError::CrossTenantAccess)
            ));
        })
        .await;

        // Outside any scope the guard fails closed (never silently allows).
        let row = Row { tenant_id: "a".into(), val: 1 };
        assert!(matches!(
            guard_tenant(&row),
            Err(TenantError::IdentificationFailed(_))
        ));
    }

    #[tokio::test]
    async fn test_tenant_layer_sets_current_per_request() {
        use axum::{body::Body, routing::get, Router};
        use tower::ServiceExt;

        let resolver = InMemoryTenantResolver::new();
        resolver.add_tenant(Tenant::new("t1", "One")).await;
        resolver.add_tenant(Tenant::new("t2", "Two")).await;

        async fn who() -> String {
            Tenant::current_id().unwrap_or_else(|| "none".into())
        }

        let app = Router::new()
            .route("/", get(who))
            .layer(TenantLayer::by_header("X-Tenant-Id", resolver));

        async fn call(app: &Router, tenant: &str) -> Response {
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .header("X-Tenant-Id", tenant)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
        }

        // Each request resolves its OWN current tenant.
        let r1 = call(&app, "t1").await;
        let b1 = axum::body::to_bytes(r1.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&b1[..], b"t1");

        let r2 = call(&app, "t2").await;
        let b2 = axum::body::to_bytes(r2.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&b2[..], b"t2");

        // Unknown tenant is rejected (404) before the handler runs.
        let r3 = call(&app, "nope").await;
        assert_eq!(r3.status(), StatusCode::NOT_FOUND);
    }
}
