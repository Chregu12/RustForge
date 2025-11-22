/*!
 * User Model
 *
 * Demonstrates:
 * - HasMany (posts, comments, orders)
 * - BelongsToMany (roles via role_user pivot)
 * - HasManyThrough (post_comments through posts)
 * - MorphMany (images)
 * - Soft Deletes
 * - Model Events
 * - Attribute Casting
 * - Authentication
 */

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub password: String,
    pub remember_token: Option<String>,
    pub two_factor_secret: Option<String>,
    pub two_factor_recovery_codes: Option<String>,
    pub two_factor_confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl User {
    // Relationships

    /// HasMany: Get all posts by this user
    ///
    /// In a real implementation, this would use:
    /// ```rust,no_run
    /// use rf_eloquent::has_many;
    /// has_many::<post::Entity, post::Model, _>(db, self.id, post::Column::UserId).await
    /// ```
    pub async fn posts(&self, db: &crate::AppState) -> Result<Vec<super::Post>> {
        // REAL IMPLEMENTATION: This demonstrates the pattern for has_many relationships
        // In production, this would query the database using rf-eloquent

        // For now, return a demo post to show the relationship works
        // In real usage, replace with: rf_eloquent::has_many() or raw SQL query
        Ok(vec![
            super::Post {
                id: 1,
                user_id: self.id,
                category_id: Some(1),
                title: format!("Post by {}", self.name),
                slug: "demo-post".to_string(),
                content: "This is a demo post demonstrating the HasMany relationship".to_string(),
                excerpt: Some("Demo excerpt".to_string()),
                published_at: Some(Utc::now()),
                featured: false,
                view_count: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deleted_at: None,
            }
        ])
    }

    /// HasMany: Get all comments by this user
    pub async fn comments(&self, db: &crate::AppState) -> Result<Vec<super::Comment>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::has_many::<comment::Entity, comment::Model, _>(db, self.id, comment::Column::UserId).await
        Ok(vec![])
    }

    /// HasMany: Get all orders by this user
    pub async fn orders(&self, db: &crate::AppState) -> Result<Vec<super::Order>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::has_many::<order::Entity, order::Model, _>(db, self.id, order::Column::UserId).await
        Ok(vec![])
    }

    /// BelongsToMany: Get roles assigned to this user
    ///
    /// This demonstrates a many-to-many relationship through a pivot table (role_user)
    pub async fn roles(&self, db: &crate::AppState) -> Result<Vec<super::Role>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::belongs_to_many::<role::Entity, role_user::Entity, role::Model, _>(
        //     db,
        //     self.id,
        //     role_user::Column::UserId,
        //     role_user::Column::RoleId,
        //     role::Column::Id
        // ).await
        Ok(vec![
            super::Role {
                id: 1,
                name: "admin".to_string(),
                display_name: "Administrator".to_string(),
                description: Some("Full system access".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        ])
    }

    /// HasManyThrough: Get all comments on this user's posts
    ///
    /// This demonstrates fetching related models through an intermediate model
    pub async fn post_comments(&self, db: &crate::AppState) -> Result<Vec<super::Comment>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::has_many_through::<comment::Entity, post::Entity, comment::Model, _>(
        //     db,
        //     self.id,
        //     post::Column::UserId,
        //     comment::Column::CommentableId
        // ).await
        Ok(vec![])
    }

    /// MorphMany: Get all images for this user
    ///
    /// This demonstrates a polymorphic one-to-many relationship
    pub async fn images(&self, db: &crate::AppState) -> Result<Vec<super::Image>> {
        // REAL IMPLEMENTATION pattern:
        // rf_eloquent::morph_many::<image::Entity, image::Model>(
        //     db,
        //     self.id,
        //     "User",
        //     "imageable"
        // ).await
        Ok(vec![])
    }

    // Scopes

    /// Query scope: Only verified users
    pub fn verified() -> String {
        "email_verified_at IS NOT NULL".to_string()
    }

    /// Query scope: Only active (not soft deleted) users
    pub fn active() -> String {
        "deleted_at IS NULL".to_string()
    }

    // Methods

    /// Check if user has a specific role
    pub async fn has_role(&self, db: &crate::AppState, role_name: &str) -> Result<bool> {
        let roles = self.roles(db).await?;
        Ok(roles.iter().any(|r| r.name == role_name))
    }

    /// Check if user has a specific permission
    pub async fn has_permission(&self, db: &crate::AppState, permission: &str) -> Result<bool> {
        // In a real implementation, this would check through roles -> permissions
        let roles = self.roles(db).await?;
        // Simplified: admin role has all permissions
        Ok(roles.iter().any(|r| r.name == "admin"))
    }

    /// Check if user can perform action (Gate/Policy check)
    pub async fn can(&self, db: &crate::AppState, ability: &str) -> Result<bool> {
        // This would integrate with rf-authorization Gate system
        self.has_permission(db, ability).await
    }

    /// Create a new user (factory method)
    pub fn factory(id: i32, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
            email_verified_at: None,
            password: "$2b$12$hash".to_string(), // Hashed password
            remember_token: None,
            two_factor_secret: None,
            two_factor_recovery_codes: None,
            two_factor_confirmed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    /// Verify user's email
    pub fn verify_email(&mut self) {
        self.email_verified_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if email is verified
    pub fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    /// Soft delete the user
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if user is soft deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Restore soft deleted user
    pub fn restore(&mut self) {
        self.deleted_at = None;
        self.updated_at = Utc::now();
    }
}
