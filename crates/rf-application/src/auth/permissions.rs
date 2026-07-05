//! Role-Based Access Control (RBAC) implementation
//!
//! This module provides a flexible permission and role system for fine-grained
//! access control throughout the application.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, QueryResult, Statement, Value};
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

    fn backend(&self) -> DatabaseBackend {
        self.db.get_database_backend()
    }

    /// Get all permissions for a user (through the user's roles).
    ///
    /// Joins: `role_user` -> `permission_role` -> `permissions`.
    pub async fn get_user_permissions(&self, user_id: i64) -> Result<Vec<Permission>, AuthError> {
        let sql = "SELECT DISTINCT p.id, p.name, p.slug, p.description, p.created_at, p.updated_at \
                   FROM permissions p \
                   INNER JOIN permission_role pr ON pr.permission_id = p.id \
                   INNER JOIN role_user ru ON ru.role_id = pr.role_id \
                   WHERE ru.user_id = ? \
                   ORDER BY p.id";
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                self.backend(),
                sql,
                [Value::BigInt(Some(user_id))],
            ))
            .await
            .map_err(db_err)?;
        rows.iter().map(row_to_permission).collect()
    }

    /// Get all roles assigned to a user.
    ///
    /// Joins: `role_user` -> `roles`.
    pub async fn get_user_roles(&self, user_id: i64) -> Result<Vec<Role>, AuthError> {
        let sql = "SELECT r.id, r.name, r.slug, r.description, r.created_at, r.updated_at \
                   FROM roles r \
                   INNER JOIN role_user ru ON ru.role_id = r.id \
                   WHERE ru.user_id = ? \
                   ORDER BY r.id";
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                self.backend(),
                sql,
                [Value::BigInt(Some(user_id))],
            ))
            .await
            .map_err(db_err)?;
        rows.iter().map(row_to_role).collect()
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

    /// Assign a role to a user (idempotent).
    pub async fn assign_role_to_user(&self, user_id: i64, role_id: i64) -> Result<(), AuthError> {
        let sql = match self.backend() {
            DatabaseBackend::Postgres => {
                "INSERT INTO role_user (role_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            }
            _ => "INSERT OR IGNORE INTO role_user (role_id, user_id) VALUES (?, ?)",
        };
        self.db
            .execute(Statement::from_sql_and_values(
                self.backend(),
                sql,
                [Value::BigInt(Some(role_id)), Value::BigInt(Some(user_id))],
            ))
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// Remove a role from a user.
    pub async fn remove_role_from_user(
        &self,
        user_id: i64,
        role_id: i64,
    ) -> Result<(), AuthError> {
        self.db
            .execute(Statement::from_sql_and_values(
                self.backend(),
                "DELETE FROM role_user WHERE role_id = ? AND user_id = ?",
                [Value::BigInt(Some(role_id)), Value::BigInt(Some(user_id))],
            ))
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// Assign a permission to a role (idempotent).
    pub async fn assign_permission_to_role(
        &self,
        role_id: i64,
        permission_id: i64,
    ) -> Result<(), AuthError> {
        let sql = match self.backend() {
            DatabaseBackend::Postgres => {
                "INSERT INTO permission_role (permission_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            }
            _ => "INSERT OR IGNORE INTO permission_role (permission_id, role_id) VALUES (?, ?)",
        };
        self.db
            .execute(Statement::from_sql_and_values(
                self.backend(),
                sql,
                [
                    Value::BigInt(Some(permission_id)),
                    Value::BigInt(Some(role_id)),
                ],
            ))
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// Remove a permission from a role.
    pub async fn remove_permission_from_role(
        &self,
        role_id: i64,
        permission_id: i64,
    ) -> Result<(), AuthError> {
        self.db
            .execute(Statement::from_sql_and_values(
                self.backend(),
                "DELETE FROM permission_role WHERE permission_id = ? AND role_id = ?",
                [
                    Value::BigInt(Some(permission_id)),
                    Value::BigInt(Some(role_id)),
                ],
            ))
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// Create a new permission and return it with its assigned id.
    pub async fn create_permission(
        &self,
        name: String,
        slug: String,
        description: Option<String>,
    ) -> Result<Permission, AuthError> {
        let now = Utc::now();
        let id = self
            .insert_named("permissions", &name, &slug, &description, now)
            .await?;
        Ok(Permission {
            id,
            name,
            slug,
            description,
            created_at: now,
            updated_at: now,
        })
    }

    /// Create a new role and return it with its assigned id.
    pub async fn create_role(
        &self,
        name: String,
        slug: String,
        description: Option<String>,
    ) -> Result<Role, AuthError> {
        let now = Utc::now();
        let id = self
            .insert_named("roles", &name, &slug, &description, now)
            .await?;
        Ok(Role {
            id,
            name,
            slug,
            description,
            created_at: now,
            updated_at: now,
        })
    }

    /// Shared insert for the `permissions` / `roles` tables. Returns the new row id.
    async fn insert_named(
        &self,
        table: &str,
        name: &str,
        slug: &str,
        description: &Option<String>,
        now: DateTime<Utc>,
    ) -> Result<i64, AuthError> {
        let values = [
            Value::String(Some(Box::new(name.to_string()))),
            Value::String(Some(Box::new(slug.to_string()))),
            match description {
                Some(d) => Value::String(Some(Box::new(d.clone()))),
                None => Value::String(None),
            },
            Value::ChronoDateTimeUtc(Some(Box::new(now))),
            Value::ChronoDateTimeUtc(Some(Box::new(now))),
        ];
        if self.backend() == DatabaseBackend::Postgres {
            let sql = format!(
                "INSERT INTO {table} (name, slug, description, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5) RETURNING id"
            );
            let row = self
                .db
                .query_one(Statement::from_sql_and_values(self.backend(), sql, values))
                .await
                .map_err(db_err)?
                .ok_or_else(|| AuthError::Internal("insert returned no id".into()))?;
            row.try_get::<i64>("", "id").map_err(db_err)
        } else {
            let sql = format!(
                "INSERT INTO {table} (name, slug, description, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?)"
            );
            let res = self
                .db
                .execute(Statement::from_sql_and_values(self.backend(), sql, values))
                .await
                .map_err(db_err)?;
            Ok(res.last_insert_id() as i64)
        }
    }

    /// Find a permission by its slug.
    pub async fn find_permission_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<Permission>, AuthError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                self.backend(),
                "SELECT id, name, slug, description, created_at, updated_at \
                 FROM permissions WHERE slug = ? LIMIT 1",
                [Value::String(Some(Box::new(slug.to_string())))],
            ))
            .await
            .map_err(db_err)?;
        row.as_ref().map(row_to_permission).transpose()
    }

    /// Find a role by its slug.
    pub async fn find_role_by_slug(&self, slug: &str) -> Result<Option<Role>, AuthError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                self.backend(),
                "SELECT id, name, slug, description, created_at, updated_at \
                 FROM roles WHERE slug = ? LIMIT 1",
                [Value::String(Some(Box::new(slug.to_string())))],
            ))
            .await
            .map_err(db_err)?;
        row.as_ref().map(row_to_role).transpose()
    }

    /// Get all permissions.
    pub async fn get_all_permissions(&self) -> Result<Vec<Permission>, AuthError> {
        let rows = self
            .db
            .query_all(Statement::from_string(
                self.backend(),
                "SELECT id, name, slug, description, created_at, updated_at \
                 FROM permissions ORDER BY id"
                    .to_string(),
            ))
            .await
            .map_err(db_err)?;
        rows.iter().map(row_to_permission).collect()
    }

    /// Get all roles.
    pub async fn get_all_roles(&self) -> Result<Vec<Role>, AuthError> {
        let rows = self
            .db
            .query_all(Statement::from_string(
                self.backend(),
                "SELECT id, name, slug, description, created_at, updated_at \
                 FROM roles ORDER BY id"
                    .to_string(),
            ))
            .await
            .map_err(db_err)?;
        rows.iter().map(row_to_role).collect()
    }
}

/// Map a SeaORM database error into an [`AuthError`].
fn db_err(e: sea_orm::DbErr) -> AuthError {
    AuthError::Internal(e.to_string())
}

/// Robustly read a timestamp column across backends (sqlite stores TEXT).
fn get_timestamp(row: &QueryResult, col: &str) -> DateTime<Utc> {
    if let Ok(dt) = row.try_get::<DateTime<Utc>>("", col) {
        return dt;
    }
    if let Ok(naive) = row.try_get::<chrono::NaiveDateTime>("", col) {
        return DateTime::from_naive_utc_and_offset(naive, Utc);
    }
    if let Ok(s) = row.try_get::<String>("", col) {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return dt.with_timezone(&Utc);
        }
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
            return DateTime::from_naive_utc_and_offset(naive, Utc);
        }
    }
    Utc::now()
}

fn row_to_permission(row: &QueryResult) -> Result<Permission, AuthError> {
    Ok(Permission {
        id: row.try_get("", "id").map_err(db_err)?,
        name: row.try_get("", "name").map_err(db_err)?,
        slug: row.try_get("", "slug").map_err(db_err)?,
        description: row.try_get::<Option<String>>("", "description").unwrap_or(None),
        created_at: get_timestamp(row, "created_at"),
        updated_at: get_timestamp(row, "updated_at"),
    })
}

fn row_to_role(row: &QueryResult) -> Result<Role, AuthError> {
    Ok(Role {
        id: row.try_get("", "id").map_err(db_err)?,
        name: row.try_get("", "name").map_err(db_err)?,
        slug: row.try_get("", "slug").map_err(db_err)?,
        description: row.try_get::<Option<String>>("", "description").unwrap_or(None),
        created_at: get_timestamp(row, "created_at"),
        updated_at: get_timestamp(row, "updated_at"),
    })
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
        let role = Role::new(1, "Administrator", "admin").with_description("Full system access");

        assert_eq!(role.id, 1);
        assert_eq!(role.name, "Administrator");
        assert_eq!(role.slug, "admin");
        assert_eq!(role.description, Some("Full system access".to_string()));
    }
}
