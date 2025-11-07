use crate::tenant::{Tenant, TenantId};
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Tenant middleware for Axum
#[derive(Clone)]
pub struct TenantMiddleware {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
}

impl TenantMiddleware {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, tenant: Tenant) {
        let mut tenants = self.tenants.write().await;
        tenants.insert(tenant.id.clone(), tenant);
    }

    pub async fn find_by_domain(&self, domain: &str) -> Option<Tenant> {
        let tenants = self.tenants.read().await;
        tenants
            .values()
            .find(|t| t.domain.as_deref() == Some(domain))
            .cloned()
    }

    pub async fn find_by_slug(&self, slug: &str) -> Option<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.values().find(|t| t.slug == slug).cloned()
    }

    pub async fn find(&self, id: &TenantId) -> Option<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.get(id).cloned()
    }
}

impl Default for TenantMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware function to identify tenant from request
pub async fn tenant_identifier_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    // Extract tenant from host header or subdomain
    let host = request
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Extract tenant ID from subdomain or path
    // This is a simple implementation - can be customized
    let tenant_id = extract_tenant_from_host(host);

    // Store tenant ID in request extensions
    if let Some(id) = tenant_id {
        request.extensions_mut().insert(id);
    }

    next.run(request).await
}

fn extract_tenant_from_host(host: &str) -> Option<TenantId> {
    // Extract subdomain as tenant identifier
    // Example: tenant1.example.com -> tenant1
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 3 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_middleware() {
        let middleware = TenantMiddleware::new();
        let tenant = Tenant::new("acme", "Acme Corp")
            .with_domain("acme.example.com");

        middleware.register(tenant.clone()).await;

        let found = middleware.find_by_domain("acme.example.com").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug, "acme");
    }

    #[test]
    fn test_extract_tenant_from_host() {
        let tenant = extract_tenant_from_host("acme.example.com");
        assert_eq!(tenant, Some("acme".to_string()));

        let tenant = extract_tenant_from_host("example.com");
        assert_eq!(tenant, None);
    }
}
