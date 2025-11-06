//! Email Verification Management

use crate::models::EmailVerification;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Email Verification Manager
pub struct EmailVerificationManager {
    verifications: Arc<RwLock<HashMap<String, EmailVerification>>>,
}

impl EmailVerificationManager {
    pub fn new() -> Self {
        Self {
            verifications: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store an email verification token
    pub fn store(&self, verification: EmailVerification) {
        let mut verifications = self.verifications.write()
            .expect("Email verification lock poisoned - unrecoverable state");
        verifications.insert(verification.token.clone(), verification);
    }

    /// Find verification by token
    pub fn find(&self, token: &str) -> Option<EmailVerification> {
        let verifications = self.verifications.read()
            .expect("Email verification lock poisoned - unrecoverable state");
        verifications.get(token).cloned()
    }

    /// Delete verification token
    pub fn delete(&self, token: &str) {
        let mut verifications = self.verifications.write()
            .expect("Email verification lock poisoned - unrecoverable state");
        verifications.remove(token);
    }

    /// Delete all verification tokens for a user
    pub fn delete_for_user(&self, user_id: Uuid) {
        let mut verifications = self.verifications.write()
            .expect("Email verification lock poisoned - unrecoverable state");
        verifications.retain(|_, verification| verification.user_id != user_id);
    }

    /// Clean up expired tokens
    pub fn cleanup_expired(&self) {
        let mut verifications = self.verifications.write()
            .expect("Email verification lock poisoned - unrecoverable state");
        verifications.retain(|_, verification| !verification.is_expired());
    }
}

impl Default for EmailVerificationManager {
    fn default() -> Self {
        Self::new()
    }
}
