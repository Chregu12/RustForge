//! Model Events Usage Examples
//!
//! Demonstrates how to use the model events system in RustForge.
//! This shows Laravel-equivalent model lifecycle events.

use async_trait::async_trait;
use chrono::Utc;
use rf_eloquent::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Example 1: Basic Model Events
#[derive(Clone, Debug)]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub slug: String,
    pub created_at: Option<chrono::DateTime<Utc>>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

#[async_trait]
impl ModelEvents for User {
    /// Called before creating a new user
    async fn creating(&mut self) -> EventResult {
        println!("Creating event: Setting timestamps and generating slug");

        // Auto-generate slug from name
        self.slug = self.name.to_lowercase().replace(" ", "-");

        // Set created_at timestamp
        self.created_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    /// Called after user is created
    async fn created(&self) -> EventResult {
        println!("Created event: User {} created successfully", self.name);

        // In real app: Send welcome email, create profile, etc.
        // send_welcome_email(&self.email).await?;

        Ok(())
    }

    /// Called before updating a user
    async fn updating(&mut self) -> EventResult {
        println!("Updating event: Validating and updating timestamp");

        // Validate email
        if !self.email.contains('@') {
            return Err(EventError::ValidationFailed(
                "Invalid email format".to_string(),
            ));
        }

        // Update timestamp
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    /// Called after user is updated
    async fn updated(&self) -> EventResult {
        println!("Updated event: User {} updated", self.name);

        // In real app: Invalidate cache, log change, etc.
        // cache::forget(&format!("user:{}", self.id)).await?;

        Ok(())
    }

    /// Called before deleting a user
    async fn deleting(&mut self) -> EventResult {
        println!("Deleting event: Checking if deletion is allowed");

        // In real app: Check if user has active subscriptions, etc.

        Ok(())
    }

    /// Called after user is deleted
    async fn deleted(&self) -> EventResult {
        println!("Deleted event: User {} deleted", self.name);

        // In real app: Clean up related data, send notifications, etc.

        Ok(())
    }
}

/// Example 2: Event Validation
#[derive(Clone, Debug)]
struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub view_count: i32,
}

#[async_trait]
impl ModelEvents for Post {
    async fn creating(&mut self) -> EventResult {
        // Validate title length
        if self.title.len() < 3 {
            return Err(EventError::ValidationFailed(
                "Title must be at least 3 characters".to_string(),
            ));
        }

        // Validate content
        if self.content.is_empty() {
            return Err(EventError::ValidationFailed(
                "Content cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    async fn updating(&mut self) -> EventResult {
        // Prevent modification of popular posts
        if self.published && self.view_count > 1000 {
            return Err(EventError::ValidationFailed(
                "Cannot modify popular published posts".to_string(),
            ));
        }

        Ok(())
    }
}

/// Example 3: Event Dispatcher
async fn example_event_dispatcher() -> Result<(), EventError> {
    println!("\n=== Event Dispatcher Example ===");

    let dispatcher = EventDispatcher::new();

    // Register listeners
    dispatcher
        .listen(ModelEvent::Creating, "User", |ctx| {
            println!("Listener 1: User creating at {:?}", ctx.timestamp);
            Ok(())
        })
        .await;

    dispatcher
        .listen(ModelEvent::Creating, "User", |ctx| {
            println!("Listener 2: User creating for {}", ctx.model_type);
            Ok(())
        })
        .await;

    // Dispatch event
    let context = EventContext::new(ModelEvent::Creating, "User")
        .with_metadata("user_id", "123")
        .with_metadata("ip_address", "192.168.1.1");

    dispatcher.dispatch(&context).await?;

    println!(
        "Total listeners: {}",
        dispatcher
            .listener_count(ModelEvent::Creating, "User")
            .await
    );

    Ok(())
}

/// Example 4: Event Observer
async fn example_event_observer() -> Result<(), EventError> {
    println!("\n=== Event Observer Example ===");

    let observer = EventObserver::new();

    // Register observers for different events
    observer
        .creating("User", |ctx| {
            println!("Observer: User being created");
            if let Some(id) = ctx.get_metadata("user_id") {
                println!("  User ID: {}", id);
            }
            Ok(())
        })
        .await;

    observer
        .created("User", |ctx| {
            println!("Observer: User created successfully");
            Ok(())
        })
        .await;

    observer
        .updating("User", |ctx| {
            println!("Observer: User being updated");
            Ok(())
        })
        .await;

    observer
        .updated("User", |ctx| {
            println!("Observer: User updated successfully");
            Ok(())
        })
        .await;

    // Fire events
    observer
        .fire(EventContext::new(ModelEvent::Creating, "User").with_metadata("user_id", "456"))
        .await?;

    observer
        .fire(EventContext::new(ModelEvent::Created, "User"))
        .await?;

    Ok(())
}

/// Example 5: Lifecycle Tracking
#[derive(Clone)]
struct Order {
    pub id: i32,
    pub status: String,
    pub lifecycle: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ModelEvents for Order {
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

    async fn deleting(&mut self) -> EventResult {
        self.lifecycle.lock().await.push("deleting".to_string());
        Ok(())
    }

    async fn deleted(&self) -> EventResult {
        self.lifecycle.lock().await.push("deleted".to_string());
        Ok(())
    }
}

async fn example_lifecycle_tracking() -> Result<(), EventError> {
    println!("\n=== Lifecycle Tracking Example ===");

    let lifecycle = Arc::new(Mutex::new(Vec::new()));
    let mut order = Order {
        id: 1,
        status: "pending".to_string(),
        lifecycle: lifecycle.clone(),
    };

    // Simulate create lifecycle
    order.creating().await?;
    order.created().await?;

    // Simulate update lifecycle
    order.updating().await?;
    order.updated().await?;

    // Simulate delete lifecycle
    order.deleting().await?;
    order.deleted().await?;

    let events = lifecycle.lock().await;
    println!("Lifecycle events: {:?}", *events);

    Ok(())
}

/// Example 6: Soft Delete Events
#[derive(Clone, Debug)]
struct Article {
    pub id: i32,
    pub title: String,
    pub deleted_at: Option<chrono::DateTime<Utc>>,
}

#[async_trait]
impl ModelEvents for Article {
    async fn deleting(&mut self) -> EventResult {
        println!("Soft deleting article: {}", self.title);
        self.deleted_at = Some(Utc::now());
        Ok(())
    }

    async fn restoring(&mut self) -> EventResult {
        println!("Restoring article: {}", self.title);
        self.deleted_at = None;
        Ok(())
    }

    async fn restored(&self) -> EventResult {
        println!("Article restored: {}", self.title);
        // Send notification, update search index, etc.
        Ok(())
    }
}

async fn example_soft_delete_events() -> Result<(), EventError> {
    println!("\n=== Soft Delete Events Example ===");

    let mut article = Article {
        id: 1,
        title: "Test Article".to_string(),
        deleted_at: None,
    };

    // Soft delete
    article.deleting().await?;
    println!("Article deleted at: {:?}", article.deleted_at);

    // Restore
    article.restoring().await?;
    article.restored().await?;
    println!("Article deleted at after restore: {:?}", article.deleted_at);

    Ok(())
}

/// Example 7: Event Context Usage
async fn example_event_context() -> Result<(), EventError> {
    println!("\n=== Event Context Example ===");

    let context = EventContext::new(ModelEvent::Creating, "User")
        .with_metadata("user_agent", "Mozilla/5.0")
        .with_metadata("ip_address", "192.168.1.1")
        .with_metadata("action", "registration");

    println!("Event: {}", context.event.name());
    println!("Model: {}", context.model_type);
    println!("Timestamp: {}", context.timestamp);
    println!("Metadata:");
    for (key, value) in &context.metadata {
        println!("  {}: {}", key, value);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Model Events Usage Examples");
    println!("===========================\n");

    // Example 1: Basic user with events
    println!("=== Basic Model Events ===");
    let mut user = User {
        id: 0,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        slug: String::new(),
        created_at: None,
        updated_at: None,
    };

    user.creating().await?;
    println!("After creating: slug = {}", user.slug);

    user.created().await?;

    // Update user
    user.email = "john.doe@example.com".to_string();
    user.updating().await?;
    user.updated().await?;

    // Example 2: Validation
    println!("\n=== Event Validation ===");
    let mut invalid_post = Post {
        id: 0,
        title: "Ab".to_string(), // Too short
        content: String::new(),  // Empty
        published: false,
        view_count: 0,
    };

    match invalid_post.creating().await {
        Err(EventError::ValidationFailed(msg)) => {
            println!("Validation failed: {}", msg);
        }
        _ => println!("Unexpected result"),
    }

    // Run other examples
    example_event_dispatcher().await?;
    example_event_observer().await?;
    example_lifecycle_tracking().await?;
    example_soft_delete_events().await?;
    example_event_context().await?;

    println!("\n=== All examples completed successfully! ===");

    Ok(())
}
