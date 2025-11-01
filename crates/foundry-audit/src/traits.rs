//! Audit traits for models

use async_trait::async_trait;
use crate::{AuditAction, AuditLog, Result};

/// Context for audit logging
#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub user_id: Option<i64>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub reason: Option<String>,
}

impl AuditContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, ua: String) -> Self {
        self.user_agent = Some(ua);
        self
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }
}

/// Trait for auditable models
#[async_trait]
pub trait Auditable: serde::Serialize {
    /// Get the type name for audit logs
    fn auditable_type() -> String {
        std::any::type_name::<Self>().to_string()
    }

    /// Get the ID for audit logs
    fn auditable_id(&self) -> String;

    /// Create audit log for this model
    fn create_audit_log(
        &self,
        action: AuditAction,
        context: &AuditContext,
    ) -> Result<AuditLog> {
        let mut log = AuditLog::new(
            action,
            Self::auditable_type(),
            self.auditable_id(),
        );

        if let Some(user_id) = context.user_id {
            log = log.with_user(user_id);
        }

        if let Some(ref ip) = context.ip_address {
            log = log.with_ip(ip.clone());
        }

        if let Some(ref ua) = context.user_agent {
            log = log.with_user_agent(ua.clone());
        }

        if let Some(ref reason) = context.reason {
            log = log.with_reason(reason.clone());
        }

        let new_values = serde_json::to_value(self)?;
        log = log.with_new_values(new_values);

        Ok(log)
    }

    /// Create audit log with old values (for updates)
    fn create_update_audit_log(
        &self,
        old: &Self,
        context: &AuditContext,
    ) -> Result<AuditLog> {
        let old_values = serde_json::to_value(old)?;
        let new_values = serde_json::to_value(self)?;

        let mut log = AuditLog::new(
            AuditAction::Updated,
            Self::auditable_type(),
            self.auditable_id(),
        );

        if let Some(user_id) = context.user_id {
            log = log.with_user(user_id);
        }

        if let Some(ref ip) = context.ip_address {
            log = log.with_ip(ip.clone());
        }

        if let Some(ref ua) = context.user_agent {
            log = log.with_user_agent(ua.clone());
        }

        if let Some(ref reason) = context.reason {
            log = log.with_reason(reason.clone());
        }

        log = log.with_old_values(old_values).with_new_values(new_values);

        Ok(log)
    }
}

/// Extension trait for Sea-ORM models
#[async_trait]
pub trait AuditableExt: Auditable {
    /// Log creation
    async fn audit_create<C>(&self, db: &C, context: &AuditContext) -> Result<()>
    where
        C: sea_orm::ConnectionTrait + Send + Sync,
    {
        let log = self.create_audit_log(AuditAction::Created, context)?;
        // Save to database
        // This would use Sea-ORM to insert the audit log
        Ok(())
    }

    /// Log update
    async fn audit_update<C>(
        &self,
        db: &C,
        old: &Self,
        context: &AuditContext,
    ) -> Result<()>
    where
        C: sea_orm::ConnectionTrait + Send + Sync,
    {
        let log = self.create_update_audit_log(old, context)?;
        // Save to database
        Ok(())
    }

    /// Log deletion
    async fn audit_delete<C>(&self, db: &C, context: &AuditContext) -> Result<()>
    where
        C: sea_orm::ConnectionTrait + Send + Sync,
    {
        let log = self.create_audit_log(AuditAction::Deleted, context)?;
        // Save to database
        Ok(())
    }
}
