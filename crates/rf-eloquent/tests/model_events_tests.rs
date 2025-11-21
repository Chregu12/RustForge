//! Comprehensive tests for Model Events system
//!
//! Tests cover:
//! - Creating/created events
//! - Updating/updated events
//! - Deleting/deleted events
//! - Canceling operations from events
//! - Multiple listeners
//! - Event with model modification
//! - Global event listeners
//! - Async operations in events
//! - Event context and metadata
//! - Event dispatcher functionality

use async_trait::async_trait;
use chrono::Utc;
use rf_eloquent::events::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test 1: Fire creating event before insert
#[tokio::test]
async fn test_fire_creating_event() {
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();

    #[derive(Clone, Debug)]
    struct User {
        id: i32,
        name: String,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn creating(&mut self) -> EventResult {
            // This would be called before insert
            Ok(())
        }
    }

    let mut user = User {
        id: 0,
        name: "Test".to_string(),
    };

    let result = user.creating().await;
    assert!(result.is_ok());
}

/// Test 2: Fire created event after insert
#[tokio::test]
async fn test_fire_created_event() {
    #[derive(Clone, Debug)]
    struct User {
        id: i32,
        name: String,
        created_at: Option<chrono::DateTime<Utc>>,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn created(&self) -> EventResult {
            // This would be called after insert
            // Could send welcome email, etc.
            Ok(())
        }
    }

    let user = User {
        id: 1,
        name: "Test".to_string(),
        created_at: Some(Utc::now()),
    };

    let result = user.created().await;
    assert!(result.is_ok());
}

/// Test 3: Fire updating event before update
#[tokio::test]
async fn test_fire_updating_event() {
    #[derive(Clone, Debug)]
    struct User {
        id: i32,
        email: String,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn updating(&mut self) -> EventResult {
            // Validate before update
            if self.email.is_empty() {
                return Err(EventError::ValidationFailed(
                    "Email is required".to_string(),
                ));
            }
            Ok(())
        }
    }

    let mut user = User {
        id: 1,
        email: "test@example.com".to_string(),
    };

    let result = user.updating().await;
    assert!(result.is_ok());

    // Test validation failure
    user.email = "".to_string();
    let result = user.updating().await;
    assert!(result.is_err());
}

/// Test 4: Fire updated event after update
#[tokio::test]
async fn test_fire_updated_event() {
    #[derive(Clone, Debug)]
    struct User {
        id: i32,
        name: String,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn updated(&self) -> EventResult {
            // Log the update, invalidate cache, etc.
            Ok(())
        }
    }

    let user = User {
        id: 1,
        name: "Updated".to_string(),
    };

    let result = user.updated().await;
    assert!(result.is_ok());
}

/// Test 5: Fire deleting event before delete
#[tokio::test]
async fn test_fire_deleting_event() {
    #[derive(Clone, Debug)]
    struct User {
        id: i32,
        has_active_subscription: bool,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn deleting(&mut self) -> EventResult {
            // Prevent deletion if user has active subscription
            if self.has_active_subscription {
                return Err(EventError::ValidationFailed(
                    "Cannot delete user with active subscription".to_string(),
                ));
            }
            Ok(())
        }
    }

    let mut user = User {
        id: 1,
        has_active_subscription: false,
    };

    let result = user.deleting().await;
    assert!(result.is_ok());

    // Test prevention
    user.has_active_subscription = true;
    let result = user.deleting().await;
    assert!(result.is_err());
}

/// Test 6: Fire deleted event after delete
#[tokio::test]
async fn test_fire_deleted_event() {
    #[derive(Clone, Debug)]
    struct User {
        id: i32,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn deleted(&self) -> EventResult {
            // Clean up related data, send notifications, etc.
            Ok(())
        }
    }

    let user = User { id: 1 };
    let result = user.deleted().await;
    assert!(result.is_ok());
}

/// Test 7: Cancel operation from creating event (return error)
#[tokio::test]
async fn test_cancel_from_creating_event() {
    #[derive(Clone, Debug)]
    struct User {
        age: i32,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn creating(&mut self) -> EventResult {
            if self.age < 18 {
                return Err(EventError::ValidationFailed(
                    "User must be 18 or older".to_string(),
                ));
            }
            Ok(())
        }
    }

    let mut user = User { age: 16 };
    let result = user.creating().await;
    assert!(result.is_err());
    match result {
        Err(EventError::ValidationFailed(msg)) => {
            assert_eq!(msg, "User must be 18 or older");
        }
        _ => panic!("Expected ValidationFailed error"),
    }
}

/// Test 8: Cancel operation from updating event
#[tokio::test]
async fn test_cancel_from_updating_event() {
    #[derive(Clone, Debug)]
    struct Post {
        published: bool,
        views: i32,
    }

    #[async_trait]
    impl ModelEvents for Post {
        async fn updating(&mut self) -> EventResult {
            if self.published && self.views > 1000 {
                return Err(EventError::ValidationFailed(
                    "Cannot modify popular published posts".to_string(),
                ));
            }
            Ok(())
        }
    }

    let mut post = Post {
        published: true,
        views: 1500,
    };
    let result = post.updating().await;
    assert!(result.is_err());
}

/// Test 9: Multiple listeners for same event
#[tokio::test]
async fn test_multiple_listeners() {
    let dispatcher = EventDispatcher::new();
    let call_count = Arc::new(AtomicU32::new(0));

    // Register first listener
    let count1 = call_count.clone();
    dispatcher
        .listen(ModelEvent::Creating, "User", move |_ctx| {
            count1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;

    // Register second listener
    let count2 = call_count.clone();
    dispatcher
        .listen(ModelEvent::Creating, "User", move |_ctx| {
            count2.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;

    // Dispatch event
    let context = EventContext::new(ModelEvent::Creating, "User");
    dispatcher.dispatch(&context).await.unwrap();

    // Both listeners should have been called
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

/// Test 10: Event with model modification
#[tokio::test]
async fn test_event_with_model_modification() {
    #[derive(Clone, Debug)]
    struct User {
        name: String,
        slug: String,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn creating(&mut self) -> EventResult {
            // Auto-generate slug from name
            self.slug = self.name.to_lowercase().replace(" ", "-");
            Ok(())
        }
    }

    let mut user = User {
        name: "John Doe".to_string(),
        slug: String::new(),
    };

    user.creating().await.unwrap();
    assert_eq!(user.slug, "john-doe");
}

/// Test 11: Global event listener
#[tokio::test]
async fn test_global_event_listener() {
    let observer = EventObserver::new();
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();

    // Register global listener for all User creating events
    observer
        .creating("User", move |ctx| {
            called_clone.store(true, Ordering::SeqCst);
            assert_eq!(ctx.model_type, "User");
            Ok(())
        })
        .await;

    // Fire event
    let context = EventContext::new(ModelEvent::Creating, "User");
    observer.fire(context).await.unwrap();

    assert!(called.load(Ordering::SeqCst));
}

/// Test 12: Event with async operations
#[tokio::test]
async fn test_event_with_async_operations() {
    #[derive(Clone, Debug)]
    struct User {
        email: String,
        email_sent: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn created(&self) -> EventResult {
            // Simulate sending welcome email
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            *self.email_sent.lock().await = true;
            Ok(())
        }
    }

    let user = User {
        email: "test@example.com".to_string(),
        email_sent: Arc::new(Mutex::new(false)),
    };

    user.created().await.unwrap();
    assert!(*user.email_sent.lock().await);
}

/// Test 13: EventContext with metadata
#[test]
fn test_event_context_metadata() {
    let context = EventContext::new(ModelEvent::Creating, "User")
        .with_metadata("ip_address", "192.168.1.1")
        .with_metadata("user_agent", "Mozilla/5.0");

    assert_eq!(context.event, ModelEvent::Creating);
    assert_eq!(context.model_type, "User");
    assert_eq!(
        context.get_metadata("ip_address").unwrap(),
        "192.168.1.1"
    );
    assert_eq!(
        context.get_metadata("user_agent").unwrap(),
        "Mozilla/5.0"
    );
    assert!(context.get_metadata("nonexistent").is_none());
}

/// Test 14: EventDispatcher listener count
#[tokio::test]
async fn test_event_dispatcher_listener_count() {
    let dispatcher = EventDispatcher::new();

    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Creating, "User")
            .await,
        0
    );

    dispatcher
        .listen(ModelEvent::Creating, "User", |_| Ok(()))
        .await;
    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Creating, "User")
            .await,
        1
    );

    dispatcher
        .listen(ModelEvent::Creating, "User", |_| Ok(()))
        .await;
    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Creating, "User")
            .await,
        2
    );
}

/// Test 15: EventDispatcher forget and clear
#[tokio::test]
async fn test_event_dispatcher_forget_clear() {
    let dispatcher = EventDispatcher::new();

    dispatcher
        .listen(ModelEvent::Creating, "User", |_| Ok(()))
        .await;
    dispatcher
        .listen(ModelEvent::Created, "User", |_| Ok(()))
        .await;

    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Creating, "User")
            .await,
        1
    );

    dispatcher.forget(ModelEvent::Creating, "User").await;
    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Creating, "User")
            .await,
        0
    );
    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Created, "User")
            .await,
        1
    );

    dispatcher.clear().await;
    assert_eq!(
        dispatcher
            .listener_count(ModelEvent::Created, "User")
            .await,
        0
    );
}

/// Test 16: Saving and Saved events
#[tokio::test]
async fn test_saving_saved_events() {
    #[derive(Clone, Debug)]
    struct User {
        updated_at: Option<chrono::DateTime<Utc>>,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn saving(&mut self) -> EventResult {
            // Set timestamp before any save (create or update)
            self.updated_at = Some(Utc::now());
            Ok(())
        }

        async fn saved(&self) -> EventResult {
            // Log after save
            Ok(())
        }
    }

    let mut user = User { updated_at: None };

    user.saving().await.unwrap();
    assert!(user.updated_at.is_some());

    user.saved().await.unwrap();
}

/// Test 17: Restoring and Restored events
#[tokio::test]
async fn test_restoring_restored_events() {
    #[derive(Clone, Debug)]
    struct User {
        deleted_at: Option<chrono::DateTime<Utc>>,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn restoring(&mut self) -> EventResult {
            // Clear deleted_at timestamp
            self.deleted_at = None;
            Ok(())
        }

        async fn restored(&self) -> EventResult {
            // Send notification, etc.
            Ok(())
        }
    }

    let mut user = User {
        deleted_at: Some(Utc::now()),
    };

    user.restoring().await.unwrap();
    assert!(user.deleted_at.is_none());

    user.restored().await.unwrap();
}

/// Test 18: ModelEvent is_before and is_after
#[test]
fn test_model_event_is_before_after() {
    assert!(ModelEvent::Creating.is_before());
    assert!(ModelEvent::Updating.is_before());
    assert!(ModelEvent::Saving.is_before());
    assert!(ModelEvent::Deleting.is_before());
    assert!(ModelEvent::Restoring.is_before());

    assert!(ModelEvent::Created.is_after());
    assert!(ModelEvent::Updated.is_after());
    assert!(ModelEvent::Saved.is_after());
    assert!(ModelEvent::Deleted.is_after());
    assert!(ModelEvent::Restored.is_after());
}

/// Test 19: ModelEvent name()
#[test]
fn test_model_event_name() {
    assert_eq!(ModelEvent::Creating.name(), "creating");
    assert_eq!(ModelEvent::Created.name(), "created");
    assert_eq!(ModelEvent::Updating.name(), "updating");
    assert_eq!(ModelEvent::Updated.name(), "updated");
    assert_eq!(ModelEvent::Saving.name(), "saving");
    assert_eq!(ModelEvent::Saved.name(), "saved");
    assert_eq!(ModelEvent::Deleting.name(), "deleting");
    assert_eq!(ModelEvent::Deleted.name(), "deleted");
    assert_eq!(ModelEvent::Restoring.name(), "restoring");
    assert_eq!(ModelEvent::Restored.name(), "restored");
}

/// Test 20: EventObserver created and updated helpers
#[tokio::test]
async fn test_event_observer_helpers() {
    let observer = EventObserver::new();
    let created_called = Arc::new(AtomicBool::new(false));
    let updated_called = Arc::new(AtomicBool::new(false));

    let created_clone = created_called.clone();
    observer
        .created("User", move |_| {
            created_clone.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await;

    let updated_clone = updated_called.clone();
    observer
        .updated("User", move |_| {
            updated_clone.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await;

    // Fire created event
    observer
        .fire(EventContext::new(ModelEvent::Created, "User"))
        .await
        .unwrap();
    assert!(created_called.load(Ordering::SeqCst));

    // Fire updated event
    observer
        .fire(EventContext::new(ModelEvent::Updated, "User"))
        .await
        .unwrap();
    assert!(updated_called.load(Ordering::SeqCst));
}

/// Test 21: Event error types
#[test]
fn test_event_error_types() {
    let validation_err = EventError::ValidationFailed("test".to_string());
    assert!(matches!(validation_err, EventError::ValidationFailed(_)));

    let handler_err = EventError::HandlerFailed("test".to_string());
    assert!(matches!(handler_err, EventError::HandlerFailed(_)));

    let propagation_err = EventError::PropagationStopped;
    assert!(matches!(propagation_err, EventError::PropagationStopped));
}

/// Test 22: Multiple events in lifecycle
#[tokio::test]
async fn test_multiple_events_lifecycle() {
    let lifecycle = Arc::new(Mutex::new(Vec::<String>::new()));

    #[derive(Clone)]
    struct User {
        id: i32,
        lifecycle: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ModelEvents for User {
        async fn creating(&mut self) -> EventResult {
            self.lifecycle.lock().await.push("creating".to_string());
            Ok(())
        }

        async fn created(&self) -> EventResult {
            self.lifecycle.lock().await.push("created".to_string());
            Ok(())
        }

        async fn updating(&mut self) -> EventResult {
            self.lifecycle.lock().await.push("updating".to_string());
            Ok(())
        }

        async fn updated(&self) -> EventResult {
            self.lifecycle.lock().await.push("updated".to_string());
            Ok(())
        }
    }

    let mut user = User {
        id: 1,
        lifecycle: lifecycle.clone(),
    };

    // Simulate create lifecycle
    user.creating().await.unwrap();
    user.created().await.unwrap();

    // Simulate update lifecycle
    user.updating().await.unwrap();
    user.updated().await.unwrap();

    let events = lifecycle.lock().await;
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], "creating");
    assert_eq!(events[1], "created");
    assert_eq!(events[2], "updating");
    assert_eq!(events[3], "updated");
}
