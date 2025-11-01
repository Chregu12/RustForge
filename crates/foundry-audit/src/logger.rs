//! Audit logger implementation

use crate::{AuditAction, AuditContext, AuditLog, Result};
use sea_orm::ConnectionTrait;

/// Audit logger for recording changes
pub struct AuditLogger<'a, C>
where
    C: ConnectionTrait,
{
    db: &'a C,
    context: AuditContext,
}

impl<'a, C> AuditLogger<'a, C>
where
    C: ConnectionTrait,
{
    pub fn new(db: &'a C, context: AuditContext) -> Self {
        Self { db, context }
    }

    pub async fn log(
        &self,
        action: AuditAction,
        auditable_type: String,
        auditable_id: String,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
    ) -> Result<AuditLog> {
        let mut log = AuditLog::new(action, auditable_type, auditable_id);

        if let Some(user_id) = self.context.user_id {
            log = log.with_user(user_id);
        }

        if let Some(ref ip) = self.context.ip_address {
            log = log.with_ip(ip.clone());
        }

        if let Some(ref ua) = self.context.user_agent {
            log = log.with_user_agent(ua.clone());
        }

        if let Some(ref reason) = self.context.reason {
            log = log.with_reason(reason.clone());
        }

        if let Some(old) = old_values {
            log = log.with_old_values(old);
        }

        if let Some(new) = new_values {
            log = log.with_new_values(new);
        }

        // Insert into database
        // This would use Sea-ORM to save the audit log

        Ok(log)
    }

    pub async fn log_create(
        &self,
        auditable_type: String,
        auditable_id: String,
        new_values: serde_json::Value,
    ) -> Result<AuditLog> {
        self.log(
            AuditAction::Created,
            auditable_type,
            auditable_id,
            None,
            Some(new_values),
        )
        .await
    }

    pub async fn log_update(
        &self,
        auditable_type: String,
        auditable_id: String,
        old_values: serde_json::Value,
        new_values: serde_json::Value,
    ) -> Result<AuditLog> {
        self.log(
            AuditAction::Updated,
            auditable_type,
            auditable_id,
            Some(old_values),
            Some(new_values),
        )
        .await
    }

    pub async fn log_delete(
        &self,
        auditable_type: String,
        auditable_id: String,
        old_values: serde_json::Value,
    ) -> Result<AuditLog> {
        self.log(
            AuditAction::Deleted,
            auditable_type,
            auditable_id,
            Some(old_values),
            None,
        )
        .await
    }
}
