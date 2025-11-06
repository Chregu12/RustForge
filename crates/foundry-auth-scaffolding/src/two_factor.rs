//! Two-Factor Authentication (TOTP)
//!
//! Time-based One-Time Password authentication

use base64::{Engine as _, engine::general_purpose::STANDARD};
use qrcode::QrCode;
use rand::Rng;
use totp_rs::{Algorithm, Secret, TOTP};

/// Two-Factor Authentication Service
pub struct TwoFactorService {
    app_name: String,
}

impl TwoFactorService {
    pub fn new(app_name: String) -> Self {
        Self { app_name }
    }

    /// Generate a new TOTP secret
    pub fn generate_secret(&self) -> String {
        use rand::Rng;
        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(20)
            .collect();
        STANDARD.encode(&random_bytes)
    }

    /// Generate recovery codes
    pub fn generate_recovery_codes(&self, count: usize) -> Vec<String> {
        let mut codes = Vec::with_capacity(count);
        let mut rng = rand::thread_rng();

        for _ in 0..count {
            let code: String = (0..8)
                .map(|_| {
                    let digit = rng.gen_range(0..10);
                    char::from_digit(digit, 10).unwrap()
                })
                .collect();

            // Format as XXXX-XXXX
            let formatted = format!("{}-{}", &code[0..4], &code[4..8]);
            codes.push(formatted);
        }

        codes
    }

    /// Generate QR code for TOTP setup
    pub fn generate_qr_code(&self, email: &str, secret: &str) -> Result<String, String> {
        // Generate OTP auth URL
        let url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}",
            urlencoding::encode(&self.app_name),
            urlencoding::encode(email),
            secret,
            urlencoding::encode(&self.app_name)
        );

        // Generate QR code as SVG
        let qr = QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
        let svg = qr.render::<qrcode::render::svg::Color>().build();

        Ok(svg)
    }

    /// Verify a TOTP code
    pub fn verify_code(&self, secret: &str, code: &str) -> Result<bool, String> {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Encoded(secret.to_string())
                .to_bytes()
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        totp.check_current(code).map_err(|e| e.to_string())
    }

    /// Verify a recovery code
    pub fn verify_recovery_code(&self, recovery_codes: &[String], code: &str) -> bool {
        use subtle::ConstantTimeEq;

        // Use constant-time comparison to prevent timing attacks
        recovery_codes.iter().any(|rc| {
            rc.as_bytes().ct_eq(code.as_bytes()).into()
        })
    }

    /// Remove a used recovery code
    pub fn use_recovery_code(&self, recovery_codes: &mut Vec<String>, code: &str) -> bool {
        use subtle::ConstantTimeEq;

        // Use constant-time comparison to prevent timing attacks
        if let Some(pos) = recovery_codes.iter().position(|rc| {
            rc.as_bytes().ct_eq(code.as_bytes()).into()
        }) {
            recovery_codes.remove(pos);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let service = TwoFactorService::new("Test App".to_string());
        let secret = service.generate_secret();
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_generate_recovery_codes() {
        let service = TwoFactorService::new("Test App".to_string());
        let codes = service.generate_recovery_codes(10);

        assert_eq!(codes.len(), 10);
        for code in &codes {
            assert_eq!(code.len(), 9); // XXXX-XXXX
            assert!(code.contains('-'));
        }
    }

    #[test]
    fn test_recovery_code_usage() {
        let service = TwoFactorService::new("Test App".to_string());
        let mut codes = vec![
            "1234-5678".to_string(),
            "8765-4321".to_string(),
        ];

        assert!(service.verify_recovery_code(&codes, "1234-5678"));
        assert!(!service.verify_recovery_code(&codes, "9999-9999"));

        assert!(service.use_recovery_code(&mut codes, "1234-5678"));
        assert_eq!(codes.len(), 1);
        assert!(!service.use_recovery_code(&mut codes, "1234-5678"));
    }
}
