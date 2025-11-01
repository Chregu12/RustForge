use crate::tenant::{Tenant, TenantError, TenantId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Tenant manager for CRUD operations
pub struct TenantManager {
    tenants: Arc<RwLock<HashMap<TenantId, Tenant>>>,
    domains: Arc<RwLock<HashMap<String, TenantId>>>,
    slugs: Arc<RwLock<HashMap<String, TenantId>>>,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            domains: Arc::new(RwLock::new(HashMap::new())),
            slugs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new tenant
    pub async fn register(&self, tenant: Tenant) {
        let id = tenant.id.clone();
        let slug = tenant.slug.clone();
        let domain = tenant.domain.clone();

        info!(tenant_id = %id, slug = %slug, "Registering tenant");

        let mut tenants = self.tenants.write().await;
        let mut slugs = self.slugs.write().await;

        tenants.insert(id.clone(), tenant);
        slugs.insert(slug, id.clone());

        if let Some(domain) = domain {
            let mut domains = self.domains.write().await;
            domains.insert(domain, id);
        }

        debug!("Tenant registered successfully");
    }

    /// Find tenant by ID
    pub async fn find(&self, id: &TenantId) -> Result<Tenant, TenantError> {
        let tenants = self.tenants.read().await;
        tenants
            .get(id)
            .cloned()
            .ok_or_else(|| TenantError::NotFound(id.clone()))
    }

    /// Find tenant by domain
    pub async fn find_by_domain(&self, domain: &str) -> Result<Tenant, TenantError> {
        let domains = self.domains.read().await;
        let tenant_id = domains
            .get(domain)
            .ok_or_else(|| TenantError::NotFound(domain.to_string()))?;

        self.find(tenant_id).await
    }

    /// Find tenant by slug
    pub async fn find_by_slug(&self, slug: &str) -> Result<Tenant, TenantError> {
        let slugs = self.slugs.read().await;
        let tenant_id = slugs
            .get(slug)
            .ok_or_else(|| TenantError::NotFound(slug.to_string()))?;

        self.find(tenant_id).await
    }

    /// List all tenants
    pub async fn list(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.values().cloned().collect()
    }

    /// List active tenants only
    pub async fn list_active(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read().await;
        tenants
            .values()
            .filter(|t| t.is_active())
            .cloned()
            .collect()
    }

    /// Update tenant
    pub async fn update(&self, id: &TenantId, tenant: Tenant) -> Result<(), TenantError> {
        let mut tenants = self.tenants.write().await;

        if !tenants.contains_key(id) {
            return Err(TenantError::NotFound(id.clone()));
        }

        tenants.insert(id.clone(), tenant);
        Ok(())
    }

    /// Delete tenant
    pub async fn delete(&self, id: &TenantId) -> Result<(), TenantError> {
        let mut tenants = self.tenants.write().await;
        let mut domains = self.domains.write().await;
        let mut slugs = self.slugs.write().await;

        if let Some(tenant) = tenants.remove(id) {
            slugs.remove(&tenant.slug);
            if let Some(domain) = tenant.domain {
                domains.remove(&domain);
            }
            Ok(())
        } else {
            Err(TenantError::NotFound(id.clone()))
        }
    }

    /// Count total tenants
    pub async fn count(&self) -> usize {
        let tenants = self.tenants.read().await;
        tenants.len()
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_manager_register() {
        let manager = TenantManager::new();
        let tenant = Tenant::new("acme", "Acme Corp");

        manager.register(tenant.clone()).await;

        let found = manager.find(&tenant.id).await.unwrap();
        assert_eq!(found.slug, "acme");
    }

    #[tokio::test]
    async fn test_tenant_manager_find_by_slug() {
        let manager = TenantManager::new();
        let tenant = Tenant::new("acme", "Acme Corp");

        manager.register(tenant.clone()).await;

        let found = manager.find_by_slug("acme").await.unwrap();
        assert_eq!(found.id, tenant.id);
    }

    #[tokio::test]
    async fn test_tenant_manager_list() {
        let manager = TenantManager::new();

        manager.register(Tenant::new("acme", "Acme Corp")).await;
        manager.register(Tenant::new("globex", "Globex Corp")).await;

        let tenants = manager.list().await;
        assert_eq!(tenants.len(), 2);
    }

    #[tokio::test]
    async fn test_tenant_manager_delete() {
        let manager = TenantManager::new();
        let tenant = Tenant::new("acme", "Acme Corp");
        let id = tenant.id.clone();

        manager.register(tenant).await;
        manager.delete(&id).await.unwrap();

        assert!(manager.find(&id).await.is_err());
    }
}
