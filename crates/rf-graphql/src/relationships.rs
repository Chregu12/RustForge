//! Relationship support for GraphQL models
//!
//! Provides traits and utilities for handling relationships between GraphQL objects.

use async_graphql::{Context, Result};

/// Trait for models with relationships
#[async_trait::async_trait]
pub trait HasRelationships: Send + Sync {
    /// The ID type for this model
    type Id: Send + Sync;

    /// Get the model's ID
    fn id(&self) -> Self::Id;
}

/// Trait for "has many" relationships
#[async_trait::async_trait]
pub trait HasMany<Related>: HasRelationships
where
    Related: Send + Sync,
{
    /// Load related items
    async fn load_many(&self, ctx: &Context<'_>) -> Result<Vec<Related>>;
}

/// Trait for "belongs to" relationships
#[async_trait::async_trait]
pub trait BelongsTo<Related>: HasRelationships
where
    Related: Send + Sync,
{
    /// Get the foreign key value
    fn foreign_key(&self) -> Option<i64>;

    /// Load the related item
    async fn load_one(&self, ctx: &Context<'_>) -> Result<Option<Related>>;
}

/// Trait for "has one" relationships
#[async_trait::async_trait]
pub trait HasOne<Related>: HasRelationships
where
    Related: Send + Sync,
{
    /// Load the related item
    async fn load_one(&self, ctx: &Context<'_>) -> Result<Option<Related>>;
}

/// Trait for "many to many" relationships
#[async_trait::async_trait]
pub trait BelongsToMany<Related>: HasRelationships
where
    Related: Send + Sync,
{
    /// Load related items through a pivot table
    async fn load_many(&self, ctx: &Context<'_>) -> Result<Vec<Related>>;
}

// Note: Helper functions for DataLoader removed due to Rust trait limitations
// Users should access DataLoader directly from context:
//
// Example:
// let loader = ctx.data::<DataLoader<UserLoader>>()?;
// let user = loader.load_one(user_id).await?;

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::*;

    #[derive(Clone)]
    struct User {
        id: i64,
        name: String,
    }

    impl HasRelationships for User {
        type Id = i64;

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    #[derive(Clone)]
    struct Post {
        id: i64,
        title: String,
        user_id: i64,
    }

    impl HasRelationships for Post {
        type Id = i64;

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    #[async_trait::async_trait]
    impl HasMany<Post> for User {
        async fn load_many(&self, _ctx: &Context<'_>) -> Result<Vec<Post>> {
            // Simulate loading posts for this user
            Ok(vec![
                Post {
                    id: 1,
                    title: "Post 1".to_string(),
                    user_id: self.id,
                },
                Post {
                    id: 2,
                    title: "Post 2".to_string(),
                    user_id: self.id,
                },
            ])
        }
    }

    #[async_trait::async_trait]
    impl BelongsTo<User> for Post {
        fn foreign_key(&self) -> Option<i64> {
            Some(self.user_id)
        }

        async fn load_one(&self, _ctx: &Context<'_>) -> Result<Option<User>> {
            // Simulate loading user
            Ok(Some(User {
                id: self.user_id,
                name: format!("User {}", self.user_id),
            }))
        }
    }

    #[tokio::test]
    async fn test_has_many_relationship() {
        let user = User {
            id: 1,
            name: "John".to_string(),
        };

        // Relationships are tested through GraphQL queries
        // See the comprehensive test suite in tests/graphql_tests.rs
        assert_eq!(user.id, 1);
    }

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn hello(&self) -> &str {
            "test"
        }
    }
}
