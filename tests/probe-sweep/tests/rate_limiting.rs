// Integration probe: rate_limiting
// Adapted from sandbox/probes/rate_limiting/src/main.rs
// Exercises MemoryRateLimiter: window allow, N+1 block, reset, clear.

use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimiter};

#[tokio::test]
async fn test_rate_limiting() -> anyhow::Result<()> {
    const N: u64 = 5;
    let config = RateLimitConfig::per_minute(N);
    let limiter = MemoryRateLimiter::new(config);

    // First N attempts must be allowed with decaying `remaining`.
    for i in 0..N {
        let result = limiter.check("user:42").await?;
        assert!(result.allowed, "attempt {} should be allowed", i + 1);
        assert_eq!(result.limit, N, "limit should equal configured max");
        assert_eq!(
            result.remaining,
            N - 1 - i,
            "remaining mismatch on attempt {}",
            i + 1
        );
        assert!(
            result.retry_after.is_none(),
            "retry_after must be None while allowed"
        );
    }

    // Attempt N+1 must be blocked.
    let blocked = limiter.check("user:42").await?;
    assert!(!blocked.allowed, "attempt N+1 must be blocked");
    assert_eq!(blocked.remaining, 0, "remaining must be 0 when blocked");
    let retry = blocked
        .retry_after
        .expect("retry_after must be Some when blocked");
    assert!(retry > 0, "retry_after seconds should be positive");
    assert_eq!(retry, 60, "per_minute window => 60s retry_after");

    // A separate key is independent.
    let other = limiter.check("user:99").await?;
    assert!(other.allowed, "independent key must still be allowed");
    assert_eq!(other.remaining, N - 1);

    // reset(key) clears that key's counter.
    limiter.reset("user:42").await?;
    let after_reset = limiter.check("user:42").await?;
    assert!(after_reset.allowed, "after reset the key should be allowed again");
    assert_eq!(after_reset.remaining, N - 1, "counter should be fresh after reset");

    // info() is a non-mutating peek: must NOT consume quota.
    let info = limiter.info("user:99").await?;
    assert_eq!(info.limit, N);
    assert_eq!(info.remaining, N - 1, "info() must not consume a slot");
    let info2 = limiter.info("user:99").await?;
    assert_eq!(info2.remaining, N - 1, "info() must be idempotent");

    // clear() wipes all tracked keys.
    assert!(limiter.key_count() > 0, "should track keys before clear");
    limiter.clear();
    assert_eq!(limiter.key_count(), 0, "clear() must drop all tracked keys");
    let fresh = limiter.check("user:42").await?;
    assert!(fresh.allowed, "after clear the key starts fresh");
    assert_eq!(fresh.remaining, N - 1);

    Ok(())
}
