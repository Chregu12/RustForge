//! Authorization helpers for form requests.

use async_trait::async_trait;

/// Trait for authorizing requests.
#[async_trait]
pub trait Authorizable {
    /// The user type for authorization.
    type User;

    /// Authorize the request for the given user.
    async fn authorize(&self, user: &Self::User) -> bool;
}

/// Authorization policy.
#[async_trait]
pub trait AuthorizationPolicy<T> {
    type User;

    /// Check if the user can perform the action.
    async fn can(&self, user: &Self::User, action: &str, resource: &T) -> bool;
}

/// Helper for authorization checks.
pub struct AuthorizationChecker<U> {
    // Stored for the (currently simplified) permission/role checks; WIP.
    #[allow(dead_code)]
    user: U,
}

impl<U> AuthorizationChecker<U> {
    /// Create a new authorization checker.
    pub fn new(user: U) -> Self {
        Self { user }
    }

    /// Check if user has permission.
    pub fn can(&self, _permission: &str) -> bool {
        // Simplified - would check actual permissions
        true
    }

    /// Check if user has role.
    pub fn has_role(&self, _role: &str) -> bool {
        // Simplified - would check actual roles
        true
    }

    /// Check if user has any of the given roles.
    pub fn has_any_role(&self, _roles: &[&str]) -> bool {
        // Simplified - would check actual roles
        true
    }

    /// Check if user has all of the given roles.
    pub fn has_all_roles(&self, _roles: &[&str]) -> bool {
        // Simplified - would check actual roles
        true
    }
}

/// Authorization result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationResult {
    Allowed,
    Denied(String),
}

impl AuthorizationResult {
    /// Check if authorization is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthorizationResult::Allowed)
    }

    /// Check if authorization is denied.
    pub fn is_denied(&self) -> bool {
        !self.is_allowed()
    }

    /// Get the denial reason if denied.
    pub fn denial_reason(&self) -> Option<&str> {
        match self {
            AuthorizationResult::Denied(reason) => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestUser {
        id: i64,
        is_admin: bool,
    }

    struct TestResource {
        user_id: i64,
    }

    struct TestPolicy;

    #[async_trait]
    impl AuthorizationPolicy<TestResource> for TestPolicy {
        type User = TestUser;

        async fn can(&self, user: &Self::User, action: &str, resource: &TestResource) -> bool {
            match action {
                "view" => true,
                "edit" => user.id == resource.user_id || user.is_admin,
                "delete" => user.is_admin,
                _ => false,
            }
        }
    }

    #[tokio::test]
    async fn test_authorization_policy() {
        let policy = TestPolicy;
        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let resource = TestResource { user_id: 1 };

        assert!(policy.can(&user, "view", &resource).await);
        assert!(policy.can(&user, "edit", &resource).await);
        assert!(!policy.can(&user, "delete", &resource).await);
    }

    #[test]
    fn test_authorization_checker() {
        let user = TestUser {
            id: 1,
            is_admin: true,
        };
        let checker = AuthorizationChecker::new(user);

        assert!(checker.can("create_post"));
        assert!(checker.has_role("admin"));
    }

    #[test]
    fn test_authorization_result() {
        let allowed = AuthorizationResult::Allowed;
        assert!(allowed.is_allowed());
        assert!(!allowed.is_denied());

        let denied = AuthorizationResult::Denied("Insufficient permissions".to_string());
        assert!(!denied.is_allowed());
        assert!(denied.is_denied());
        assert_eq!(denied.denial_reason(), Some("Insufficient permissions"));
    }
}
