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

/// Implemented by user types so an [`AuthorizationChecker`] can make real
/// permission/role decisions.
///
/// Without this, the checker would have no way to know what a user is allowed
/// to do. Implement it for your authenticated-user type (typically by reading
/// the roles/permissions you loaded from the database or token).
///
/// # Wildcards
///
/// A permission of `"*"` returned from [`permissions`](HasAuthorization::permissions)
/// grants every permission (Laravel-style super-admin). A `"prefix.*"` entry
/// grants every permission under that dotted prefix (e.g. `"posts.*"` grants
/// `"posts.create"`).
pub trait HasAuthorization {
    /// The permissions granted to this user (e.g. `"posts.create"`).
    fn permissions(&self) -> Vec<String>;

    /// The roles assigned to this user (e.g. `"admin"`).
    fn roles(&self) -> Vec<String>;
}

/// Returns `true` when `granted` authorizes `wanted`, honouring `"*"` and
/// `"prefix.*"` wildcards.
fn permission_matches(granted: &str, wanted: &str) -> bool {
    if granted == "*" || granted == wanted {
        return true;
    }
    if let Some(prefix) = granted.strip_suffix(".*") {
        // "posts.*" grants "posts.create" (and nested "posts.tags.edit").
        return wanted
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.'));
    }
    false
}

/// Helper for authorization checks.
pub struct AuthorizationChecker<U> {
    user: U,
}

impl<U> AuthorizationChecker<U> {
    /// Create a new authorization checker.
    pub fn new(user: U) -> Self {
        Self { user }
    }

    /// Borrow the wrapped user.
    pub fn user(&self) -> &U {
        &self.user
    }
}

impl<U: HasAuthorization> AuthorizationChecker<U> {
    /// Check if the user has the given permission.
    ///
    /// Honours `"*"` (all permissions) and `"prefix.*"` wildcards.
    pub fn can(&self, permission: &str) -> bool {
        self.user
            .permissions()
            .iter()
            .any(|granted| permission_matches(granted, permission))
    }

    /// Check if the user has the given role.
    pub fn has_role(&self, role: &str) -> bool {
        self.user.roles().iter().any(|r| r == role)
    }

    /// Check if the user has any of the given roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        let user_roles = self.user.roles();
        roles
            .iter()
            .any(|role| user_roles.iter().any(|r| r == role))
    }

    /// Check if the user has all of the given roles.
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        let user_roles = self.user.roles();
        roles
            .iter()
            .all(|role| user_roles.iter().any(|r| r == role))
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

    impl HasAuthorization for TestUser {
        fn permissions(&self) -> Vec<String> {
            if self.is_admin {
                vec!["*".to_string()]
            } else {
                vec!["posts.create".to_string(), "posts.edit".to_string()]
            }
        }

        fn roles(&self) -> Vec<String> {
            if self.is_admin {
                vec!["admin".to_string(), "editor".to_string()]
            } else {
                vec!["editor".to_string()]
            }
        }
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
    fn test_authorization_checker_admin_wildcard() {
        let checker = AuthorizationChecker::new(TestUser {
            id: 1,
            is_admin: true,
        });

        // "*" grants every permission.
        assert!(checker.can("posts.delete"));
        assert!(checker.can("anything.at.all"));
        assert!(checker.has_role("admin"));
        assert!(checker.has_any_role(&["admin", "nope"]));
        assert!(checker.has_all_roles(&["admin", "editor"]));
    }

    #[test]
    fn test_authorization_checker_denies_missing() {
        let checker = AuthorizationChecker::new(TestUser {
            id: 2,
            is_admin: false,
        });

        // Real checks: only granted permissions/roles pass.
        assert!(checker.can("posts.create"));
        assert!(!checker.can("posts.delete"));
        assert!(!checker.can("*"));
        assert!(checker.has_role("editor"));
        assert!(!checker.has_role("admin"));
        assert!(!checker.has_any_role(&["admin", "owner"]));
        assert!(checker.has_any_role(&["admin", "editor"]));
        assert!(!checker.has_all_roles(&["editor", "admin"]));
    }

    #[test]
    fn test_permission_wildcard_prefix() {
        struct Mod;
        impl HasAuthorization for Mod {
            fn permissions(&self) -> Vec<String> {
                vec!["posts.*".to_string()]
            }
            fn roles(&self) -> Vec<String> {
                vec![]
            }
        }
        let checker = AuthorizationChecker::new(Mod);
        assert!(checker.can("posts.create"));
        assert!(checker.can("posts.tags.edit"));
        assert!(!checker.can("users.create"));
        assert!(!checker.can("posts")); // no trailing segment
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
