//! Schema builder and utilities for GraphQL
//!
//! Provides tools for building and configuring GraphQL schemas.

use async_graphql::{Context, EmptySubscription, ObjectType, Schema, SubscriptionType};

/// GraphQL schema type alias
pub type GraphQLSchema<Q, M, S = EmptySubscription> = Schema<Q, M, S>;

/// Schema builder with configuration options
pub struct SchemaBuilder<Q, M, S = EmptySubscription> {
    query: Q,
    mutation: M,
    subscription: S,
    depth_limit: Option<usize>,
    complexity_limit: Option<usize>,
}

impl<Q, M> SchemaBuilder<Q, M, EmptySubscription>
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
{
    /// Create a new schema builder with query and mutation
    pub fn new(query: Q, mutation: M) -> Self {
        Self {
            query,
            mutation,
            subscription: EmptySubscription,
            depth_limit: None,
            complexity_limit: None,
        }
    }
}

impl<Q, M, S> SchemaBuilder<Q, M, S>
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    /// Set the maximum query depth
    pub fn depth_limit(mut self, limit: usize) -> Self {
        self.depth_limit = Some(limit);
        self
    }

    /// Set the maximum query complexity
    pub fn complexity_limit(mut self, limit: usize) -> Self {
        self.complexity_limit = Some(limit);
        self
    }

    /// Add subscription support
    pub fn subscription<NS>(self, subscription: NS) -> SchemaBuilder<Q, M, NS>
    where
        NS: SubscriptionType + 'static,
    {
        SchemaBuilder {
            query: self.query,
            mutation: self.mutation,
            subscription,
            depth_limit: self.depth_limit,
            complexity_limit: self.complexity_limit,
        }
    }

    /// Build the final schema
    pub fn build(self) -> Schema<Q, M, S> {
        let mut builder = Schema::build(self.query, self.mutation, self.subscription);

        if let Some(limit) = self.depth_limit {
            builder = builder.limit_depth(limit);
        }

        if let Some(limit) = self.complexity_limit {
            builder = builder.limit_complexity(limit);
        }

        builder.finish()
    }
}

/// Build a schema with default settings
pub fn build_schema<Q, M>(query: Q, mutation: M) -> Schema<Q, M, EmptySubscription>
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
{
    SchemaBuilder::new(query, mutation).build()
}

/// Build a schema with subscription support
pub fn build_schema_with_subscription<Q, M, S>(
    query: Q,
    mutation: M,
    subscription: S,
) -> Schema<Q, M, S>
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    Schema::build(query, mutation, subscription).finish()
}

/// Context extensions for common patterns
pub trait ContextExt {
    /// Get data or return a GraphQL error
    fn get_data<T: Send + Sync + 'static>(&self) -> async_graphql::Result<&T>;
}

impl<'a> ContextExt for Context<'a> {
    fn get_data<T: Send + Sync + 'static>(&self) -> async_graphql::Result<&T> {
        self.data::<T>().map_err(|_| {
            async_graphql::Error::new(format!("Data not found: {}", std::any::type_name::<T>()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{build_schema, GraphQLSchema, SchemaBuilder};
    use async_graphql::{EmptyMutation, EmptySubscription, Object, SimpleObject, ID};

    #[derive(SimpleObject, Clone)]
    struct User {
        id: ID,
        name: String,
    }

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn hello(&self) -> &str {
            "Hello, world!"
        }
    }

    struct MutationRoot;

    #[Object]
    impl MutationRoot {
        async fn noop(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_schema_builder() {
        let schema = SchemaBuilder::new(QueryRoot, MutationRoot)
            .depth_limit(10)
            .complexity_limit(100)
            .build();

        let query = r#"{ hello }"#;
        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_build_schema() {
        let schema = build_schema(QueryRoot, MutationRoot);
        let query = r#"{ hello }"#;
        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());
    }
}
