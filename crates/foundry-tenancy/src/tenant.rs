use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Tenant ID type
pub type TenantId = String;

/// Tenant model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub slug: String,
    pub name: String,
    pub domain: Option<String>,
    pub settings: serde_json::Value,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    pub fn new(slug: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            slug: slug.into(),
            name: name.into(),
            domain: None,
            settings: serde_json::json!({}),
            active: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn with_settings(mut self, settings: serde_json::Value) -> Self {
        self.settings = settings;
        self
    }

    pub fn deactivate(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl fmt::Display for Tenant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.slug)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TenantError {
    #[error("Tenant not found: {0}")]
    NotFound(String),

    #[error("Tenant inactive: {0}")]
    Inactive(String),

    #[error("Invalid tenant: {0}")]
    Invalid(String),

    #[error("Tenant error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("acme", "Acme Corp");
        assert_eq!(tenant.slug, "acme");
        assert_eq!(tenant.name, "Acme Corp");
        assert!(tenant.is_active());
    }

    #[test]
    fn test_tenant_with_domain() {
        let tenant = Tenant::new("acme", "Acme Corp")
            .with_domain("acme.example.com");

        assert_eq!(tenant.domain, Some("acme.example.com".to_string()));
    }

    #[test]
    fn test_tenant_deactivate() {
        let tenant = Tenant::new("acme", "Acme Corp").deactivate();
        assert!(!tenant.is_active());
    }
}
