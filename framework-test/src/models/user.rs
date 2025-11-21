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
    pub async fn posts(&self) -> Vec<super::Post> {
        // Implementation: Post::where("user_id", self.id).get()
        vec![]
    }

    /// HasMany: Get all comments by this user
    pub async fn comments(&self) -> Vec<super::Comment> {
        // Implementation: Comment::where("user_id", self.id).get()
        vec![]
    }

    /// HasMany: Get all orders by this user
    pub async fn orders(&self) -> Vec<super::Order> {
        // Implementation: Order::where("user_id", self.id).get()
        vec![]
    }

    /// BelongsToMany: Get roles assigned to this user
    pub async fn roles(&self) -> Vec<super::Role> {
        // Implementation: Role::whereHas("users", |q| q.where("user_id", self.id)).get()
        vec![]
    }

    /// HasManyThrough: Get all comments on this user's posts
    pub async fn post_comments(&self) -> Vec<super::Comment> {
        // Implementation: Comment::whereHas("post.user", |q| q.where("id", self.id)).get()
        vec![]
    }

    /// MorphMany: Get all images for this user
    pub async fn images(&self) -> Vec<super::Image> {
        // Implementation: Image::where("imageable_type", "User")
        //                       .where("imageable_id", self.id).get()
        vec![]
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
    pub async fn has_role(&self, role: &str) -> bool {
        // Implementation
        false
    }

    /// Check if user has a specific permission
    pub async fn has_permission(&self, permission: &str) -> bool {
        // Implementation
        false
    }

    /// Check if user can perform action
    pub async fn can(&self, ability: &str) -> bool {
        // Implementation
        false
    }
}
