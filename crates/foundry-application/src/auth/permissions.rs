//! Role-Based Access Control (RBAC) implementation
//!
//! This module provides a flexible permission and role system for fine-grained
//! access control throughout the application.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::guard::AuthError;

/// Permission entity
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Permission {
    /// Create a new permission (for testing/seeding)
    pub fn new(id: i64, name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            slug: slug.into(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Role entity
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Role {
    /// Create a new role (for testing/seeding)
    pub fn new(id: i64, name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            slug: slug.into(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Trait for checking if a user has a specific permission
#[async_trait]
pub trait HasPermission {
    /// Check if user has the given permission by slug
    async fn has_permission(&self, permission: &str) -> bool;

    /// Check if user has any of the given permissions
    async fn has_any_permission(&self, permissions: &[&str]) -> bool;

    /// Check if user has all of the given permissions
    async fn has_all_permissions(&self, permissions: &[&str]) -> bool;

    /// Get all permissions for this user
    async fn get_permissions(&self) -> Vec<Permission>;
}

/// Trait for checking if a user has a specific role
#[async_trait]
pub trait HasRole {
    /// Check if user has the given role by slug
    async fn has_role(&self, role: &str) -> bool;

    /// Check if user has any of the given roles
    async fn has_any_role(&self, roles: &[&str]) -> bool;

    /// Check if user has all of the given roles
    async fn has_all_roles(&self, roles: &[&str]) -> bool;

    /// Get all roles for this user
    async fn get_roles(&self) -> Vec<Role>;
}

/// Permission checker service
pub struct PermissionService {
    db: Arc<DatabaseConnection>,
}

impl PermissionService {
    /// Create a new permission service
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Get all permissions for a user
    pub async fn get_user_permissions(&self, user_id: i64) -> Result<Vec<Permission>, AuthError> {
        // Query to get all permissions for a user through their roles
        // This would join: users -> role_user -> roles -> permission_role -> permissions

        // Simplified placeholder
        // In production, use proper SeaORM queries
        Ok(vec![])
    }

    /// Get all roles for a user
    pub async fn get_user_roles(&self, user_id: i64) -> Result<Vec<Role>, AuthError> {
        // Query to get all roles for a user
        // This would join: users -> role_user -> roles

        // Simplified placeholder
        // In production, use proper SeaORM queries
        Ok(vec![])
    }

    /// Check if user has permission
    pub async fn user_has_permission(
        &self,
        user_id: i64,
        permission_slug: &str,
    ) -> Result<bool, AuthError> {
        let permissions = self.get_user_permissions(user_id).await?;
        Ok(permissions.iter().any(|p| p.slug == permission_slug))
    }

    /// Check if user has role
    pub async fn user_has_role(&self, user_id: i64, role_slug: &str) -> Result<bool, AuthError> {
        let roles = self.get_user_roles(user_id).await?;
        Ok(roles.iter().any(|r| r.slug == role_slug))
    }

    /// Assign a role to a user
    pub async fn assign_role_to_user(
        &self,
        user_id: i64,
        role_id: i64,
    ) -> Result<(), AuthError> {
        // Insert into role_user table
        // In production, use SeaORM
        Ok(())
    }

    /// Remove a role from a user
    pub async fn remove_role_from_user(
        &self,
        user_id: i64,
        role_id: i64,
    ) -> Result<(), AuthError> {
        // Delete from role_user table
        // In production, use SeaORM
        Ok(())
    }

    /// Assign a permission to a role
    pub async fn assign_permission_to_role(
        &self,
        role_id: i64,
        permission_id: i64,
    ) -> Result<(), AuthError> {
        // Insert into permission_role table
        // In production, use SeaORM
        Ok(())
    }

    /// Remove a permission from a role
    pub async fn remove_permission_from_role(
        &self,
        role_id: i64,
        permission_id: i64,
    ) -> Result<(), AuthError> {
        // Delete from permission_role table
        // In production, use SeaORM
        Ok(())
    }

    /// Create a new permission
    pub async fn create_permission(
        &self,
        name: String,
        slug: String,
        description: Option<String>,
    ) -> Result<Permission, AuthError> {
        // Insert into permissions table
        // In production, use SeaORM
        let now = Utc::now();
        Ok(Permission {
            id: 0,
            name,
            slug,
            description,
            created_at: now,
            updated_at: now,
        })
    }

    /// Create a new role
    pub async fn create_role(
        &self,
        name: String,
        slug: String,
        description: Option<String>,
    ) -> Result<Role, AuthError> {
        // Insert into roles table
        // In production, use SeaORM
        let now = Utc::now();
        Ok(Role {
            id: 0,
            name,
            slug,
            description,
            created_at: now,
            updated_at: now,
        })
    }

    /// Find permission by slug
    pub async fn find_permission_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<Permission>, AuthError> {
        // Query permissions table
        // In production, use SeaORM
        Ok(None)
    }

    /// Find role by slug
    pub async fn find_role_by_slug(&self, slug: &str) -> Result<Option<Role>, AuthError> {
        // Query roles table
        // In production, use SeaORM
        Ok(None)
    }

    /// Get all permissions
    pub async fn get_all_permissions(&self) -> Result<Vec<Permission>, AuthError> {
        // Query all permissions
        // In production, use SeaORM
        Ok(vec![])
    }

    /// Get all roles
    pub async fn get_all_roles(&self) -> Result<Vec<Role>, AuthError> {
        // Query all roles
        // In production, use SeaORM
        Ok(vec![])
    }
}

/// User with permissions and roles (wrapper around DbUser)
pub struct AuthorizedUser {
    pub user_id: i64,
    permissions: Arc<PermissionService>,
}

impl AuthorizedUser {
    /// Create a new authorized user
    pub fn new(user_id: i64, permissions: Arc<PermissionService>) -> Self {
        Self {
            user_id,
            permissions,
        }
    }
}

#[async_trait]
impl HasPermission for AuthorizedUser {
    async fn has_permission(&self, permission: &str) -> bool {
        self.permissions
            .user_has_permission(self.user_id, permission)
            .await
            .unwrap_or(false)
    }

    async fn has_any_permission(&self, permissions: &[&str]) -> bool {
        for perm in permissions {
            if self.has_permission(perm).await {
                return true;
            }
        }
        false
    }

    async fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        for perm in permissions {
            if !self.has_permission(perm).await {
                return false;
            }
        }
        true
    }

    async fn get_permissions(&self) -> Vec<Permission> {
        self.permissions
            .get_user_permissions(self.user_id)
            .await
            .unwrap_or_default()
    }
}

#[async_trait]
impl HasRole for AuthorizedUser {
    async fn has_role(&self, role: &str) -> bool {
        self.permissions
            .user_has_role(self.user_id, role)
            .await
            .unwrap_or(false)
    }

    async fn has_any_role(&self, roles: &[&str]) -> bool {
        for role in roles {
            if self.has_role(role).await {
                return true;
            }
        }
        false
    }

    async fn has_all_roles(&self, roles: &[&str]) -> bool {
        for role in roles {
            if !self.has_role(role).await {
                return false;
            }
        }
        true
    }

    async fn get_roles(&self) -> Vec<Role> {
        self.permissions
            .get_user_roles(self.user_id)
            .await
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new(1, "View Dashboard", "dashboard.view")
            .with_description("Can view the dashboard");

        assert_eq!(perm.id, 1);
        assert_eq!(perm.name, "View Dashboard");
        assert_eq!(perm.slug, "dashboard.view");
        assert_eq!(perm.description, Some("Can view the dashboard".to_string()));
    }

    #[test]
    fn test_role_creation() {
        let role = Role::new(1, "Administrator", "admin")
            .with_description("Full system access");

        assert_eq!(role.id, 1);
        assert_eq!(role.name, "Administrator");
        assert_eq!(role.slug, "admin");
        assert_eq!(role.description, Some("Full system access".to_string()));
    }
}
