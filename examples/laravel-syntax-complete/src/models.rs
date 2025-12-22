//! Laravel-style Models (Demo without actual DB connection)
//!
//! In a real application, you would use #[model] macro which requires:
//! - Models in separate modules to avoid Relation enum conflicts
//! - rf crate with DB facade for static query methods

use serde::{Deserialize, Serialize};

// ==========================================
// USER MODEL
// ==========================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// Get user's posts
    pub fn posts(&self) -> Vec<Post> {
        // HasMany relationship
        vec![] // Simplified for demo
    }

    /// Check if user is admin
    pub fn is_administrator(&self) -> bool {
        self.is_admin
    }
}

// ==========================================
// POST MODEL
// ==========================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub user_id: i32,
    pub views: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Post {
    /// Get post author
    pub fn author(&self) -> Option<User> {
        // BelongsTo relationship
        None // Simplified for demo
    }

    /// Get post comments
    pub fn comments(&self) -> Vec<Comment> {
        // HasMany relationship
        vec![] // Simplified for demo
    }

    /// Increment views
    pub fn increment_views(&mut self) {
        self.views += 1;
    }
}

// ==========================================
// COMMENT MODEL
// ==========================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: i32,
    pub content: String,
    pub user_id: i32,
    pub post_id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Comment {
    /// Get comment author
    pub fn author(&self) -> Option<User> {
        // BelongsTo relationship
        None // Simplified for demo
    }

    /// Get parent post
    pub fn post(&self) -> Option<Post> {
        // BelongsTo relationship
        None // Simplified for demo
    }
}

// Mock implementations for demo
impl User {
    pub fn create<T>(_data: T) -> Self {
        Self {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed".to_string(),
            is_admin: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![]
    }

    pub fn find(_id: i32) -> Option<Self> {
        Some(Self::create(()))
    }

    pub fn r#where(_field: &str, _value: &str) -> QueryBuilder {
        QueryBuilder
    }

    pub fn with_posts() -> QueryBuilder {
        QueryBuilder
    }
}

impl Post {
    pub fn create<T>(_data: T) -> Self {
        Self {
            id: 1,
            title: "Test Post".to_string(),
            content: "Content".to_string(),
            published: true,
            user_id: 1,
            views: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![]
    }

    pub fn find(_id: i32) -> Self {
        Self::create(())
    }

    pub fn r#where(_field: &str, _value: i32) -> QueryBuilder {
        QueryBuilder
    }

    pub fn or_fail(self) -> Self {
        self
    }

    pub fn update<T>(&mut self, _data: T) {
        self.updated_at = chrono::Utc::now();
    }

    pub fn delete(&self) {
        // Delete post
    }

    pub fn force_delete(&self) {
        // Force delete (bypass soft deletes)
    }
}

impl Comment {
    pub fn create<T>(_data: T) -> Self {
        Self {
            id: 1,
            content: "Test Comment".to_string(),
            user_id: 1,
            post_id: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

// Mock QueryBuilder
pub struct QueryBuilder;
impl QueryBuilder {
    pub fn first(self) -> Option<User> {
        Some(User::create(()))
    }

    pub fn get(self) -> Vec<Post> {
        vec![]
    }
}
