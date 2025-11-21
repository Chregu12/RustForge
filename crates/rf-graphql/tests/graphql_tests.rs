//! Comprehensive GraphQL tests
//!
//! Tests all major features of the GraphQL implementation.

use async_graphql::{
    dataloader::DataLoader, Context, EmptySubscription, InputObject, Object, Request, Result,
    Schema, SimpleObject, Variables, ID,
};
use rf_graphql::{
    auth::{AuthGuard, AuthUser, RoleGuard},
    dataloader::BatchLoader,
    errors::{not_found_error, validation_error, ErrorCode},
    pagination::{
        encode_cursor, Connection, Edge, OffsetPaginationInput, PageInfo, PaginatedResult,
    },
};
use std::collections::HashMap;

// ============================================================================
// Test Models
// ============================================================================

#[derive(Debug, Clone, SimpleObject, PartialEq)]
struct User {
    id: ID,
    name: String,
    email: String,
}

#[derive(Debug, Clone, SimpleObject, PartialEq)]
struct Post {
    id: ID,
    title: String,
    user_id: ID,
}

#[derive(InputObject)]
struct CreateUserInput {
    name: String,
    email: String,
}

// ============================================================================
// Test Query/Mutation Roots
// ============================================================================

struct TestQuery;

#[Object]
impl TestQuery {
    async fn user(&self, id: ID) -> Result<User> {
        Ok(User {
            id,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        })
    }

    async fn users(&self) -> Result<Vec<User>> {
        Ok(vec![
            User {
                id: ID::from("1"),
                name: "User 1".to_string(),
                email: "user1@example.com".to_string(),
            },
            User {
                id: ID::from("2"),
                name: "User 2".to_string(),
                email: "user2@example.com".to_string(),
            },
        ])
    }

    async fn users_paginated(
        &self,
        pagination: Option<OffsetPaginationInput>,
    ) -> Result<PaginatedResult<User>> {
        let all_users = vec![
            User {
                id: ID::from("1"),
                name: "User 1".to_string(),
                email: "user1@example.com".to_string(),
            },
            User {
                id: ID::from("2"),
                name: "User 2".to_string(),
                email: "user2@example.com".to_string(),
            },
        ];

        let pagination = pagination.unwrap_or_default();
        Ok(PaginatedResult::new(
            all_users,
            pagination.page.unwrap_or(0),
            pagination.per_page.unwrap_or(10),
            2,
        ))
    }

    async fn search_users(&self, query: String) -> Result<Vec<User>> {
        if query.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![User {
            id: ID::from("1"),
            name: "User 1".to_string(),
            email: "user1@example.com".to_string(),
        }])
    }

    #[graphql(guard = "AuthGuard")]
    async fn protected_user(&self, ctx: &Context<'_>) -> Result<User> {
        let auth_user = rf_graphql::auth::get_auth_user(ctx)?;
        Ok(User {
            id: ID::from(auth_user.id.to_string()),
            name: auth_user.username.clone(),
            email: "auth@example.com".to_string(),
        })
    }

    async fn error_query(&self) -> Result<String> {
        Err(not_found_error("Resource", 123))
    }
}

struct TestMutation;

#[Object]
impl TestMutation {
    async fn create_user(&self, input: CreateUserInput) -> Result<User> {
        if !input.email.contains('@') {
            return Err(validation_error("Invalid email", Some("email")));
        }

        Ok(User {
            id: ID::from("123"),
            name: input.name,
            email: input.email,
        })
    }

    async fn update_user(&self, id: ID, name: Option<String>) -> Result<User> {
        Ok(User {
            id,
            name: name.unwrap_or_else(|| "Updated".to_string()),
            email: "updated@example.com".to_string(),
        })
    }

    async fn delete_user(&self, id: ID) -> Result<bool> {
        Ok(true)
    }

    #[graphql(guard = "AuthGuard")]
    async fn protected_mutation(&self) -> Result<bool> {
        Ok(true)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_query_single_object() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let query = r#"
        query {
            user(id: "1") {
                id
                name
                email
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty(), "Expected no errors");

    let data = result.data.into_json().unwrap();
    assert_eq!(data["user"]["id"], "1");
    assert_eq!(data["user"]["name"], "Test User");
}

#[tokio::test]
async fn test_query_list_of_objects() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

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
    assert_eq!(data["users"].as_array().unwrap().len(), 2);
    assert_eq!(data["users"][0]["id"], "1");
    assert_eq!(data["users"][1]["id"], "2");
}

#[tokio::test]
async fn test_mutation_create() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let mutation = r#"
        mutation {
            createUser(input: {
                name: "New User",
                email: "new@example.com"
            }) {
                id
                name
                email
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
async fn test_mutation_update() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let mutation = r#"
        mutation {
            updateUser(id: "1", name: "Updated Name") {
                id
                name
            }
        }
    "#;

    let result = schema.execute(mutation).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert_eq!(data["updateUser"]["name"], "Updated Name");
}

#[tokio::test]
async fn test_mutation_delete() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let mutation = r#"
        mutation {
            deleteUser(id: "1")
        }
    "#;

    let result = schema.execute(mutation).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert_eq!(data["deleteUser"], true);
}

#[tokio::test]
async fn test_input_validation() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let mutation = r#"
        mutation {
            createUser(input: {
                name: "Test",
                email: "invalid-email"
            }) {
                id
            }
        }
    "#;

    let result = schema.execute(mutation).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Invalid email"));
}

#[tokio::test]
async fn test_error_handling() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let query = r#"
        query {
            errorQuery
        }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("not found"));
}

#[tokio::test]
async fn test_pagination() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let query = r#"
        query {
            usersPaginated(pagination: { page: 0, perPage: 10 }) {
                data {
                    id
                    name
                }
                page
                perPage
                total
                totalPages
                hasNextPage
                hasPreviousPage
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert_eq!(data["usersPaginated"]["page"], 0);
    assert_eq!(data["usersPaginated"]["perPage"], 10);
    assert_eq!(data["usersPaginated"]["total"], 2);
}

#[tokio::test]
async fn test_filtering() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let query = r#"
        query {
            searchUsers(query: "User") {
                id
                name
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert!(data["searchUsers"].is_array());
}

#[tokio::test]
async fn test_authentication_required() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let query = r#"
        query {
            protectedUser {
                id
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("Unauthorized"));
}

#[tokio::test]
async fn test_authentication_with_user() {
    let auth_user = AuthUser {
        id: 1,
        username: "john".to_string(),
        roles: vec!["user".to_string()],
    };

    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription)
        .data(auth_user)
        .finish();

    let query = r#"
        query {
            protectedUser {
                id
                name
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert_eq!(data["protectedUser"]["name"], "john");
}

#[tokio::test]
async fn test_variables() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

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
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let query = r#"
        fragment UserFields on User {
            id
            name
            email
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
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

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

#[tokio::test]
async fn test_introspection() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

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
async fn test_dataloader() {
    use async_graphql::dataloader::Loader;
    use std::sync::Arc;

    struct UserLoader;

    impl Loader<ID> for UserLoader {
        type Value = User;
        type Error = Arc<String>;

        fn load(&self, keys: &[ID]) -> impl std::future::Future<Output = Result<HashMap<ID, Self::Value>, Self::Error>> + Send {
            let keys = keys.to_vec();
            async move {
                let users: HashMap<ID, User> = keys
                    .iter()
                    .map(|id| {
                        (
                            id.clone(),
                            User {
                                id: id.clone(),
                                name: format!("User {}", id.as_str()),
                                email: format!("user{}@example.com", id.as_str()),
                            },
                        )
                    })
                    .collect();

                Ok(users)
            }
        }
    }

    let loader = DataLoader::new(UserLoader, tokio::spawn);

    // Load users in batch
    let user1 = loader.load_one(ID::from("1")).await.unwrap();
    let user2 = loader.load_one(ID::from("2")).await.unwrap();

    assert_eq!(user1.unwrap().name, "User 1");
    assert_eq!(user2.unwrap().name, "User 2");
}

#[tokio::test]
async fn test_performance_simple_query() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let start = std::time::Instant::now();

    for _ in 0..100 {
        let query = r#"{ users { id name } }"#;
        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());
    }

    let duration = start.elapsed();
    println!("100 queries took: {:?}", duration);

    // Should complete in reasonable time (< 1 second for 100 queries)
    assert!(duration.as_secs() < 1);
}

#[tokio::test]
async fn test_concurrent_queries() {
    let schema = Schema::build(TestQuery, TestMutation, EmptySubscription).finish();

    let mut handles = vec![];

    for i in 0..10 {
        let schema = schema.clone();
        let handle = tokio::spawn(async move {
            let query = format!(r#"{{ user(id: "{}") {{ id name }} }}"#, i);
            schema.execute(&query).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    for result in results {
        let response = result.unwrap();
        assert!(response.errors.is_empty());
    }
}
