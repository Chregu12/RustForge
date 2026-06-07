use rf_queue::routing::{QueueRoute, QueueRouter};
use std::time::Duration;

// ---------- QueueRoute ----------

#[test]
fn test_route_new_sets_connection_and_queue() {
    let r = QueueRoute::new("redis", "emails");
    assert_eq!(r.connection, "redis");
    assert_eq!(r.queue, "emails");
}

#[test]
fn test_route_delay_none_by_default() {
    let r = QueueRoute::new("redis", "emails");
    assert!(r.delay.is_none());
}

#[test]
fn test_route_max_tries_none_by_default() {
    let r = QueueRoute::new("redis", "emails");
    assert!(r.max_tries.is_none());
}

#[test]
fn test_route_timeout_none_by_default() {
    let r = QueueRoute::new("redis", "emails");
    assert!(r.timeout.is_none());
}

#[test]
fn test_route_with_delay() {
    let r = QueueRoute::new("redis", "emails").delay(Duration::from_secs(60));
    assert_eq!(r.delay, Some(Duration::from_secs(60)));
}

#[test]
fn test_route_with_max_tries() {
    let r = QueueRoute::new("redis", "emails").max_tries(5);
    assert_eq!(r.max_tries, Some(5));
}

#[test]
fn test_route_with_timeout() {
    let r = QueueRoute::new("redis", "emails").timeout(Duration::from_secs(30));
    assert_eq!(r.timeout, Some(Duration::from_secs(30)));
}

#[test]
fn test_route_builder_chaining() {
    let r = QueueRoute::new("sqs", "notifications")
        .delay(Duration::from_secs(10))
        .max_tries(3)
        .timeout(Duration::from_secs(120));
    assert_eq!(r.connection, "sqs");
    assert_eq!(r.queue, "notifications");
    assert_eq!(r.delay, Some(Duration::from_secs(10)));
    assert_eq!(r.max_tries, Some(3));
    assert_eq!(r.timeout, Some(Duration::from_secs(120)));
}

// ---------- QueueRouter ----------

#[test]
fn test_router_new_has_no_routes() {
    let router = QueueRouter::new();
    assert_eq!(router.routes().len(), 0);
}

#[test]
fn test_router_register_route() {
    let mut router = QueueRouter::new();
    router.route("send_email", QueueRoute::new("redis", "emails"));
    assert_eq!(router.routes().len(), 1);
    assert!(router.has_route("send_email"));
}

#[test]
fn test_router_resolve_registered_job() {
    let mut router = QueueRouter::new();
    router.route("send_email", QueueRoute::new("redis", "emails"));
    let route = router.resolve("send_email");
    assert_eq!(route.connection, "redis");
    assert_eq!(route.queue, "emails");
}

#[test]
fn test_router_resolve_unregistered_returns_default() {
    let router = QueueRouter::new();
    let route = router.resolve("unknown_job");
    assert_eq!(route.connection, "default");
    assert_eq!(route.queue, "default");
}

#[test]
fn test_router_has_route_true_for_registered() {
    let mut router = QueueRouter::new();
    router.route("process_payment", QueueRoute::new("redis", "payments"));
    assert!(router.has_route("process_payment"));
}

#[test]
fn test_router_has_route_false_for_unregistered() {
    let router = QueueRouter::new();
    assert!(!router.has_route("nonexistent_job"));
}

#[test]
fn test_router_default_connection_override() {
    let router = QueueRouter::new().default_connection("redis");
    let route = router.resolve("unknown_job");
    assert_eq!(route.connection, "redis");
}

#[test]
fn test_router_default_queue_override() {
    let router = QueueRouter::new().default_queue("high-priority");
    let route = router.resolve("unknown_job");
    assert_eq!(route.queue, "high-priority");
}

#[test]
fn test_router_multiple_routes() {
    let mut router = QueueRouter::new();
    router.route("send_email", QueueRoute::new("redis", "emails"));
    router.route("process_payment", QueueRoute::new("sqs", "payments"));
    router.route("generate_report", QueueRoute::new("database", "reports"));
    assert_eq!(router.routes().len(), 3);
}

#[test]
fn test_router_resolve_preserves_route_delay() {
    let mut router = QueueRouter::new();
    router.route(
        "delayed_job",
        QueueRoute::new("redis", "slow").delay(Duration::from_secs(300)),
    );
    let route = router.resolve("delayed_job");
    assert_eq!(route.delay, Some(Duration::from_secs(300)));
}

#[test]
fn test_router_resolve_preserves_max_tries() {
    let mut router = QueueRouter::new();
    router.route(
        "retry_job",
        QueueRoute::new("redis", "retries").max_tries(10),
    );
    let route = router.resolve("retry_job");
    assert_eq!(route.max_tries, Some(10));
}

#[test]
fn test_router_default_both_overridden() {
    let router = QueueRouter::new()
        .default_connection("sqs")
        .default_queue("fallback");
    let route = router.resolve("anything");
    assert_eq!(route.connection, "sqs");
    assert_eq!(route.queue, "fallback");
}
