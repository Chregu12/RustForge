use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DbErr};

/// Model events that can be fired during model lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelEvent {
    /// Fired before a model is created
    Creating,
    /// Fired after a model is created
    Created,
    /// Fired before a model is updated
    Updating,
    /// Fired after a model is updated
    Updated,
    /// Fired before a model is deleted
    Deleting,
    /// Fired after a model is deleted
    Deleted,
    /// Fired before a model is saved (create or update)
    Saving,
    /// Fired after a model is saved (create or update)
    Saved,
}

/// Result of an event handler
pub type EventResult = Result<(), DbErr>;

/// Trait for models that support lifecycle events
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::events::{ModelEvents, EventResult};
/// use async_trait::async_trait;
/// use sea_orm::Set;
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub title: String, pub slug: String }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
///
/// #[async_trait]
/// impl ModelEvents for post::ActiveModel {
///     async fn before_create(&mut self) -> EventResult {
///         // Set slug from title before creating
///         if self.slug.is_not_set() {
///             self.slug = Set(self.title.as_ref().to_lowercase());
///         }
///         Ok(())
///     }
///
///     async fn after_create(&self) -> EventResult {
///         // Send notification after creating
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait ModelEvents: ActiveModelTrait {
    /// Called before creating a new model
    async fn before_create(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after creating a new model
    async fn after_create(&self) -> EventResult {
        Ok(())
    }

    /// Called before updating a model
    async fn before_update(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after updating a model
    async fn after_update(&self) -> EventResult {
        Ok(())
    }

    /// Called before deleting a model
    async fn before_delete(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after deleting a model
    async fn after_delete(&self) -> EventResult {
        Ok(())
    }

    /// Called before saving (create or update)
    async fn before_save(&mut self) -> EventResult {
        Ok(())
    }

    /// Called after saving (create or update)
    async fn after_save(&self) -> EventResult {
        Ok(())
    }
}

/// Global event observer system
///
/// Allows registering observers for specific models
pub struct EventObserver {
    observers:
        std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<ObserverCallback>>>>,
}

type ObserverCallback = Box<dyn Fn(ModelEvent) -> EventResult + Send + Sync>;

impl EventObserver {
    pub fn new() -> Self {
        Self {
            observers: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Register an observer for a model
    pub fn observe<F>(&self, model_name: &str, callback: F)
    where
        F: Fn(ModelEvent) -> EventResult + Send + Sync + 'static,
    {
        let mut observers = self.observers.lock().unwrap();
        observers
            .entry(model_name.to_string())
            .or_default()
            .push(Box::new(callback));
    }

    /// Fire an event for a model
    pub fn fire(&self, model_name: &str, event: ModelEvent) -> EventResult {
        let observers = self.observers.lock().unwrap();

        if let Some(callbacks) = observers.get(model_name) {
            for callback in callbacks {
                callback(event)?;
            }
        }

        Ok(())
    }
}

impl Default for EventObserver {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to automatically implement timestamps
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::timestamps;
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model {
/// #         #[sea_orm(primary_key)] pub id: i32,
/// #         pub created_at: DateTimeUtc,
/// #         pub updated_at: DateTimeUtc,
/// #     }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
///
/// timestamps!(post::ActiveModel, created_at, updated_at);
/// ```
#[macro_export]
macro_rules! timestamps {
    ($model:ty, $created_at:ident, $updated_at:ident) => {
        #[async_trait::async_trait]
        impl $crate::events::ModelEvents for $model {
            async fn before_create(&mut self) -> $crate::events::EventResult {
                use chrono::Utc;
                use sea_orm::ActiveValue::Set;

                let now = Utc::now();
                if self.$created_at.is_not_set() {
                    self.$created_at = Set(now);
                }
                if self.$updated_at.is_not_set() {
                    self.$updated_at = Set(now);
                }

                Ok(())
            }

            async fn before_update(&mut self) -> $crate::events::EventResult {
                use chrono::Utc;
                use sea_orm::ActiveValue::Set;

                self.$updated_at = Set(Utc::now());

                Ok(())
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_observer() {
        let observer = EventObserver::new();

        observer.observe("Post", |event| {
            match event {
                ModelEvent::Creating => {
                    // Do something
                }
                ModelEvent::Created => {
                    // Do something
                }
                _ => {}
            }
            Ok(())
        });

        assert!(observer.fire("Post", ModelEvent::Creating).is_ok());
    }
}
