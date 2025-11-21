//! Notification entity for database storage

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub r#type: String,
    pub notifiable_id: i64,
    pub notifiable_type: String,
    pub data: Json,
    pub read_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Migration SQL for creating the notifications table
pub const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    type VARCHAR NOT NULL,
    notifiable_id BIGINT NOT NULL,
    notifiable_type VARCHAR NOT NULL,
    data JSONB NOT NULL,
    read_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_notifiable ON notifications(notifiable_id, notifiable_type);
CREATE INDEX idx_notifications_read_at ON notifications(read_at);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_model() {
        let notification = Model {
            id: Uuid::new_v4(),
            r#type: "TestNotification".to_string(),
            notifiable_id: 1,
            notifiable_type: "User".to_string(),
            data: serde_json::json!({"key": "value"}),
            read_at: None,
            created_at: DateTimeUtc::from(chrono::Utc::now()),
        };

        assert_eq!(notification.notifiable_id, 1);
        assert_eq!(notification.notifiable_type, "User");
    }
}
