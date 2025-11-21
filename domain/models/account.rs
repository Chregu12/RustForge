use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Account Model
///
/// Represents a user account in the system.
/// Contains account metadata and authentication information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique account identifier
    pub id: i64,
    /// Account email address
    pub email: String,
    /// Account name
    pub name: String,
    /// Whether the account is active
    pub is_active: bool,
    /// Timestamp when the account was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the account was last updated
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// Creates a new account instance
    pub fn new(id: i64, email: String, name: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            email,
            name,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Activates the account
    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }

    /// Deactivates the account
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_new_account() {
        let account = Account::new(1, "test@example.com".to_string(), "Test User".to_string());

        assert_eq!(account.id, 1);
        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.name, "Test User");
        assert!(account.is_active);
    }

    #[test]
    fn can_deactivate_account() {
        let mut account = Account::new(1, "test@example.com".to_string(), "Test User".to_string());
        account.deactivate();

        assert!(!account.is_active);
    }

    #[test]
    fn can_activate_account() {
        let mut account = Account::new(1, "test@example.com".to_string(), "Test User".to_string());
        account.deactivate();
        account.activate();

        assert!(account.is_active);
    }
}
