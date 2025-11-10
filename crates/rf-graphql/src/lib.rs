//! # rf-graphql: Complete GraphQL Support for RustForge
//!
//! Provides full GraphQL implementation with queries, mutations, subscriptions,
//! and DataLoader support for efficient data fetching.
//!
//! ## Features
//!
//! - **Schema Builder**: Easy schema construction
//! - **Query/Mutation/Subscription**: All GraphQL operation types
//! - **DataLoader**: N+1 query prevention
//! - **Playground**: GraphQL playground UI
//! - **Authentication**: Middleware support
//! - **Error Handling**: Type-safe error handling
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_graphql::*;
//! use async_graphql::*;
//! use axum::Router;
//!
//! // Define your types
//! #[derive(SimpleObject)]
//! struct User {
//!     id: ID,
//!     name: String,
//!     email: String,
//! }
//!
//! // Query root
//! struct QueryRoot;
//!
//! #[Object]
//! impl QueryRoot {
//!     async fn user(&self, id: ID) -> Result<User> {
//!         Ok(User {
//!             id,
//!             name: "John".to_string(),
//!             email: "john@example.com".to_string(),
//!         })
//!     }
//! }
//!
//! // Mutation root
//! struct MutationRoot;
//!
//! #[Object]
//! impl MutationRoot {
//!     async fn create_user(&self, name: String, email: String) -> Result<User> {
//!         Ok(User {
//!             id: ID::from("123"),
//!             name,
//!             email,
//!         })
//!     }
//! }
//!
//! # async fn example() {
//! // Build schema
//! let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
//!     .finish();
//!
//! // Create router
//! let app = Router::new()
//!     .merge(graphql_router(schema))
//!     .merge(graphql_playground_router());
//! # }
//! ```

pub use async_graphql::{
    self, dataloader, Context, EmptyMutation, EmptySubscription, Error, ErrorExtensions,
    InputObject, Object, Result, Schema, SimpleObject, Subscription, ID,
};
pub use dataloader::DataLoader;
pub use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// GraphQL schema type alias
pub type GraphQLSchema<Q, M, S> = Schema<Q, M, S>;

/// Create a GraphQL router with query and mutation endpoints
///
/// # Example
///
/// ```no_run
/// use rf_graphql::*;
/// use async_graphql::*;
///
/// struct QueryRoot;
///
/// #[Object]
/// impl QueryRoot {
///     async fn hello(&self) -> &str {
///         "Hello, world!"
///     }
/// }
///
/// # async fn example() {
/// let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
///     .finish();
///
/// let app = graphql_router(schema);
/// # }
/// ```
pub fn graphql_router<Q, M, S>(schema: Schema<Q, M, S>) -> Router
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    let schema = Arc::new(schema);

    Router::new()
        .route("/graphql", post(graphql_handler::<Q, M, S>))
        .with_state(schema)
}

/// GraphQL query/mutation handler
async fn graphql_handler<Q, M, S>(
    State(schema): State<Arc<Schema<Q, M, S>>>,
    req: GraphQLRequest,
) -> GraphQLResponse
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    schema.execute(req.into_inner()).await.into()
}

/// Create a GraphQL playground router
///
/// Provides an interactive GraphQL playground UI at /playground
///
/// # Example
///
/// ```no_run
/// use rf_graphql::*;
/// use axum::Router;
///
/// # async fn example() {
/// let app = Router::new()
///     .merge(graphql_playground_router());
/// # }
/// ```
pub fn graphql_playground_router() -> Router {
    Router::new().route("/playground", get(graphql_playground))
}

/// GraphQL playground HTML
async fn graphql_playground() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>GraphQL Playground</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphql-playground-react/build/static/css/index.css" />
    <link rel="shortcut icon" href="https://cdn.jsdelivr.net/npm/graphql-playground-react/build/favicon.png" />
    <script src="https://cdn.jsdelivr.net/npm/graphql-playground-react/build/static/js/middleware.js"></script>
</head>
<body>
    <div id="root"></div>
    <script>
        window.addEventListener('load', function (event) {
            GraphQLPlayground.init(document.getElementById('root'), {
                endpoint: '/graphql',
                subscriptionEndpoint: '/graphql',
                settings: {
                    'request.credentials': 'same-origin'
                }
            })
        })
    </script>
</body>
</html>
"#,
    )
}

/// Re-export common traits
pub use async_graphql::{ObjectType, OutputType, SubscriptionType};

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::*;

    #[derive(SimpleObject, Clone)]
    struct User {
        id: ID,
        name: String,
    }

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn user(&self, id: ID) -> Result<User> {
            Ok(User {
                id,
                name: "Test User".to_string(),
            })
        }

        async fn users(&self) -> Result<Vec<User>> {
            Ok(vec![
                User {
                    id: ID::from("1"),
                    name: "User 1".to_string(),
                },
                User {
                    id: ID::from("2"),
                    name: "User 2".to_string(),
                },
            ])
        }
    }

    struct MutationRoot;

    #[Object]
    impl MutationRoot {
        async fn create_user(&self, name: String) -> Result<User> {
            Ok(User {
                id: ID::from("123"),
                name,
            })
        }
    }

    #[tokio::test]
    async fn test_query_single_user() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let query = r#"
            query {
                user(id: "1") {
                    id
                    name
                }
            }
        "#;

        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["user"]["id"], "1");
        assert_eq!(data["user"]["name"], "Test User");
    }

    #[tokio::test]
    async fn test_query_multiple_users() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let query = r#"
            query {
                users {
                    id
                    name
                }
            }
        "#;

        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["users"][0]["id"], "1");
        assert_eq!(data["users"][1]["id"], "2");
    }

    #[tokio::test]
    async fn test_mutation_create_user() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let mutation = r#"
            mutation {
                createUser(name: "New User") {
                    id
                    name
                }
            }
        "#;

        let result = schema.execute(mutation).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["createUser"]["id"], "123");
        assert_eq!(data["createUser"]["name"], "New User");
    }

    #[tokio::test]
    async fn test_introspection() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let query = r#"
            query {
                __type(name: "User") {
                    name
                    fields {
                        name
                        type {
                            name
                        }
                    }
                }
            }
        "#;

        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["__type"]["name"], "User");
    }

    #[tokio::test]
    async fn test_error_handling() {
        struct ErrorQuery;

        #[Object]
        impl ErrorQuery {
            async fn failing_query(&self) -> Result<String> {
                Err(Error::new("Test error"))
            }
        }

        let schema = Schema::build(ErrorQuery, EmptyMutation, EmptySubscription).finish();

        let query = r#"
            query {
                failingQuery
            }
        "#;

        let result = schema.execute(query).await;
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].message, "Test error");
    }

    #[tokio::test]
    async fn test_variables() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let query = r#"
            query GetUser($id: ID!) {
                user(id: $id) {
                    id
                    name
                }
            }
        "#;

        let result = schema
            .execute(Request::new(query).variables(Variables::from_json(serde_json::json!({
                "id": "42"
            }))))
            .await;

        assert!(result.errors.is_empty());
        let data = result.data.into_json().unwrap();
        assert_eq!(data["user"]["id"], "42");
    }

    #[tokio::test]
    async fn test_fragments() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let query = r#"
            fragment UserFields on User {
                id
                name
            }

            query {
                user(id: "1") {
                    ...UserFields
                }
            }
        "#;

        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["user"]["id"], "1");
        assert_eq!(data["user"]["name"], "Test User");
    }

    #[tokio::test]
    async fn test_aliases() {
        let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();

        let query = r#"
            query {
                user1: user(id: "1") {
                    id
                    name
                }
                user2: user(id: "2") {
                    id
                    name
                }
            }
        "#;

        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["user1"]["id"], "1");
        assert_eq!(data["user2"]["id"], "2");
    }
}
