//! Authentication and authorization for GraphQL
//!
//! Provides middleware and guards for securing GraphQL resolvers.

use async_graphql::{Context, Guard, Result};
use serde::{Deserialize, Serialize};

/// Authenticated user context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    /// User ID
    pub id: i64,
    /// Username
    pub username: String,
    /// User roles
    pub roles: Vec<String>,
}

impl AuthUser {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

/// Extract authenticated user from context
pub fn get_auth_user<'a>(ctx: &'a Context<'_>) -> Result<&'a AuthUser> {
    ctx.data::<AuthUser>()
        .map_err(|_| async_graphql::Error::new("Unauthorized: No authenticated user"))
}

/// Extract optional authenticated user from context
pub fn get_optional_auth_user<'a>(ctx: &'a Context<'_>) -> Option<&'a AuthUser> {
    ctx.data::<AuthUser>().ok()
}

/// Guard that requires authentication
///
/// Example:
/// ```ignore
/// #[Object]
/// impl Query {
///     #[graphql(guard = "AuthGuard")]
///     async fn protected_data(&self, ctx: &Context<'_>) -> Result<String> {
///         Ok("Protected data".to_string())
///     }
/// }
/// ```
pub struct AuthGuard;

impl AuthGuard {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuthGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Guard for AuthGuard {
    // Explicit `impl Future + Send` matches the async-graphql `Guard` trait and
    // guarantees the `Send` bound; do not desugar to `async fn`.
    #[allow(clippy::manual_async_fn)]
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            get_auth_user(ctx)?;
            Ok(())
        }
    }
}

/// Guard that requires specific roles
pub struct RoleGuard {
    roles: Vec<String>,
}

impl RoleGuard {
    /// Create a new role guard
    pub fn new(roles: Vec<String>) -> Self {
        Self { roles }
    }

    /// Create a role guard for a single role
    pub fn single(role: impl Into<String>) -> Self {
        Self::new(vec![role.into()])
    }
}

impl Guard for RoleGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let roles = self.roles.clone();
        async move {
            let user = get_auth_user(ctx)?;

            if user.has_any_role(&roles.iter().map(|r| r.as_str()).collect::<Vec<_>>()) {
                Ok(())
            } else {
                Err(async_graphql::Error::new(format!(
                    "Forbidden: Requires one of these roles: {}",
                    roles.join(", ")
                )))
            }
        }
    }
}

/// Guard that requires all specified roles
pub struct AllRolesGuard {
    roles: Vec<String>,
}

impl AllRolesGuard {
    /// Create a new all roles guard
    pub fn new(roles: Vec<String>) -> Self {
        Self { roles }
    }
}

impl Guard for AllRolesGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let roles = self.roles.clone();
        async move {
            let user = get_auth_user(ctx)?;

            if user.has_all_roles(&roles.iter().map(|r| r.as_str()).collect::<Vec<_>>()) {
                Ok(())
            } else {
                Err(async_graphql::Error::new(format!(
                    "Forbidden: Requires all of these roles: {}",
                    roles.join(", ")
                )))
            }
        }
    }
}

/// Guard that requires ownership
pub struct OwnershipGuard<F>
where
    F: Fn(&Context<'_>, i64) -> bool + Send + Sync,
{
    check_ownership: F,
}

impl<F> OwnershipGuard<F>
where
    F: Fn(&Context<'_>, i64) -> bool + Send + Sync,
{
    /// Create a new ownership guard
    pub fn new(check_ownership: F) -> Self {
        Self { check_ownership }
    }
}

impl<F> Guard for OwnershipGuard<F>
where
    F: Fn(&Context<'_>, i64) -> bool + Send + Sync + 'static,
{
    // Explicit `impl Future + Send` matches the async-graphql `Guard` trait and
    // guarantees the `Send` bound; do not desugar to `async fn`.
    #[allow(clippy::manual_async_fn)]
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let user = get_auth_user(ctx)?;

            if (self.check_ownership)(ctx, user.id) {
                Ok(())
            } else {
                Err(async_graphql::Error::new(
                    "Forbidden: You don't own this resource",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_has_role() {
        let user = AuthUser {
            id: 1,
            username: "john".to_string(),
            roles: vec!["admin".to_string(), "user".to_string()],
        };

        assert!(user.has_role("admin"));
        assert!(user.has_role("user"));
        assert!(!user.has_role("superadmin"));
    }

    #[test]
    fn test_auth_user_has_any_role() {
        let user = AuthUser {
            id: 1,
            username: "john".to_string(),
            roles: vec!["admin".to_string()],
        };

        assert!(user.has_any_role(&["admin", "superadmin"]));
        assert!(!user.has_any_role(&["user", "guest"]));
    }

    #[test]
    fn test_auth_user_has_all_roles() {
        let user = AuthUser {
            id: 1,
            username: "john".to_string(),
            roles: vec!["admin".to_string(), "user".to_string()],
        };

        assert!(user.has_all_roles(&["admin", "user"]));
        assert!(!user.has_all_roles(&["admin", "superadmin"]));
    }

    #[tokio::test]
    async fn test_role_guard() {
        let guard = RoleGuard::single("admin");

        // Guard check would require a proper Context with AuthUser
        // This is a simplified test
        assert_eq!(guard.roles, vec!["admin"]);
    }

    #[tokio::test]
    async fn test_all_roles_guard() {
        let guard = AllRolesGuard::new(vec!["admin".to_string(), "user".to_string()]);

        assert_eq!(guard.roles.len(), 2);
    }
}
