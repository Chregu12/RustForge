//! Integration tests for rf-ratelimit

use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimiter};
use std::time::Duration;

// ── Config ────────────────────────────────────────────────────────────────────

#[test]
fn config_per_minute_sets_correct_window() {
    let cfg = RateLimitConfig::per_minute(100);
    assert_eq!(cfg.max_requests, 100);
    assert_eq!(cfg.window, Duration::from_secs(60));
}

#[test]
fn config_per_hour_sets_correct_window() {
    let cfg = RateLimitConfig::per_hour(1000);
    assert_eq!(cfg.max_requests, 1000);
    assert_eq!(cfg.window, Duration::from_secs(3600));
}

#[test]
fn config_per_second_sets_correct_window() {
    let cfg = RateLimitConfig::per_second(10);
    assert_eq!(cfg.max_requests, 10);
    assert_eq!(cfg.window, Duration::from_secs(1));
}

#[test]
fn config_custom_window_and_limit() {
    let cfg = RateLimitConfig::custom(25, Duration::from_secs(30));
    assert_eq!(cfg.max_requests, 25);
    assert_eq!(cfg.window, Duration::from_secs(30));
}

#[test]
fn config_with_prefix_changes_key_prefix() {
    let cfg = RateLimitConfig::per_minute(10).with_prefix("api");
    assert_eq!(cfg.key_prefix, "api");
}

#[test]
fn config_default_is_60_per_minute() {
    let cfg = RateLimitConfig::default();
    assert_eq!(cfg.max_requests, 60);
    assert_eq!(cfg.window, Duration::from_secs(60));
}

// ── Counter increments ────────────────────────────────────────────────────────

#[tokio::test]
async fn counter_increments_on_each_allowed_request() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(5));

    let r1 = limiter.check("user:1").await.unwrap();
    assert!(r1.allowed);
    assert_eq!(r1.remaining, 4);

    let r2 = limiter.check("user:1").await.unwrap();
    assert!(r2.allowed);
    assert_eq!(r2.remaining, 3);
}

#[tokio::test]
async fn request_allowed_up_to_limit() {
    let cfg = RateLimitConfig::per_minute(3);
    let limiter = MemoryRateLimiter::new(cfg);

    for i in 0..3 {
        let result = limiter.check("key").await.unwrap();
        assert!(result.allowed, "request {} should be allowed", i + 1);
    }
}

#[tokio::test]
async fn request_rejected_when_limit_exceeded() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(3));

    for _ in 0..3 {
        limiter.check("key").await.unwrap();
    }

    let result = limiter.check("key").await.unwrap();
    assert!(!result.allowed);
    assert_eq!(result.remaining, 0);
    assert!(result.retry_after.is_some());
}

#[tokio::test]
async fn remaining_is_zero_when_at_limit() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(2));

    let _ = limiter.check("k").await.unwrap();
    let last_allowed = limiter.check("k").await.unwrap();

    assert!(last_allowed.allowed);
    assert_eq!(last_allowed.remaining, 0);
}

#[tokio::test]
async fn limit_field_matches_configured_max() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(42));
    let result = limiter.check("test").await.unwrap();
    assert_eq!(result.limit, 42);
}

// ── Reset ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn reset_clears_counter_so_requests_are_allowed_again() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(2));

    for _ in 0..2 {
        limiter.check("user").await.unwrap();
    }
    let blocked = limiter.check("user").await.unwrap();
    assert!(!blocked.allowed);

    limiter.reset("user").await.unwrap();

    let after_reset = limiter.check("user").await.unwrap();
    assert!(after_reset.allowed);
}

// ── Key isolation ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn different_keys_are_tracked_independently() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(1));

    // Exhaust key-a
    limiter.check("key-a").await.unwrap();
    let blocked = limiter.check("key-a").await.unwrap();
    assert!(!blocked.allowed);

    // key-b should be unaffected
    let allowed = limiter.check("key-b").await.unwrap();
    assert!(allowed.allowed);
}

#[tokio::test]
async fn multiple_users_tracked_separately() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(5));

    for _ in 0..5 {
        limiter.check("user:10").await.unwrap();
    }

    // user:10 exhausted
    assert!(!limiter.check("user:10").await.unwrap().allowed);

    // user:20 untouched
    assert!(limiter.check("user:20").await.unwrap().allowed);
}

// ── Memory helpers ────────────────────────────────────────────────────────────

#[tokio::test]
async fn clear_removes_all_tracked_keys() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(5));

    limiter.check("a").await.unwrap();
    limiter.check("b").await.unwrap();
    assert_eq!(limiter.key_count(), 2);

    limiter.clear();
    assert_eq!(limiter.key_count(), 0);
}

#[tokio::test]
async fn key_count_grows_with_new_keys() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(10));
    assert_eq!(limiter.key_count(), 0);

    limiter.check("x1").await.unwrap();
    assert_eq!(limiter.key_count(), 1);

    limiter.check("x2").await.unwrap();
    assert_eq!(limiter.key_count(), 2);
}

// ── Info (non-mutating) ───────────────────────────────────────────────────────

#[tokio::test]
async fn info_returns_correct_limit_field() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(15));
    let info = limiter.info("info-test").await.unwrap();
    assert_eq!(info.limit, 15);
}

// ── retry_after field ─────────────────────────────────────────────────────────

#[tokio::test]
async fn retry_after_is_none_when_request_allowed() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(10));
    let result = limiter.check("ok").await.unwrap();
    assert!(result.retry_after.is_none());
}

#[tokio::test]
async fn retry_after_is_some_when_request_blocked() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(1));
    limiter.check("blocked").await.unwrap();

    let result = limiter.check("blocked").await.unwrap();
    assert!(!result.allowed);
    assert!(result.retry_after.is_some());
}

// ── reset_after field ─────────────────────────────────────────────────────────

#[tokio::test]
async fn reset_after_matches_configured_window_in_seconds() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(10));
    let result = limiter.check("window-test").await.unwrap();
    assert_eq!(result.reset_after, 60);
}

#[tokio::test]
async fn reset_after_matches_custom_window() {
    let limiter = MemoryRateLimiter::new(RateLimitConfig::custom(5, Duration::from_secs(120)));
    let result = limiter.check("custom").await.unwrap();
    assert_eq!(result.reset_after, 120);
}
