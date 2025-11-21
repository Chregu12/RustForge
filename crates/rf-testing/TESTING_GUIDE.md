# RustForge Testing Utilities Guide

Comprehensive testing utilities for Laravel-style developer experience in Rust.

## Table of Contents

- [Database Assertions](#database-assertions)
- [Queue Fakes](#queue-fakes)
- [Event Fakes](#event-fakes)
- [Best Practices](#best-practices)

---

## Database Assertions

Laravel-style database assertions for elegant testing.

### Basic Usage

```rust
use rf_testing::{assert_database_has, assert_database_missing, assert_database_count, assert_database_empty};

#[tokio::test]
async fn test_user_creation() {
    let db = setup_test_db().await;

    // Create a user
    create_user(&db, "test@example.com").await?;

    // Assert user exists
    assert_database_has!("users", {
        "email" => "test@example.com",
        "active" => true
    });

    // Assert count
    assert_database_count!("users", 1);
}
```

### Available Assertions

#### `assert_database_has!`
Assert that a record exists matching the given conditions.

```rust
assert_database_has!("users", {
    "email" => "test@example.com",
    "name" => "Test User"
});
```

#### `assert_database_missing!`
Assert that NO record exists matching the given conditions.

```rust
// Delete user
delete_user(&db, "test@example.com").await?;

// Assert user was deleted
assert_database_missing!("users", {
    "email" => "test@example.com"
});
```

#### `assert_database_count!`
Assert exact row count in a table.

```rust
assert_database_count!("users", 10);
```

#### `assert_database_empty!`
Assert that a table is empty.

```rust
truncate_table(&db, "users").await?;

assert_database_empty!("users");
```

### Multiple Conditions

You can assert multiple fields at once:

```rust
assert_database_has!("posts", {
    "title" => "My Post",
    "published" => true,
    "views" => 100,
    "author_id" => 1
});
```

---

## Queue Fakes

Test job dispatching without actually processing jobs.

### Basic Usage

```rust
use rf_testing::fakes::{QueueFake, queue::JobRecord};
use serde_json::json;

#[test]
fn test_job_dispatching() {
    let fake = QueueFake::new();

    // Dispatch a job (using the record_push helper for testing)
    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({
            "to": "test@example.com",
            "subject": "Hello"
        }),
        queue: "default".to_string(),
        job_id: "123".to_string(),
        priority: 0,
    });

    // Assert
    fake.assert_pushed("send_email");
    fake.assert_pushed_times("send_email", 1);
}
```

### Available Assertions

#### `assert_pushed()`
Assert that a job of the given type was pushed.

```rust
fake.assert_pushed("send_email");
```

#### `assert_pushed_times()`
Assert exact number of dispatches.

```rust
fake.assert_pushed_times("send_email", 3);
```

#### `assert_pushed_on()`
Assert job was pushed to a specific queue.

```rust
fake.assert_pushed_on("send_email", "emails");
```

#### `assert_not_pushed()`
Assert a job was NOT pushed.

```rust
fake.assert_not_pushed("process_payment");
```

#### `assert_nothing_pushed()`
Assert NO jobs were pushed at all.

```rust
fake.assert_nothing_pushed();
```

#### `assert_pushed_with()`
Assert job was pushed with specific payload values.

```rust
fake.assert_pushed_with("send_email", |payload| {
    payload["to"] == "test@example.com"
});
```

### Inspecting Jobs

Get the pushed jobs for inspection:

```rust
// Get all jobs of a type
let jobs = fake.pushed_jobs_of_type("send_email");
assert_eq!(jobs.len(), 2);

// Get total count
assert_eq!(fake.count(), 5);

// Count by type
assert_eq!(fake.count_of_type("send_email"), 2);

// Clear all jobs
fake.clear();
```

---

## Event Fakes

Test event dispatching and listeners.

### Basic Usage

```rust
use rf_testing::fakes::EventFake;
use serde_json::json;

#[test]
fn test_event_dispatching() {
    let fake = EventFake::new();

    // Dispatch events
    fake.dispatch_simple("UserCreated", json!({
        "user_id": 1,
        "email": "test@example.com"
    }));

    // Assert
    fake.assert_dispatched("UserCreated");
    fake.assert_dispatched_times("UserCreated", 1);
}
```

### Available Assertions

#### `assert_dispatched()`
Assert that an event was dispatched.

```rust
fake.assert_dispatched("UserCreated");
```

#### `assert_dispatched_times()`
Assert exact number of dispatches.

```rust
fake.assert_dispatched_times("UserCreated", 3);
```

#### `assert_not_dispatched()`
Assert an event was NOT dispatched.

```rust
fake.assert_not_dispatched("UserDeleted");
```

#### `assert_nothing_dispatched()`
Assert NO events were dispatched at all.

```rust
fake.assert_nothing_dispatched();
```

#### `assert_dispatched_with()`
Assert event was dispatched with specific payload values.

```rust
fake.assert_dispatched_with("UserCreated", |payload| {
    payload["user_id"] == 1
});
```

#### `assert_dispatched_in_order()`
Assert events were dispatched in a specific order.

```rust
fake.dispatch_simple("UserCreated", json!({}));
fake.dispatch_simple("EmailSent", json!({}));
fake.dispatch_simple("WelcomeEmailSent", json!({}));

fake.assert_dispatched_in_order(&[
    "UserCreated",
    "EmailSent",
    "WelcomeEmailSent"
]);
```

### Inspecting Events

Get dispatched events for inspection:

```rust
// Get all events of a type
let events = fake.dispatched_events_of_type("UserCreated");
assert_eq!(events.len(), 2);

// Get total count
assert_eq!(fake.count(), 5);

// Count by type
assert_eq!(fake.dispatch_count("UserCreated"), 2);

// Clear all events
fake.clear();
```

---

## Best Practices

### 1. Use Descriptive Test Names

```rust
#[tokio::test]
async fn test_user_registration_sends_welcome_email() {
    // Clear test
}
```

### 2. Arrange-Act-Assert Pattern

```rust
#[tokio::test]
async fn test_job_dispatching() {
    // Arrange
    let fake = QueueFake::new();

    // Act
    dispatch_welcome_email(&fake, "test@example.com").await?;

    // Assert
    fake.assert_pushed("send_email");
    fake.assert_pushed_with("send_email", |job| {
        job["to"] == "test@example.com"
    });
}
```

### 3. Test One Thing at a Time

```rust
#[tokio::test]
async fn test_user_created_event_dispatched() {
    let fake = EventFake::new();

    create_user(&fake, "test@example.com").await?;

    // Test ONLY that the event was dispatched
    fake.assert_dispatched("UserCreated");
}

#[tokio::test]
async fn test_user_created_event_has_correct_data() {
    let fake = EventFake::new();

    create_user(&fake, "test@example.com").await?;

    // Test ONLY the event data
    fake.assert_dispatched_with("UserCreated", |event| {
        event["email"] == "test@example.com"
    });
}
```

### 4. Clean Up Between Tests

```rust
#[tokio::test]
async fn test_multiple_operations() {
    let fake = QueueFake::new();

    // First operation
    dispatch_job(&fake, "job1").await?;
    fake.assert_pushed("job1");

    // Clear for next operation
    fake.clear();

    // Second operation
    dispatch_job(&fake, "job2").await?;
    fake.assert_pushed("job2");
    fake.assert_not_pushed("job1"); // Verifies clear worked
}
```

### 5. Combine Assertions

```rust
#[tokio::test]
async fn test_complete_user_registration_flow() {
    let db = setup_test_db().await;
    let queue_fake = QueueFake::new();
    let event_fake = EventFake::new();

    // Act
    register_user(&db, &queue_fake, &event_fake, "test@example.com").await?;

    // Assert database
    assert_database_has!("users", {
        "email" => "test@example.com",
        "verified" => false
    });

    // Assert job
    queue_fake.assert_pushed("send_verification_email");

    // Assert event
    event_fake.assert_dispatched("UserRegistered");
}
```

---

## Integration with Real Components

### With rf-queue

```rust
// In production code
use rf_queue::{Queue, JobMetadata};

async fn dispatch_email(queue: &impl Queue, to: &str) -> Result<()> {
    let job = SendEmailJob { to: to.to_string() };
    let metadata = JobMetadata::new(&job)?;
    queue.push(metadata).await?;
    Ok(())
}

// In tests
#[test]
fn test_email_dispatching() {
    let fake = QueueFake::new();

    dispatch_email(&fake, "test@example.com").await?;

    fake.assert_pushed("send_email");
}
```

### With rf-events

```rust
// In production code
use rf_events::{EventDispatcher, Event};

async fn create_user(events: &EventDispatcher, email: &str) -> Result<()> {
    // ... create user logic ...

    events.dispatch(UserCreated {
        email: email.to_string()
    }).await?;

    Ok(())
}

// In tests
#[test]
fn test_user_creation_event() {
    let fake = EventFake::new();

    create_user(&fake, "test@example.com").await?;

    fake.assert_dispatched("UserCreated");
}
```

---

## Example Test Suite

Complete example combining all utilities:

```rust
use rf_testing::{
    assert_database_has,
    fakes::{QueueFake, EventFake},
};

#[tokio::test]
async fn test_complete_order_flow() {
    // Setup
    let db = setup_test_db().await;
    let queue = QueueFake::new();
    let events = EventFake::new();

    // Act: Create order
    let order_id = create_order(
        &db,
        &queue,
        &events,
        "customer@example.com",
        vec!["item1", "item2"]
    ).await?;

    // Assert: Database
    assert_database_has!("orders", {
        "id" => order_id,
        "customer_email" => "customer@example.com",
        "status" => "pending"
    });

    assert_database_count!("order_items", 2);

    // Assert: Jobs
    queue.assert_pushed("send_order_confirmation");
    queue.assert_pushed("process_payment");
    queue.assert_pushed_times("send_order_confirmation", 1);

    // Assert: Events
    events.assert_dispatched("OrderCreated");
    events.assert_dispatched_with("OrderCreated", |event| {
        event["order_id"] == order_id
    });

    // Assert: Event order
    events.assert_dispatched_in_order(&[
        "OrderCreated",
        "PaymentInitiated"
    ]);
}
```

---

## Tips & Tricks

### 1. Debugging Failed Assertions

```rust
// Get all pushed jobs to see what was actually dispatched
let jobs = fake.pushed_jobs();
println!("Pushed jobs: {:?}", jobs);

// Get all dispatched events
let events = fake.dispatched_events();
println!("Dispatched events: {:?}", events);
```

### 2. Custom Assertions

Create helper functions for common assertions:

```rust
fn assert_email_sent(fake: &QueueFake, to: &str, subject: &str) {
    fake.assert_pushed_with("send_email", |payload| {
        payload["to"] == to && payload["subject"] == subject
    });
}

// Usage
assert_email_sent(&fake, "test@example.com", "Welcome!");
```

### 3. Test Fixtures

```rust
fn create_queue_with_jobs() -> QueueFake {
    let fake = QueueFake::new();

    for i in 0..5 {
        fake.record_push(JobRecord {
            job_type: "test_job".to_string(),
            payload: json!({ "id": i }),
            queue: "default".to_string(),
            job_id: i.to_string(),
            priority: 0,
        });
    }

    fake
}
```

---

## Troubleshooting

### Macros Not Found

Make sure to import at the crate root:

```rust
use rf_testing::{assert_database_has, assert_database_missing};
```

### Async Issues

All database assertions are async and must be awaited:

```rust
// ✅ Correct - macros already include .await
let result = assert_database_has!("users", {
    "email" => "test@example.com"
});

// ❌ Wrong - don't add .await again
let result = assert_database_has!("users", {
    "email" => "test@example.com"
}).await; // Error!
```

### Fake Not Recording

Make sure you're using the fake instance throughout your test:

```rust
let fake = QueueFake::new();

// ✅ Correct
dispatch_job(&fake, "test").await?;
fake.assert_pushed("test");

// ❌ Wrong - different instance!
let another_fake = QueueFake::new();
dispatch_job(&fake, "test").await?;
another_fake.assert_pushed("test"); // Will fail!
```

---

For more examples, see the tests in `crates/rf-testing/tests/`.
