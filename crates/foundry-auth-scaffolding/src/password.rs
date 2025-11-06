//! Password Reset Management

use crate::models::PasswordReset;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Password Reset Manager
pub struct PasswordResetManager {
    resets: Arc<RwLock<HashMap<String, PasswordReset>>>,
}

impl PasswordResetManager {
    pub fn new() -> Self {
        Self {
            resets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a password reset token
    pub fn store(&self, reset: PasswordReset) {
        let mut resets = self.resets.write()
            .expect("Password reset lock poisoned - unrecoverable state");
        resets.insert(reset.token.clone(), reset);
    }

    /// Find reset by token
    pub fn find(&self, token: &str) -> Option<PasswordReset> {
        let resets = self.resets.read()
            .expect("Password reset lock poisoned - unrecoverable state");
        resets.get(token).cloned()
    }

    /// Delete reset token
    pub fn delete(&self, token: &str) {
        let mut resets = self.resets.write()
            .expect("Password reset lock poisoned - unrecoverable state");
        resets.remove(token);
    }

    /// Delete all reset tokens for an email
    pub fn delete_for_email(&self, email: &str) {
        let mut resets = self.resets.write()
            .expect("Password reset lock poisoned - unrecoverable state");
        resets.retain(|_, reset| reset.email != email);
    }

    /// Clean up expired tokens
    pub fn cleanup_expired(&self) {
        let mut resets = self.resets.write()
            .expect("Password reset lock poisoned - unrecoverable state");
        resets.retain(|_, reset| !reset.is_expired());
    }
}

impl Default for PasswordResetManager {
    fn default() -> Self {
        Self::new()
    }
}
