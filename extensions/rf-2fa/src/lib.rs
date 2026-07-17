//! Two-Factor Authentication (2FA) for RustForge
//!
//! This crate provides TOTP-based 2FA with QR codes and backup codes.
//!
//! # Rate-limited verification
//!
//! The bare [`TotpManager::verify`] method is a pure function with no
//! brute-force protection.  Use [`RateLimitedVerifier`] for the recommended,
//! production path:
//!
//! ```rust
//! use rf_2fa::{TotpManager, RateLimitedVerifier, TwoFactorError};
//! use std::time::Duration;
//!
//! let manager = TotpManager::default();
//! let secret  = manager.generate_secret();
//!
//! // 5 failures allowed per 30-second window (default)
//! let mut verifier = RateLimitedVerifier::new(TotpManager::default());
//!
//! // A single correct code still works
//! let code = TotpManager::default(); // fresh manager for generate_code
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

use qrcode::QrCode;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use totp_rs::{Algorithm, Secret, TOTP};

/// 2FA errors
#[derive(Debug, Error)]
pub enum TwoFactorError {
    #[error("Invalid TOTP code")]
    InvalidCode,

    #[error("TOTP generation failed: {0}")]
    TotpError(String),

    #[error("QR code generation failed: {0}")]
    QrCodeError(String),

    #[error("Invalid secret")]
    InvalidSecret,

    #[error("Backup code not found")]
    BackupCodeNotFound,

    #[error("Backup code already used")]
    BackupCodeAlreadyUsed,

    #[error("Device not trusted")]
    DeviceNotTrusted,

    /// Returned by [`RateLimitedVerifier`] when the failure threshold for an
    /// identity has been exceeded within the current window.  The caller
    /// should surface a user-facing "too many attempts" message and, if
    /// applicable, start a back-off timer.
    #[error("Too many failed TOTP attempts; try again later")]
    TooManyAttempts,
}

pub type TwoFactorResult<T> = Result<T, TwoFactorError>;

/// TOTP manager for 2FA
pub struct TotpManager {
    issuer: String,
    algorithm: Algorithm,
    digits: usize,
    step: u64,
}

impl TotpManager {
    /// Create a new TOTP manager
    pub fn new(issuer: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            algorithm: Algorithm::SHA1,
            digits: 6,
            step: 30,
        }
    }

    /// Generate a new secret
    pub fn generate_secret(&self) -> String {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 20];
        rng.fill_bytes(&mut bytes);
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &bytes)
    }

    /// Generate QR code as PNG bytes
    pub fn generate_qr_code(&self, secret: &str, account: &str) -> TwoFactorResult<Vec<u8>> {
        let totp = self.create_totp(secret, account)?;
        let qr_code_url = totp.get_url();

        let qr = QrCode::new(qr_code_url.as_bytes())
            .map_err(|e| TwoFactorError::QrCodeError(e.to_string()))?;

        let image = qr.render::<image::Luma<u8>>().build();

        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .map_err(|e| TwoFactorError::QrCodeError(e.to_string()))?;

        Ok(bytes)
    }

    /// Verify a TOTP code
    pub fn verify(&self, secret: &str, code: &str) -> TwoFactorResult<bool> {
        let totp = self.create_totp(secret, "")?;
        totp.check_current(code)
            .map_err(|_| TwoFactorError::InvalidCode)
    }

    /// Generate current TOTP code (for testing)
    pub fn generate_code(&self, secret: &str) -> TwoFactorResult<String> {
        let totp = self.create_totp(secret, "")?;
        totp.generate_current()
            .map_err(|e| TwoFactorError::TotpError(e.to_string()))
    }

    fn create_totp(&self, secret: &str, account: &str) -> TwoFactorResult<TOTP> {
        TOTP::new(
            self.algorithm,
            self.digits,
            1,
            self.step,
            Secret::Encoded(secret.to_string())
                .to_bytes()
                .map_err(|_| TwoFactorError::InvalidSecret)?,
            Some(self.issuer.clone()),
            account.to_string(),
        )
        .map_err(|e| TwoFactorError::TotpError(e.to_string()))
    }
}

impl Default for TotpManager {
    fn default() -> Self {
        Self::new("RustForge")
    }
}

// ---------------------------------------------------------------------------
// Rate-limited TOTP verifier
// ---------------------------------------------------------------------------

/// Per-identity attempt record (in-process state).
#[derive(Debug)]
struct AttemptRecord {
    /// Number of consecutive wrong-code responses within the current window.
    failures: u32,
    /// When the current window started.
    window_start: Instant,
}

/// A stateful wrapper around [`TotpManager`] that enforces a maximum number of
/// failed TOTP attempts within a sliding time window.
///
/// # Defaults
///
/// | Parameter      | Default |
/// |----------------|---------|
/// | `max_failures` | 5       |
/// | `window`       | 30 s    |
///
/// After `max_failures` wrong codes within `window` the verifier returns
/// [`TwoFactorError::TooManyAttempts`] for every subsequent call — **even if
/// the code is correct** — until the window expires or [`Self::reset`] is
/// called.  A correct code that arrives before the threshold resets the
/// counter immediately.
///
/// # Concurrency / multi-process note
///
/// The attempt state lives in process memory.  In a horizontally-scaled
/// deployment each process tracks independently, so true cluster-wide
/// enforcement requires an external shared store (e.g. Redis).  This
/// implementation is suitable for single-process deployments and as a
/// defence-in-depth layer in larger setups.
pub struct RateLimitedVerifier {
    manager: TotpManager,
    tracker: HashMap<String, AttemptRecord>,
    max_failures: u32,
    window: Duration,
}

impl RateLimitedVerifier {
    /// Create with default limits: **5 failures per 30-second window**.
    pub fn new(manager: TotpManager) -> Self {
        Self::with_limits(manager, 5, Duration::from_secs(30))
    }

    /// Create with explicit limits.
    ///
    /// * `max_failures` — number of wrong codes allowed before lockout.
    /// * `window`       — duration after which the counter resets automatically.
    pub fn with_limits(manager: TotpManager, max_failures: u32, window: Duration) -> Self {
        Self {
            manager,
            tracker: HashMap::new(),
            max_failures,
            window,
        }
    }

    /// Verify a TOTP `code` for the given `identity` (e.g. a user ID or
    /// hashed secret), enforcing the configured rate limit.
    ///
    /// * Returns `Ok(true)`  — code is correct; attempt counter is reset.
    /// * Returns `Ok(false)` — code is wrong; failure counter is incremented.
    /// * Returns `Err(TwoFactorError::TooManyAttempts)` — threshold exceeded;
    ///   the code is **not** checked.
    /// * Returns other `Err` variants on configuration/infrastructure errors
    ///   (bad secret, etc.) without touching the failure counter.
    pub fn verify(
        &mut self,
        identity: &str,
        secret: &str,
        code: &str,
    ) -> TwoFactorResult<bool> {
        let now = Instant::now();

        // Retrieve or create the attempt record, resetting if the window lapsed.
        {
            let record =
                self.tracker
                    .entry(identity.to_string())
                    .or_insert_with(|| AttemptRecord {
                        failures: 0,
                        window_start: now,
                    });

            if now.duration_since(record.window_start) >= self.window {
                record.failures = 0;
                record.window_start = now;
            }

            if record.failures >= self.max_failures {
                return Err(TwoFactorError::TooManyAttempts);
            }
        }

        // Delegate to the pure verifier.
        match self.manager.verify(secret, code) {
            Ok(true) => {
                // Correct code — clear the counter.
                self.tracker.remove(identity);
                Ok(true)
            }
            Ok(false) => {
                // Wrong code — count the failure.
                if let Some(record) = self.tracker.get_mut(identity) {
                    record.failures += 1;
                }
                Ok(false)
            }
            Err(e) => {
                // Infrastructure / config error — do not penalise the caller.
                Err(e)
            }
        }
    }

    /// Manually reset the attempt counter for `identity`.
    ///
    /// Use this after an out-of-band verification (e.g. email OTP, admin
    /// override) to restore access without waiting for the window to expire.
    pub fn reset(&mut self, identity: &str) {
        self.tracker.remove(identity);
    }

    /// Return the number of wrong-code attempts still permitted for `identity`
    /// within the current window (0 means locked out).
    ///
    /// Returns `max_failures` if no attempts have been recorded or the last
    /// window has already expired.
    pub fn remaining_attempts(&self, identity: &str) -> u32 {
        let now = Instant::now();
        match self.tracker.get(identity) {
            None => self.max_failures,
            Some(record) => {
                if now.duration_since(record.window_start) >= self.window {
                    self.max_failures
                } else {
                    self.max_failures.saturating_sub(record.failures)
                }
            }
        }
    }

    /// Return a reference to the inner [`TotpManager`].
    pub fn inner(&self) -> &TotpManager {
        &self.manager
    }
}

/// Backup codes for account recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCodes {
    codes: Vec<String>,
    used: Vec<String>,
}

impl BackupCodes {
    /// Generate backup codes
    pub fn generate(count: usize) -> Self {
        let mut rng = rand::thread_rng();
        let codes: Vec<String> = (0..count)
            .map(|_| {
                format!(
                    "{:04}-{:04}-{:04}",
                    rng.gen_range(0..10000),
                    rng.gen_range(0..10000),
                    rng.gen_range(0..10000)
                )
            })
            .collect();

        Self {
            codes,
            used: Vec::new(),
        }
    }

    /// Use a backup code
    pub fn use_code(&mut self, code: &str) -> TwoFactorResult<()> {
        if self.used.contains(&code.to_string()) {
            return Err(TwoFactorError::BackupCodeAlreadyUsed);
        }

        let index = self
            .codes
            .iter()
            .position(|c| c == code)
            .ok_or(TwoFactorError::BackupCodeNotFound)?;

        self.used.push(self.codes[index].clone());
        Ok(())
    }

    /// Check if code is valid and unused
    pub fn is_valid(&self, code: &str) -> bool {
        self.codes.contains(&code.to_string()) && !self.used.contains(&code.to_string())
    }

    /// Get remaining codes count
    pub fn remaining(&self) -> usize {
        self.codes.len() - self.used.len()
    }

    /// Get all codes (for initial display)
    pub fn get_codes(&self) -> &[String] {
        &self.codes
    }
}

/// Trusted device management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: String,
    pub name: String,
    pub trusted_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl TrustedDevice {
    /// Create a new trusted device
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            trusted_at: chrono::Utc::now(),
            last_used: None,
        }
    }

    /// Mark device as used
    pub fn mark_used(&mut self) {
        self.last_used = Some(chrono::Utc::now());
    }

    /// Check if device is still trusted (within 30 days)
    pub fn is_still_trusted(&self) -> bool {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(self.trusted_at);
        diff.num_days() < 30
    }
}

/// Device manager
#[derive(Debug, Clone, Default)]
pub struct DeviceManager {
    devices: Vec<TrustedDevice>,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Trust a device
    pub fn trust_device(&mut self, id: impl Into<String>, name: impl Into<String>) {
        let device = TrustedDevice::new(id, name);
        self.devices.push(device);
    }

    /// Check if device is trusted
    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.devices
            .iter()
            .any(|d| d.id == device_id && d.is_still_trusted())
    }

    /// Remove device
    pub fn remove_device(&mut self, device_id: &str) {
        self.devices.retain(|d| d.id != device_id);
    }

    /// Get all trusted devices
    pub fn get_devices(&self) -> &[TrustedDevice] {
        &self.devices
    }

    /// Clean expired devices
    pub fn clean_expired(&mut self) {
        self.devices.retain(|d| d.is_still_trusted());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_manager() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_generate_and_verify_code() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();
        let code = manager.generate_code(&secret).unwrap();

        assert!(manager.verify(&secret, &code).unwrap());
        assert!(!manager.verify(&secret, "000000").unwrap());
    }

    #[test]
    fn test_qr_code_generation() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();
        let qr = manager.generate_qr_code(&secret, "test@example.com");

        assert!(qr.is_ok());
        let bytes = qr.unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_backup_codes_generation() {
        let codes = BackupCodes::generate(10);
        assert_eq!(codes.codes.len(), 10);
        assert_eq!(codes.remaining(), 10);
    }

    #[test]
    fn test_backup_code_usage() {
        let mut codes = BackupCodes::generate(5);
        let first_code = codes.codes[0].clone();

        assert!(codes.is_valid(&first_code));
        codes.use_code(&first_code).unwrap();
        assert!(!codes.is_valid(&first_code));
        assert_eq!(codes.remaining(), 4);
    }

    #[test]
    fn test_backup_code_invalid() {
        let mut codes = BackupCodes::generate(5);
        let result = codes.use_code("0000-0000-0000");
        assert!(result.is_err());
    }

    #[test]
    fn test_trusted_device() {
        let device = TrustedDevice::new("device-123", "iPhone");
        assert_eq!(device.id, "device-123");
        assert_eq!(device.name, "iPhone");
        assert!(device.is_still_trusted());
        assert!(device.last_used.is_none());
    }

    #[test]
    fn test_device_manager() {
        let mut manager = DeviceManager::new();
        manager.trust_device("device-1", "iPhone");
        manager.trust_device("device-2", "iPad");

        assert!(manager.is_trusted("device-1"));
        assert!(manager.is_trusted("device-2"));
        assert!(!manager.is_trusted("device-3"));

        assert_eq!(manager.get_devices().len(), 2);
    }

    #[test]
    fn test_device_removal() {
        let mut manager = DeviceManager::new();
        manager.trust_device("device-1", "iPhone");

        manager.remove_device("device-1");
        assert!(!manager.is_trusted("device-1"));
    }

    #[test]
    fn test_backup_codes_get_codes() {
        let codes = BackupCodes::generate(3);
        let all_codes = codes.get_codes();
        assert_eq!(all_codes.len(), 3);
    }

    #[test]
    fn test_device_mark_used() {
        let mut device = TrustedDevice::new("test", "Test Device");
        assert!(device.last_used.is_none());

        device.mark_used();
        assert!(device.last_used.is_some());
    }

    // -----------------------------------------------------------------------
    // RateLimitedVerifier tests
    // -----------------------------------------------------------------------

    /// A single correct code passes without triggering the rate limiter.
    #[test]
    fn test_rate_limit_correct_code_passes() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();
        let code = manager.generate_code(&secret).unwrap();

        let mut verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 3, Duration::from_secs(60));

        let result = verifier.verify("user1", &secret, &code);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert!(result.unwrap(), "correct code should return true");
    }

    /// Wrong codes are individually counted but do not trigger lockout until
    /// the threshold is reached.
    #[test]
    fn test_rate_limit_wrong_codes_counted() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();

        // threshold = 3, so attempts 1-3 return Ok(false)
        let mut verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 3, Duration::from_secs(60));

        for i in 1..=3_u32 {
            let r = verifier.verify("user1", &secret, "000000");
            assert!(
                matches!(r, Ok(false)),
                "attempt {i} should be Ok(false), got {r:?}"
            );
            assert_eq!(verifier.remaining_attempts("user1"), 3 - i);
        }
    }

    /// After N wrong codes the (N+1)th attempt is rejected as TooManyAttempts
    /// even when the submitted code happens to be correct.
    #[test]
    fn test_rate_limit_lockout_after_threshold() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();

        // threshold = 3
        let mut verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 3, Duration::from_secs(60));

        // Exhaust the threshold
        for _ in 0..3 {
            let _ = verifier.verify("user1", &secret, "000000");
        }

        assert_eq!(verifier.remaining_attempts("user1"), 0);

        // 4th attempt — correct code, but must be rejected
        let correct = manager.generate_code(&secret).unwrap();
        let r = verifier.verify("user1", &secret, &correct);
        assert!(
            matches!(r, Err(TwoFactorError::TooManyAttempts)),
            "expected TooManyAttempts, got {r:?}"
        );
    }

    /// Lockout for one identity does not affect a different identity.
    #[test]
    fn test_rate_limit_independent_identities() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();

        let mut verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 2, Duration::from_secs(60));

        // Lock out user1
        let _ = verifier.verify("user1", &secret, "000000");
        let _ = verifier.verify("user1", &secret, "000000");
        assert!(matches!(
            verifier.verify("user1", &secret, "000000"),
            Err(TwoFactorError::TooManyAttempts)
        ));

        // user2 is unaffected
        let r = verifier.verify("user2", &secret, "000000");
        assert!(
            matches!(r, Ok(false)),
            "user2 should not be locked out, got {r:?}"
        );
    }

    /// A successful verification resets the failure counter so the identity
    /// may make fresh attempts afterward.
    #[test]
    fn test_rate_limit_success_resets_counter() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();

        // threshold = 3
        let mut verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 3, Duration::from_secs(60));

        // 2 wrong codes (below threshold)
        let _ = verifier.verify("user1", &secret, "000000");
        let _ = verifier.verify("user1", &secret, "000000");
        assert_eq!(verifier.remaining_attempts("user1"), 1);

        // Correct code — counter resets
        let correct = manager.generate_code(&secret).unwrap();
        let r = verifier.verify("user1", &secret, &correct);
        assert!(matches!(r, Ok(true)), "correct code should pass: {r:?}");
        assert_eq!(
            verifier.remaining_attempts("user1"),
            3,
            "counter should have been reset after success"
        );

        // Now a wrong code starts a fresh count, not a lockout
        let r2 = verifier.verify("user1", &secret, "000000");
        assert!(
            matches!(r2, Ok(false)),
            "fresh wrong code after reset should be Ok(false), got {r2:?}"
        );
    }

    /// `reset()` clears the counter immediately, restoring full attempt budget.
    #[test]
    fn test_rate_limit_manual_reset() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();

        let mut verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 2, Duration::from_secs(60));

        // Lock out
        let _ = verifier.verify("user1", &secret, "000000");
        let _ = verifier.verify("user1", &secret, "000000");
        assert!(matches!(
            verifier.verify("user1", &secret, "000000"),
            Err(TwoFactorError::TooManyAttempts)
        ));

        // Admin reset
        verifier.reset("user1");
        assert_eq!(verifier.remaining_attempts("user1"), 2);

        // Identity can attempt again
        let r = verifier.verify("user1", &secret, "000000");
        assert!(
            matches!(r, Ok(false)),
            "after reset should be Ok(false), got {r:?}"
        );
    }

    /// After the window elapses the counter resets and new attempts are allowed.
    #[test]
    fn test_rate_limit_window_expiry_resets_counter() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();

        // Use a 50 ms window so we can test expiry without a long sleep.
        let mut verifier = RateLimitedVerifier::with_limits(
            TotpManager::default(),
            2,
            Duration::from_millis(50),
        );

        // Lock out within the window
        let _ = verifier.verify("user1", &secret, "000000");
        let _ = verifier.verify("user1", &secret, "000000");
        assert!(matches!(
            verifier.verify("user1", &secret, "000000"),
            Err(TwoFactorError::TooManyAttempts)
        ));

        // Wait for the window to expire
        std::thread::sleep(Duration::from_millis(100));

        // Next attempt starts a fresh window — wrong code returns Ok(false), not locked
        let r = verifier.verify("user1", &secret, "000000");
        assert!(
            matches!(r, Ok(false)),
            "after window expiry should be Ok(false), got {r:?}"
        );
        assert_eq!(
            verifier.remaining_attempts("user1"),
            1,
            "one failure recorded in the new window"
        );
    }

    /// remaining_attempts() reports max_failures when no attempts recorded.
    #[test]
    fn test_remaining_attempts_fresh() {
        let verifier =
            RateLimitedVerifier::with_limits(TotpManager::default(), 5, Duration::from_secs(60));
        assert_eq!(verifier.remaining_attempts("nobody"), 5);
    }
}
