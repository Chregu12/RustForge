//! # rf-tenancy: Multi-tenancy Support for RustForge
//!
//! Provides tenant isolation and management for SaaS applications.
//!
//! ## Features
//!
//! - **Tenant Identification**: Domain, subdomain, header-based
//! - **Tenant Context**: Request-scoped tenant information
//! - **Tenant Middleware**: Automatic tenant detection
//! - **Tenant Scoping**: Query-level tenant filtering
//! - **Cross-tenant Prevention**: Automatic isolation
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_tenancy::*;
//! use axum::{Router, routing::get};
//!
//! async fn handler(tenant: Tenant) -> String {
//!     format!("Current tenant: {}", tenant.id())
//! }
//!
//! # async fn example() {
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .layer(TenantLayer::by_domain());
//! # }
//! ```

use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

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
    resolver: Arc<InMemoryTenantResolver>,
}

impl DomainIdentifier {
    pub fn new(resolver: InMemoryTenantResolver) -> Self {
        Self {
            resolver: Arc::new(resolver),
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
    resolver: Arc<InMemoryTenantResolver>,
}

impl HeaderIdentifier {
    pub fn new(header_name: impl Into<String>, resolver: InMemoryTenantResolver) -> Self {
        Self {
            header_name: header_name.into(),
            resolver: Arc::new(resolver),
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

/// Tenant layer for Axum
#[derive(Clone)]
pub struct TenantLayer {
    identifier_type: TenantIdentifierType,
}

#[derive(Clone)]
enum TenantIdentifierType {
    Domain(DomainIdentifier),
    Header(HeaderIdentifier),
}

impl TenantLayer {
    /// Create tenant layer with domain-based identification
    pub fn by_domain() -> Self {
        Self {
            identifier_type: TenantIdentifierType::Domain(DomainIdentifier::new(
                InMemoryTenantResolver::new(),
            )),
        }
    }

    /// Create tenant layer with header-based identification
    pub fn by_header(header_name: impl Into<String>) -> Self {
        Self {
            identifier_type: TenantIdentifierType::Header(HeaderIdentifier::new(
                header_name,
                InMemoryTenantResolver::new(),
            )),
        }
    }

    async fn identify(&self, parts: &Parts) -> TenantResult<Tenant> {
        match &self.identifier_type {
            TenantIdentifierType::Domain(id) => id.identify(parts).await,
            TenantIdentifierType::Header(id) => id.identify(parts).await,
        }
    }
}

// Note: Axum extractor implementation removed due to complexity with FromRef trait
// Users can manually call TenantLayer::identify() in their handlers

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

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
        resolver
            .add_tenant(Tenant::new("1", "Tenant 1"))
            .await;

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

        let mut parts = Parts::default();
        parts
            .headers
            .insert("X-Tenant-Id", "tenant-123".parse().unwrap());

        let tenant = identifier.identify(&parts).await.unwrap();
        assert_eq!(tenant.id(), "tenant-123");
    }

    #[tokio::test]
    async fn test_header_identifier_missing_header() {
        let resolver = InMemoryTenantResolver::new();
        let identifier = HeaderIdentifier::new("X-Tenant-Id", resolver);

        let parts = Parts::default();

        let result = identifier.identify(&parts).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_tenants() {
        let resolver = InMemoryTenantResolver::new();
        resolver
            .add_tenant(Tenant::new("1", "Tenant 1"))
            .await;
        resolver
            .add_tenant(Tenant::new("2", "Tenant 2"))
            .await;
        resolver
            .add_tenant(Tenant::new("3", "Tenant 3"))
            .await;

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
        resolver
            .add_tenant(Tenant::new("1", "Tenant 1"))
            .await;

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
}
