//! Database notification channel using SeaORM

use crate::channels::NotificationChannel;
use crate::entities::{notification, prelude::*};
use crate::{Notifiable, Notification, NotificationError, NotificationResult};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

/// Database channel that stores notifications in database
pub struct DatabaseChannel {
    db: DatabaseConnection,
}

impl DatabaseChannel {
    /// Create a new database channel
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get all notifications for a user
    pub async fn get_notifications(
        &self,
        user_id: i64,
    ) -> NotificationResult<Vec<notification::Model>> {
        Ok(NotificationEntity::find()
            .filter(notification::Column::NotifiableId.eq(user_id))
            .filter(notification::Column::NotifiableType.eq("User"))
            .order_by_desc(notification::Column::CreatedAt)
            .all(&self.db)
            .await?)
    }

    /// Get unread notifications
    pub async fn get_unread_notifications(
        &self,
        user_id: i64,
    ) -> NotificationResult<Vec<notification::Model>> {
        Ok(NotificationEntity::find()
            .filter(notification::Column::NotifiableId.eq(user_id))
            .filter(notification::Column::NotifiableType.eq("User"))
            .filter(notification::Column::ReadAt.is_null())
            .order_by_desc(notification::Column::CreatedAt)
            .all(&self.db)
            .await?)
    }

    /// Mark notification as read
    pub async fn mark_as_read(
        &self,
        notification_id: Uuid,
    ) -> NotificationResult<notification::Model> {
        let notification = NotificationEntity::find_by_id(notification_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| NotificationError::SendError("Notification not found".to_string()))?;

        let mut notification: notification::ActiveModel = notification.into();
        notification.read_at = Set(Some(sea_orm::prelude::DateTimeUtc::from(chrono::Utc::now())));

        Ok(notification.update(&self.db).await?)
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_as_read(&self, user_id: i64) -> NotificationResult<()> {
        let notifications = self.get_unread_notifications(user_id).await?;

        for notification in notifications {
            let mut notification: notification::ActiveModel = notification.into();
            notification.read_at = Set(Some(sea_orm::prelude::DateTimeUtc::from(chrono::Utc::now())));
            notification.update(&self.db).await?;
        }

        Ok(())
    }

    /// Get unread count
    pub async fn unread_count(&self, user_id: i64) -> NotificationResult<u64> {
        Ok(NotificationEntity::find()
            .filter(notification::Column::NotifiableId.eq(user_id))
            .filter(notification::Column::NotifiableType.eq("User"))
            .filter(notification::Column::ReadAt.is_null())
            .count(&self.db)
            .await?)
    }

    /// Delete notification
    pub async fn delete(&self, notification_id: Uuid) -> NotificationResult<()> {
        NotificationEntity::delete_by_id(notification_id)
            .exec(&self.db)
            .await?;
        Ok(())
    }

    /// Delete all notifications for a user
    pub async fn delete_all(&self, user_id: i64) -> NotificationResult<()> {
        NotificationEntity::delete_many()
            .filter(notification::Column::NotifiableId.eq(user_id))
            .filter(notification::Column::NotifiableType.eq("User"))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl NotificationChannel for DatabaseChannel {
    async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        // Get database notification from notification
        let db_notification = notification.to_database().await.ok_or_else(|| {
            NotificationError::ChannelError("No database notification provided".to_string())
        })?;

        // Get notifiable ID
        let notifiable_id = notifiable.route_notification_for_database().ok_or_else(|| {
            NotificationError::RoutingError("No user ID found".to_string())
        })?;

        // Create active model
        let active_model = notification::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            r#type: Set(std::any::type_name_of_val(notification).to_string()),
            notifiable_id: Set(notifiable_id),
            notifiable_type: Set("User".to_string()),
            data: Set(serde_json::json!({
                "title": db_notification.title,
                "message": db_notification.message,
                "data": db_notification.data,
            })),
            read_at: Set(None),
            created_at: Set(sea_orm::prelude::DateTimeUtc::from(chrono::Utc::now())),
        };

        // Insert into database
        active_model.insert(&self.db).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::DatabaseNotification;
    use sea_orm::{DbBackend, MockDatabase, MockExecResult};

    struct TestUser {
        id: i64,
    }

    impl Notifiable for TestUser {
        fn route_notification_for_database(&self) -> Option<i64> {
            Some(self.id)
        }
    }

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<crate::Channel> {
            vec![crate::Channel::Database]
        }

        async fn to_database(&self) -> Option<DatabaseNotification> {
            Some(DatabaseNotification {
                title: "Test".to_string(),
                message: "Test message".to_string(),
                data: serde_json::json!({"key": "value"}),
            })
        }
    }

    #[tokio::test]
    async fn test_database_channel_send() {
        // Create mock database
        let db = MockDatabase::new(DbBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let channel = DatabaseChannel::new(db);
        let user = TestUser { id: 1 };
        let notification = TestNotification;

        // This will execute the mock
        let result = channel.send(&notification, &user).await;
        assert!(result.is_ok());
    }
}
