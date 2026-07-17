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

pub use async_graphql::dataloader::DataLoader;
pub use async_graphql::{
    self, Context, EmptyMutation, EmptySubscription, Error, ErrorExtensions, InputObject, Object,
    Result, Schema, SimpleObject, Subscription, ID,
};
pub use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};

use axum::{
    extract::{FromRequest, Request as AxumRequest, State},
    http::request::Parts,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::sync::Arc;

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

/// Create a GraphQL router that threads per-request context into schema execution.
///
/// The plain [`graphql_router`] calls `schema.execute(req)` with **no** `.data(..)`
/// injection, which means the guards shipped in [`crate::auth`] ([`AuthGuard`],
/// [`RoleGuard`], and [`get_auth_user`]) are unreachable: nothing ever inserts the
/// [`AuthUser`] they read out of `ctx.data`. This variant fixes that.
///
/// `context_fn` is invoked for every request with the incoming HTTP request's
/// `Parts` (headers, extensions, uri, method). Return `Some(user)` to make an
/// [`AuthUser`] available in the GraphQL [`Context`] via `ctx.data::<AuthUser>()`,
/// or `None` for an unauthenticated request (the guards then reject it with a
/// GraphQL error instead of authorizing it). The request's `HeaderMap` is always
/// injected too, so resolvers can read request globals via `ctx.data::<HeaderMap>()`.
///
/// # Wiring the framework's authenticated user
///
/// Upstream `rf-auth` middleware (e.g. `auth_middleware` / `auth_layer`) inserts the
/// verified `Claims` into the request extensions. Map those into an
/// [`AuthUser`] in `context_fn`:
///
/// ```no_run
/// use rf_graphql::*;
/// use async_graphql::*;
///
/// struct QueryRoot;
///
/// #[Object]
/// impl QueryRoot {
///     #[graphql(guard = "AuthGuard")]
///     async fn me(&self, ctx: &Context<'_>) -> Result<String> {
///         Ok(get_auth_user(ctx)?.username.clone())
///     }
/// }
///
/// # async fn example() {
/// let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();
///
/// // Read whatever your auth middleware placed in the request and build an AuthUser.
/// let app = graphql_router_with_context(schema, |parts| {
///     let bearer = parts
///         .headers
///         .get(axum::http::header::AUTHORIZATION)?
///         .to_str()
///         .ok()?
///         .strip_prefix("Bearer ")?
///         .trim()
///         .parse::<i64>()
///         .ok()?;
///     Some(AuthUser {
///         id: bearer,
///         username: format!("user-{bearer}"),
///         roles: vec!["user".into()],
///     })
/// });
/// # let _ = app;
/// # }
/// ```
pub fn graphql_router_with_context<Q, M, S, F>(schema: Schema<Q, M, S>, context_fn: F) -> Router
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
    F: Fn(&Parts) -> Option<AuthUser> + Send + Sync + 'static,
{
    let state = (Arc::new(schema), Arc::new(context_fn));

    Router::new()
        .route(
            "/graphql",
            post(graphql_handler_with_context::<Q, M, S, F>),
        )
        .with_state(state)
}

/// GraphQL handler that injects per-request auth (and headers) into `ctx.data`.
async fn graphql_handler_with_context<Q, M, S, F>(
    State((schema, context_fn)): State<(Arc<Schema<Q, M, S>>, Arc<F>)>,
    request: AxumRequest,
) -> Response
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
    F: Fn(&Parts) -> Option<AuthUser> + Send + Sync + 'static,
{
    // Split the request so we can inspect headers/extensions before the body is
    // consumed by the GraphQL parser, then reassemble it for GraphQLRequest.
    let (parts, body) = request.into_parts();
    let auth_user = context_fn(&parts);
    let headers = parts.headers.clone();
    let request = AxumRequest::from_parts(parts, body);

    let mut gql_request =
        match GraphQLRequest::<async_graphql_axum::rejection::GraphQLRejection>::from_request(
            request,
            &(),
        )
        .await
        {
            Ok(req) => req.into_inner(),
            // Malformed GraphQL request: return the parser's own response, never panic.
            Err(rejection) => return rejection.into_response(),
        };

    // Request globals: headers are always available to resolvers.
    gql_request = gql_request.data(headers);
    // The load-bearing injection: make the framework's authenticated user reachable
    // so AuthGuard / RoleGuard / get_auth_user actually work.
    if let Some(user) = auth_user {
        gql_request = gql_request.data(user);
    }

    let response: GraphQLResponse = schema.execute(gql_request).await.into();
    response.into_response()
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

// Module exports
pub mod auth;
pub mod dataloader;
pub mod errors;
pub mod pagination;
pub mod relationships;
pub mod schema;

// Re-export commonly used items
pub use auth::{get_auth_user, AuthGuard, AuthUser, RoleGuard};
pub use dataloader::BatchLoader;
pub use errors::{
    error_with_code, forbidden_error, not_found_error, unauthorized_error, validation_error,
    ErrorCode, GraphQLResult, ResultExt,
};
pub use pagination::{
    decode_cursor, encode_cursor, Connection, CursorPaginationInput, Edge, OffsetPaginationInput,
    PageInfo, PaginatedResult,
};
pub use relationships::{BelongsTo, BelongsToMany, HasMany, HasOne, HasRelationships};
pub use schema::{build_schema, GraphQLSchema, SchemaBuilder};

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
            .execute(
                Request::new(query).variables(Variables::from_json(serde_json::json!({
                    "id": "42"
                }))),
            )
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

    /// Regression: `graphql_router_with_context` must populate `ctx.data::<AuthUser>()`
    /// so the shipped `AuthGuard` authorizes an authenticated request and rejects an
    /// unauthenticated one with a GraphQL error (never a panic). Before this router
    /// existed, `graphql_router` executed the schema with no `.data(..)`, so the guard
    /// was unreachable and every guarded field failed as "Unauthorized".
    #[tokio::test]
    async fn test_context_router_injects_auth_user() {
        use crate::auth::{get_auth_user, AuthGuard, AuthUser};
        use axum::{body::Body, http::Request as HttpRequest};
        use tower::ServiceExt;

        struct GuardedQuery;

        #[Object]
        impl GuardedQuery {
            #[graphql(guard = "AuthGuard")]
            async fn me(&self, ctx: &Context<'_>) -> Result<String> {
                Ok(get_auth_user(ctx)?.username.clone())
            }
        }

        let schema = Schema::build(GuardedQuery, EmptyMutation, EmptySubscription).finish();

        // context_fn: a `Bearer <id>` header authenticates; anything else is a guest.
        let app = graphql_router_with_context(schema, |parts| {
            let id = parts
                .headers
                .get(axum::http::header::AUTHORIZATION)?
                .to_str()
                .ok()?
                .strip_prefix("Bearer ")?
                .trim()
                .parse::<i64>()
                .ok()?;
            Some(AuthUser {
                id,
                username: format!("user-{id}"),
                roles: vec!["user".into()],
            })
        });

        let query = r#"{"query":"{ me }"}"#;

        // Authenticated request: guard passes, resolver returns the username.
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .header("Authorization", "Bearer 7")
                    .body(Body::from(query))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["data"]["me"], "user-7",
            "authenticated request should reach the guarded resolver: {json}"
        );
        assert!(
            json.get("errors").is_none(),
            "no errors expected for authenticated request: {json}"
        );

        // Unauthenticated request: guard rejects with a GraphQL error, no panic.
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .body(Body::from(query))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["data"]["me"].is_null(),
            "guarded field must not resolve for a guest: {json}"
        );
        assert!(
            json["errors"][0]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("Unauthorized"),
            "guest should get an Unauthorized GraphQL error: {json}"
        );
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
