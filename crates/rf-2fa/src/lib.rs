//! Two-Factor Authentication (2FA) for RustForge
//!
//! This crate provides TOTP-based 2FA with QR codes and backup codes.

use qrcode::QrCode;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use totp_rs::{Algorithm, TOTP, Secret};

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

    #[error("Device not trusted")]
    DeviceNotTrusted,
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
        image.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .map_err(|e| TwoFactorError::QrCodeError(e.to_string()))?;

        Ok(bytes)
    }

    /// Verify a TOTP code
    pub fn verify(&self, secret: &str, code: &str) -> TwoFactorResult<bool> {
        let totp = self.create_totp(secret, "")?;
        Ok(totp.check_current(code).map_err(|_| TwoFactorError::InvalidCode)?)
    }

    /// Generate current TOTP code (for testing)
    pub fn generate_code(&self, secret: &str) -> TwoFactorResult<String> {
        let totp = self.create_totp(secret, "")?;
        Ok(totp.generate_current().map_err(|e| TwoFactorError::TotpError(e.to_string()))?)
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
            return Err(TwoFactorError::BackupCodeNotFound);
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
}
