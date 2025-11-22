//! Integration tests for QueueFake

use rf_testing::fakes::{queue::JobRecord, QueueFake};
use serde_json::json;

#[test]
fn test_queue_fake_basic_usage() {
    let fake = QueueFake::new();

    // Initially empty
    assert_eq!(fake.count(), 0);
    fake.assert_nothing_pushed();

    // Push a job
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

    assert_eq!(fake.count(), 1);
    fake.assert_pushed("send_email");
}

#[test]
fn test_queue_fake_multiple_jobs() {
    let fake = QueueFake::new();

    // Push multiple jobs
    for i in 0..5 {
        fake.record_push(JobRecord {
            job_type: "send_email".to_string(),
            payload: json!({"to": format!("user{}@example.com", i)}),
            queue: "default".to_string(),
            job_id: i.to_string(),
            priority: 0,
        });
    }

    fake.assert_pushed_times("send_email", 5);
    assert_eq!(fake.count_of_type("send_email"), 5);
}

#[test]
fn test_queue_fake_different_job_types() {
    let fake = QueueFake::new();

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({}),
        queue: "default".to_string(),
        job_id: "1".to_string(),
        priority: 0,
    });

    fake.record_push(JobRecord {
        job_type: "process_payment".to_string(),
        payload: json!({}),
        queue: "default".to_string(),
        job_id: "2".to_string(),
        priority: 0,
    });

    fake.assert_pushed("send_email");
    fake.assert_pushed("process_payment");
    fake.assert_pushed_times("send_email", 1);
    fake.assert_pushed_times("process_payment", 1);
}

#[test]
fn test_queue_fake_assert_pushed_on() {
    let fake = QueueFake::new();

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({}),
        queue: "emails".to_string(),
        job_id: "1".to_string(),
        priority: 0,
    });

    fake.assert_pushed_on("send_email", "emails");
}

#[test]
#[should_panic(expected = "Failed asserting that job 'missing_job' was pushed")]
fn test_queue_fake_assert_pushed_fails() {
    let fake = QueueFake::new();
    fake.assert_pushed("missing_job");
}

#[test]
#[should_panic(expected = "Failed asserting that job 'send_email' was not pushed")]
fn test_queue_fake_assert_not_pushed_fails() {
    let fake = QueueFake::new();

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({}),
        queue: "default".to_string(),
        job_id: "1".to_string(),
        priority: 0,
    });

    fake.assert_not_pushed("send_email");
}

#[test]
fn test_queue_fake_assert_pushed_with() {
    let fake = QueueFake::new();

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({
            "to": "test@example.com",
            "subject": "Hello"
        }),
        queue: "default".to_string(),
        job_id: "1".to_string(),
        priority: 0,
    });

    fake.assert_pushed_with("send_email", |payload| payload["to"] == "test@example.com");
}

#[test]
fn test_queue_fake_clear() {
    let fake = QueueFake::new();

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({}),
        queue: "default".to_string(),
        job_id: "1".to_string(),
        priority: 0,
    });

    assert_eq!(fake.count(), 1);
    fake.clear();
    assert_eq!(fake.count(), 0);
    fake.assert_nothing_pushed();
}

#[test]
fn test_queue_fake_pushed_jobs_of_type() {
    let fake = QueueFake::new();

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({ "to": "user1@example.com" }),
        queue: "default".to_string(),
        job_id: "1".to_string(),
        priority: 0,
    });

    fake.record_push(JobRecord {
        job_type: "process_payment".to_string(),
        payload: json!({}),
        queue: "default".to_string(),
        job_id: "2".to_string(),
        priority: 0,
    });

    fake.record_push(JobRecord {
        job_type: "send_email".to_string(),
        payload: json!({ "to": "user2@example.com" }),
        queue: "default".to_string(),
        job_id: "3".to_string(),
        priority: 0,
    });

    let emails = fake.pushed_jobs_of_type("send_email");
    assert_eq!(emails.len(), 2);

    let payments = fake.pushed_jobs_of_type("process_payment");
    assert_eq!(payments.len(), 1);
}
