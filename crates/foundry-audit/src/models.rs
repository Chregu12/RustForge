//! Audit log models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Audit action types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditAction {
    Created,
    Updated,
    Deleted,
    Restored,
    Custom(&'static str),
}

impl AuditAction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
            Self::Restored => "restored",
            Self::Custom(s) => s,
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<i64>,
    pub action: String,
    pub auditable_type: String,
    pub auditable_id: String,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl AuditLog {
    pub fn new(
        action: AuditAction,
        auditable_type: String,
        auditable_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: None,
            action: action.as_str().to_string(),
            auditable_type,
            auditable_id,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
            reason: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_user(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_old_values(mut self, values: serde_json::Value) -> Self {
        self.old_values = Some(values);
        self
    }

    pub fn with_new_values(mut self, values: serde_json::Value) -> Self {
        self.new_values = Some(values);
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

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Changes between old and new values
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditChanges {
    pub field: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

impl AuditChanges {
    pub fn extract(
        old: &serde_json::Value,
        new: &serde_json::Value,
    ) -> Vec<Self> {
        let mut changes = Vec::new();

        if let (Some(old_obj), Some(new_obj)) = (old.as_object(), new.as_object()) {
            for (key, new_val) in new_obj {
                let old_val = old_obj.get(key);
                if old_val != Some(new_val) {
                    changes.push(AuditChanges {
                        field: key.clone(),
                        old_value: old_val.cloned(),
                        new_value: Some(new_val.clone()),
                    });
                }
            }
        }

        changes
    }
}
