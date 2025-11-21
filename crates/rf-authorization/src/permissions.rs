//! Database-Backed Permissions
//!
//! This module provides structures and utilities for managing permissions
//! stored in a database. This enables role-based access control (RBAC).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A permission represents a specific action a user can perform
///
/// # Example
///
/// ```rust
/// use rf_authorization::permissions::Permission;
///
/// let permission = Permission {
///     id: 1,
///     name: "posts.create".to_string(),
///     description: Some("Create new posts".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

impl Permission {
    pub fn new(id: i64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A role represents a collection of permissions
///
/// # Example
///
/// ```rust
/// use rf_authorization::permissions::{Role, Permission};
///
/// let admin_role = Role {
///     id: 1,
///     name: "admin".to_string(),
///     description: Some("Administrator role".to_string()),
///     permissions: vec![
///         Permission::new(1, "posts.create"),
///         Permission::new(2, "posts.delete"),
///         Permission::new(3, "users.manage"),
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
}

impl Role {
    pub fn new(id: i64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
            permissions: vec![],
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<Permission>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.push(permission);
    }

    pub fn has_permission(&self, permission_name: &str) -> bool {
        self.permissions.iter().any(|p| p.name == permission_name)
    }

    pub fn has_any_permission(&self, permission_names: &[&str]) -> bool {
        permission_names
            .iter()
            .any(|name| self.has_permission(name))
    }

    pub fn has_all_permissions(&self, permission_names: &[&str]) -> bool {
        permission_names
            .iter()
            .all(|name| self.has_permission(name))
    }
}

/// User permissions - aggregated from all roles
///
/// This is typically what you'd store on your User model or in session.
///
/// # Example
///
/// ```rust
/// use rf_authorization::permissions::{UserPermissions, Permission, Role};
///
/// let admin_role = Role::new(1, "admin")
///     .with_permissions(vec![
///         Permission::new(1, "posts.create"),
///         Permission::new(2, "posts.delete"),
///     ]);
///
/// let editor_role = Role::new(2, "editor")
///     .with_permissions(vec![
///         Permission::new(1, "posts.create"),
///         Permission::new(3, "posts.edit"),
///     ]);
///
/// let user_permissions = UserPermissions::from_roles(vec![admin_role, editor_role]);
///
/// assert!(user_permissions.has("posts.create"));
/// assert!(user_permissions.has("posts.delete"));
/// assert!(user_permissions.has("posts.edit"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    pub roles: Vec<Role>,
    permission_set: HashSet<String>,
}

impl UserPermissions {
    pub fn new() -> Self {
        Self {
            roles: vec![],
            permission_set: HashSet::new(),
        }
    }

    pub fn from_roles(roles: Vec<Role>) -> Self {
        let mut permission_set = HashSet::new();

        for role in &roles {
            for permission in &role.permissions {
                permission_set.insert(permission.name.clone());
            }
        }

        Self {
            roles,
            permission_set,
        }
    }

    pub fn add_role(&mut self, role: Role) {
        for permission in &role.permissions {
            self.permission_set.insert(permission.name.clone());
        }
        self.roles.push(role);
    }

    pub fn has(&self, permission: &str) -> bool {
        self.permission_set.contains(permission)
    }

    pub fn has_any(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has(p))
    }

    pub fn has_all(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.has(p))
    }

    pub fn has_role(&self, role_name: &str) -> bool {
        self.roles.iter().any(|r| r.name == role_name)
    }

    pub fn get_all_permissions(&self) -> Vec<String> {
        self.permission_set.iter().cloned().collect()
    }
}

impl Default for UserPermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for models that have permissions
pub trait HasPermissions {
    fn get_permissions(&self) -> &UserPermissions;

    fn has_permission(&self, permission: &str) -> bool {
        self.get_permissions().has(permission)
    }

    fn has_any_permission(&self, permissions: &[&str]) -> bool {
        self.get_permissions().has_any(permissions)
    }

    fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        self.get_permissions().has_all(permissions)
    }

    fn has_role(&self, role_name: &str) -> bool {
        self.get_permissions().has_role(role_name)
    }
}

/// Database loader trait
///
/// Implement this trait to load permissions from your database.
///
/// # Example
///
/// ```rust,ignore
/// use rf_authorization::permissions::{PermissionLoader, Permission, Role};
/// use async_trait::async_trait;
///
/// struct MyPermissionLoader {
///     // database connection
/// }
///
/// #[async_trait]
/// impl PermissionLoader for MyPermissionLoader {
///     async fn load_user_permissions(&self, user_id: i64) -> Result<Vec<Permission>, String> {
///         // Load from database
///         // SELECT permissions.* FROM permissions
///         // INNER JOIN role_permissions ON permissions.id = role_permissions.permission_id
///         // INNER JOIN user_roles ON role_permissions.role_id = user_roles.role_id
///         // WHERE user_roles.user_id = ?
///         Ok(vec![])
///     }
///
///     async fn load_user_roles(&self, user_id: i64) -> Result<Vec<Role>, String> {
///         // Load from database
///         // SELECT roles.* FROM roles
///         // INNER JOIN user_roles ON roles.id = user_roles.role_id
///         // WHERE user_roles.user_id = ?
///         Ok(vec![])
///     }
///
///     async fn load_role_permissions(&self, role_id: i64) -> Result<Vec<Permission>, String> {
///         // Load from database
///         // SELECT permissions.* FROM permissions
///         // INNER JOIN role_permissions ON permissions.id = role_permissions.permission_id
///         // WHERE role_permissions.role_id = ?
///         Ok(vec![])
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait PermissionLoader: Send + Sync {
    /// Load all permissions for a user (from all their roles)
    async fn load_user_permissions(&self, user_id: i64) -> Result<Vec<Permission>, String>;

    /// Load all roles for a user
    async fn load_user_roles(&self, user_id: i64) -> Result<Vec<Role>, String>;

    /// Load all permissions for a specific role
    async fn load_role_permissions(&self, role_id: i64) -> Result<Vec<Permission>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let permission = Permission::new(1, "posts.create")
            .with_description("Create posts");

        assert_eq!(permission.id, 1);
        assert_eq!(permission.name, "posts.create");
        assert_eq!(permission.description, Some("Create posts".to_string()));
    }

    #[test]
    fn test_role_creation() {
        let role = Role::new(1, "admin")
            .with_description("Administrator");

        assert_eq!(role.id, 1);
        assert_eq!(role.name, "admin");
        assert_eq!(role.description, Some("Administrator".to_string()));
    }

    #[test]
    fn test_role_has_permission() {
        let mut role = Role::new(1, "editor");
        role.add_permission(Permission::new(1, "posts.create"));
        role.add_permission(Permission::new(2, "posts.edit"));

        assert!(role.has_permission("posts.create"));
        assert!(role.has_permission("posts.edit"));
        assert!(!role.has_permission("posts.delete"));
    }

    #[test]
    fn test_role_has_any_permission() {
        let role = Role::new(1, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.edit"),
            ]);

        assert!(role.has_any_permission(&["posts.create", "posts.delete"]));
        assert!(!role.has_any_permission(&["posts.delete", "users.manage"]));
    }

    #[test]
    fn test_role_has_all_permissions() {
        let role = Role::new(1, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.edit"),
            ]);

        assert!(role.has_all_permissions(&["posts.create", "posts.edit"]));
        assert!(!role.has_all_permissions(&["posts.create", "posts.delete"]));
    }

    #[test]
    fn test_user_permissions_from_roles() {
        let admin_role = Role::new(1, "admin")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.delete"),
            ]);

        let editor_role = Role::new(2, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(3, "posts.edit"),
            ]);

        let user_permissions = UserPermissions::from_roles(vec![admin_role, editor_role]);

        assert!(user_permissions.has("posts.create"));
        assert!(user_permissions.has("posts.delete"));
        assert!(user_permissions.has("posts.edit"));
        assert!(!user_permissions.has("users.manage"));
    }

    #[test]
    fn test_user_permissions_has_any() {
        let role = Role::new(1, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.edit"),
            ]);

        let user_permissions = UserPermissions::from_roles(vec![role]);

        assert!(user_permissions.has_any(&["posts.create", "posts.delete"]));
        assert!(!user_permissions.has_any(&["posts.delete", "users.manage"]));
    }

    #[test]
    fn test_user_permissions_has_all() {
        let role = Role::new(1, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.edit"),
            ]);

        let user_permissions = UserPermissions::from_roles(vec![role]);

        assert!(user_permissions.has_all(&["posts.create", "posts.edit"]));
        assert!(!user_permissions.has_all(&["posts.create", "posts.delete"]));
    }

    #[test]
    fn test_user_permissions_has_role() {
        let admin_role = Role::new(1, "admin")
            .with_permissions(vec![Permission::new(1, "posts.create")]);

        let user_permissions = UserPermissions::from_roles(vec![admin_role]);

        assert!(user_permissions.has_role("admin"));
        assert!(!user_permissions.has_role("editor"));
    }

    #[test]
    fn test_user_permissions_add_role() {
        let mut user_permissions = UserPermissions::new();

        let role = Role::new(1, "editor")
            .with_permissions(vec![Permission::new(1, "posts.create")]);

        user_permissions.add_role(role);

        assert!(user_permissions.has("posts.create"));
        assert!(user_permissions.has_role("editor"));
    }

    #[test]
    fn test_user_permissions_get_all() {
        let role = Role::new(1, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.edit"),
            ]);

        let user_permissions = UserPermissions::from_roles(vec![role]);
        let all = user_permissions.get_all_permissions();

        assert_eq!(all.len(), 2);
        assert!(all.contains(&"posts.create".to_string()));
        assert!(all.contains(&"posts.edit".to_string()));
    }

    #[test]
    fn test_user_permissions_deduplicates() {
        // Both roles have the same permission
        let admin_role = Role::new(1, "admin")
            .with_permissions(vec![Permission::new(1, "posts.create")]);

        let editor_role = Role::new(2, "editor")
            .with_permissions(vec![Permission::new(1, "posts.create")]);

        let user_permissions = UserPermissions::from_roles(vec![admin_role, editor_role]);

        // Should only have one instance of the permission
        let all = user_permissions.get_all_permissions();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_has_permissions_trait() {
        struct TestUser {
            permissions: UserPermissions,
        }

        impl HasPermissions for TestUser {
            fn get_permissions(&self) -> &UserPermissions {
                &self.permissions
            }
        }

        let role = Role::new(1, "editor")
            .with_permissions(vec![
                Permission::new(1, "posts.create"),
                Permission::new(2, "posts.edit"),
            ]);

        let user = TestUser {
            permissions: UserPermissions::from_roles(vec![role]),
        };

        assert!(user.has_permission("posts.create"));
        assert!(user.has_any_permission(&["posts.create", "posts.delete"]));
        assert!(user.has_all_permissions(&["posts.create", "posts.edit"]));
        assert!(user.has_role("editor"));
    }
}
