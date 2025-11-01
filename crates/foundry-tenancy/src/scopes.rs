use crate::tenant::TenantId;

/// Tenant scope for database queries
#[derive(Debug, Clone)]
pub struct TenantScope {
    pub tenant_id: TenantId,
}

impl TenantScope {
    pub fn new(tenant_id: TenantId) -> Self {
        Self { tenant_id }
    }

    /// Apply tenant scope to a query
    /// This would be used with SeaORM or other ORMs
    pub fn apply_to_query(&self) -> String {
        format!("tenant_id = '{}'", self.tenant_id)
    }

    /// Check if a record belongs to this tenant
    pub fn owns(&self, record_tenant_id: &str) -> bool {
        self.tenant_id == record_tenant_id
    }
}

/// Trait for tenant-aware models
pub trait TenantAware {
    fn tenant_id(&self) -> &str;

    fn belongs_to_tenant(&self, tenant_id: &str) -> bool {
        self.tenant_id() == tenant_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_scope() {
        let scope = TenantScope::new("tenant-123".to_string());
        assert!(scope.owns("tenant-123"));
        assert!(!scope.owns("tenant-456"));
    }

    #[test]
    fn test_apply_to_query() {
        let scope = TenantScope::new("tenant-123".to_string());
        let query = scope.apply_to_query();
        assert!(query.contains("tenant-123"));
    }
}
