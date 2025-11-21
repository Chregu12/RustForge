//! Integration tests for EventFake

use rf_testing::fakes::EventFake;
use serde_json::json;

#[test]
fn test_event_fake_basic_usage() {
    let fake = EventFake::new();

    // Initially empty
    assert_eq!(fake.count(), 0);
    fake.assert_nothing_dispatched();

    // Dispatch an event
    fake.dispatch_simple("UserCreated", json!({
        "user_id": 1,
        "email": "test@example.com"
    }));

    assert_eq!(fake.count(), 1);
    fake.assert_dispatched("UserCreated");
}

#[test]
fn test_event_fake_multiple_events() {
    let fake = EventFake::new();

    // Dispatch multiple events
    for i in 0..5 {
        fake.dispatch_simple("UserCreated", json!({
            "user_id": i,
            "email": format!("user{}@example.com", i)
        }));
    }

    fake.assert_dispatched_times("UserCreated", 5);
    assert_eq!(fake.dispatch_count("UserCreated"), 5);
}

#[test]
fn test_event_fake_different_event_types() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({ "user_id": 1 }));
    fake.dispatch_simple("EmailSent", json!({ "to": "test@example.com" }));
    fake.dispatch_simple("OrderPlaced", json!({ "order_id": 123 }));

    fake.assert_dispatched("UserCreated");
    fake.assert_dispatched("EmailSent");
    fake.assert_dispatched("OrderPlaced");

    fake.assert_dispatched_times("UserCreated", 1);
    fake.assert_dispatched_times("EmailSent", 1);
    fake.assert_dispatched_times("OrderPlaced", 1);
}

#[test]
#[should_panic(expected = "Failed asserting that event 'MissingEvent' was dispatched")]
fn test_event_fake_assert_dispatched_fails() {
    let fake = EventFake::new();
    fake.assert_dispatched("MissingEvent");
}

#[test]
#[should_panic(expected = "Failed asserting that event 'UserCreated' was not dispatched")]
fn test_event_fake_assert_not_dispatched_fails() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({}));
    fake.assert_not_dispatched("UserCreated");
}

#[test]
fn test_event_fake_assert_dispatched_with() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({
        "user_id": 1,
        "email": "test@example.com"
    }));

    fake.assert_dispatched_with("UserCreated", |payload| {
        payload["user_id"] == 1 && payload["email"] == "test@example.com"
    });
}

#[test]
fn test_event_fake_clear() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({}));

    assert_eq!(fake.count(), 1);
    fake.clear();
    assert_eq!(fake.count(), 0);
    fake.assert_nothing_dispatched();
}

#[test]
fn test_event_fake_dispatched_events_of_type() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({ "user_id": 1 }));
    fake.dispatch_simple("EmailSent", json!({ "to": "user1@example.com" }));
    fake.dispatch_simple("UserCreated", json!({ "user_id": 2 }));

    let user_events = fake.dispatched_events_of_type("UserCreated");
    assert_eq!(user_events.len(), 2);

    let email_events = fake.dispatched_events_of_type("EmailSent");
    assert_eq!(email_events.len(), 1);
}

#[test]
fn test_event_fake_assert_dispatched_in_order() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({}));
    fake.dispatch_simple("EmailSent", json!({}));
    fake.dispatch_simple("WelcomeEmailSent", json!({}));

    // Should pass - events dispatched in this order
    fake.assert_dispatched_in_order(&["UserCreated", "EmailSent", "WelcomeEmailSent"]);
}

#[test]
#[should_panic(expected = "Failed asserting events were dispatched in order")]
fn test_event_fake_assert_dispatched_in_order_fails() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({}));
    fake.dispatch_simple("WelcomeEmailSent", json!({}));

    // Should fail - EmailSent was never dispatched
    fake.assert_dispatched_in_order(&["UserCreated", "EmailSent", "WelcomeEmailSent"]);
}

#[test]
fn test_event_fake_assert_nothing_dispatched() {
    let fake = EventFake::new();
    fake.assert_nothing_dispatched();
}

#[test]
#[should_panic(expected = "Failed asserting that no events were dispatched")]
fn test_event_fake_assert_nothing_dispatched_fails() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({}));
    fake.assert_nothing_dispatched();
}

#[test]
fn test_event_fake_dispatch_count() {
    let fake = EventFake::new();

    fake.dispatch_simple("UserCreated", json!({}));
    fake.dispatch_simple("UserCreated", json!({}));
    fake.dispatch_simple("EmailSent", json!({}));

    assert_eq!(fake.dispatch_count("UserCreated"), 2);
    assert_eq!(fake.dispatch_count("EmailSent"), 1);
    assert_eq!(fake.dispatch_count("NonExistent"), 0);
}
