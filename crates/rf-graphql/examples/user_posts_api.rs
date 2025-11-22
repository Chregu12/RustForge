//! Complete User/Post GraphQL API Example
//!
//! Demonstrates a full-featured GraphQL API with:
//! - Queries and mutations
//! - Relationships (User -> Posts)
//! - DataLoader for N+1 prevention
//! - Pagination
//! - Authentication
//! - Error handling
//!
//! Run with: cargo run --example user_posts_api

use async_graphql::{
    dataloader::DataLoader, ComplexObject, Context, EmptySubscription, InputObject, Object, Result,
    Schema, SimpleObject, ID,
};
use rf_graphql::{
    auth::{AuthGuard, AuthUser},
    dataloader::BatchLoader,
    errors::{not_found_error, validation_error, ErrorCode, ResultExt},
    pagination::{
        encode_cursor, Connection, Edge, OffsetPaginationInput, PageInfo, PaginatedResult,
    },
    relationships::{BelongsTo, HasMany, HasRelationships},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Models
// ============================================================================

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
struct User {
    id: ID,
    name: String,
    email: String,
}

#[ComplexObject]
impl User {
    /// Get user's posts (has many relationship)
    async fn posts(&self, ctx: &Context<'_>) -> Result<Vec<Post>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_posts_by_user(&self.id))
    }

    /// Get post count
    async fn post_count(&self, ctx: &Context<'_>) -> Result<i64> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_posts_by_user(&self.id).len() as i64)
    }
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
struct Post {
    id: ID,
    title: String,
    body: String,
    user_id: ID,
    published: bool,
}

#[ComplexObject]
impl Post {
    /// Get post's author (belongs to relationship)
    async fn author(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let loader = ctx.data::<DataLoader<UserLoader>>()?;
        Ok(loader.load_one(self.user_id.clone()).await?)
    }
}

// ============================================================================
// Input Types
// ============================================================================

#[derive(InputObject)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(InputObject)]
struct UpdateUserInput {
    name: Option<String>,
    email: Option<String>,
}

#[derive(InputObject)]
struct CreatePostInput {
    title: String,
    body: String,
    published: Option<bool>,
}

#[derive(InputObject)]
struct UpdatePostInput {
    title: Option<String>,
    body: Option<String>,
    published: Option<bool>,
}

// ============================================================================
// Database (In-memory for example)
// ============================================================================

#[derive(Clone)]
struct Database {
    users: Arc<Mutex<HashMap<ID, User>>>,
    posts: Arc<Mutex<HashMap<ID, Post>>>,
    next_user_id: Arc<Mutex<i64>>,
    next_post_id: Arc<Mutex<i64>>,
}

impl Database {
    fn new() -> Self {
        let db = Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            posts: Arc::new(Mutex::new(HashMap::new())),
            next_user_id: Arc::new(Mutex::new(1)),
            next_post_id: Arc::new(Mutex::new(1)),
        };

        // Seed data
        db.seed_data();
        db
    }

    fn seed_data(&self) {
        // Create users
        for i in 1..=3 {
            let id = ID::from(i.to_string());
            let user = User {
                id: id.clone(),
                name: format!("User {}", i),
                email: format!("user{}@example.com", i),
            };
            self.users.lock().unwrap().insert(id, user);
        }

        // Create posts
        for i in 1..=10 {
            let user_id = ((i - 1) % 3 + 1).to_string();
            let id = ID::from(i.to_string());
            let post = Post {
                id: id.clone(),
                title: format!("Post {}", i),
                body: format!("This is the body of post {}", i),
                user_id: ID::from(user_id),
                published: i % 2 == 0,
            };
            self.posts.lock().unwrap().insert(id, post);
        }

        *self.next_user_id.lock().unwrap() = 4;
        *self.next_post_id.lock().unwrap() = 11;
    }

    fn get_user(&self, id: &ID) -> Option<User> {
        self.users.lock().unwrap().get(id).cloned()
    }

    fn get_all_users(&self) -> Vec<User> {
        self.users.lock().unwrap().values().cloned().collect()
    }

    fn create_user(&self, input: CreateUserInput) -> Result<User> {
        let mut next_id = self.next_user_id.lock().unwrap();
        let id = ID::from(next_id.to_string());
        *next_id += 1;

        let user = User {
            id: id.clone(),
            name: input.name,
            email: input.email,
        };

        self.users.lock().unwrap().insert(id, user.clone());
        Ok(user)
    }

    fn update_user(&self, id: ID, input: UpdateUserInput) -> Result<User> {
        let mut users = self.users.lock().unwrap();
        let user = users
            .get_mut(&id)
            .ok_or_else(|| not_found_error("User", id.as_str()))?;

        if let Some(name) = input.name {
            user.name = name;
        }
        if let Some(email) = input.email {
            user.email = email;
        }

        Ok(user.clone())
    }

    fn delete_user(&self, id: &ID) -> Result<bool> {
        let mut users = self.users.lock().unwrap();
        users
            .remove(id)
            .ok_or_else(|| not_found_error("User", id.as_str()))?;
        Ok(true)
    }

    fn get_post(&self, id: &ID) -> Option<Post> {
        self.posts.lock().unwrap().get(id).cloned()
    }

    fn get_all_posts(&self) -> Vec<Post> {
        self.posts.lock().unwrap().values().cloned().collect()
    }

    fn get_posts_by_user(&self, user_id: &ID) -> Vec<Post> {
        self.posts
            .lock()
            .unwrap()
            .values()
            .filter(|p| &p.user_id == user_id)
            .cloned()
            .collect()
    }

    fn create_post(&self, user_id: ID, input: CreatePostInput) -> Result<Post> {
        let mut next_id = self.next_post_id.lock().unwrap();
        let id = ID::from(next_id.to_string());
        *next_id += 1;

        let post = Post {
            id: id.clone(),
            title: input.title,
            body: input.body,
            user_id,
            published: input.published.unwrap_or(false),
        };

        self.posts.lock().unwrap().insert(id, post.clone());
        Ok(post)
    }

    fn update_post(&self, id: ID, input: UpdatePostInput) -> Result<Post> {
        let mut posts = self.posts.lock().unwrap();
        let post = posts
            .get_mut(&id)
            .ok_or_else(|| not_found_error("Post", id.as_str()))?;

        if let Some(title) = input.title {
            post.title = title;
        }
        if let Some(body) = input.body {
            post.body = body;
        }
        if let Some(published) = input.published {
            post.published = published;
        }

        Ok(post.clone())
    }

    fn delete_post(&self, id: &ID) -> Result<bool> {
        let mut posts = self.posts.lock().unwrap();
        posts
            .remove(id)
            .ok_or_else(|| not_found_error("Post", id.as_str()))?;
        Ok(true)
    }
}

// ============================================================================
// DataLoader
// ============================================================================

struct UserLoader {
    db: Database,
}

impl async_graphql::dataloader::Loader<ID> for UserLoader {
    type Value = User;
    type Error = std::sync::Arc<String>;

    fn load(
        &self,
        keys: &[ID],
    ) -> impl std::future::Future<Output = Result<HashMap<ID, Self::Value>, Self::Error>> + Send
    {
        let keys = keys.to_vec();
        let db = self.db.clone();
        async move {
            let users: HashMap<ID, User> = keys
                .iter()
                .filter_map(|id| db.get_user(id).map(|u| (id.clone(), u)))
                .collect();

            Ok(users)
        }
    }
}

// ============================================================================
// Query Root
// ============================================================================

struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get user by ID
    async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<Option<User>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_user(&id))
    }

    /// Get all users
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_all_users())
    }

    /// Get users with pagination
    async fn users_paginated(
        &self,
        ctx: &Context<'_>,
        pagination: Option<OffsetPaginationInput>,
    ) -> Result<PaginatedResult<User>> {
        let db = ctx.data::<Database>()?;
        let all_users = db.get_all_users();

        let pagination = pagination.unwrap_or_default();
        let offset = pagination.offset() as usize;
        let limit = pagination.limit() as usize;

        let paginated_users = all_users.iter().skip(offset).take(limit).cloned().collect();

        Ok(PaginatedResult::new(
            paginated_users,
            pagination.page.unwrap_or(0),
            pagination.per_page.unwrap_or(10),
            all_users.len() as i64,
        ))
    }

    /// Get post by ID
    async fn post(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Post>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_post(&id))
    }

    /// Get all posts
    async fn posts(&self, ctx: &Context<'_>, published_only: Option<bool>) -> Result<Vec<Post>> {
        let db = ctx.data::<Database>()?;
        let mut posts = db.get_all_posts();

        if let Some(true) = published_only {
            posts.retain(|p| p.published);
        }

        Ok(posts)
    }

    /// Search posts by title
    async fn search_posts(&self, ctx: &Context<'_>, query: String) -> Result<Vec<Post>> {
        let db = ctx.data::<Database>()?;
        let posts = db.get_all_posts();

        let results = posts
            .into_iter()
            .filter(|p| p.title.to_lowercase().contains(&query.to_lowercase()))
            .collect();

        Ok(results)
    }

    /// Get current authenticated user
    #[graphql(guard = "AuthGuard")]
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let auth_user = rf_graphql::auth::get_auth_user(ctx)?;
        let db = ctx.data::<Database>()?;

        db.get_user(&ID::from(auth_user.id.to_string()))
            .ok_or_else(|| not_found_error("User", &auth_user.id))
    }
}

// ============================================================================
// Mutation Root
// ============================================================================

struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new user
    async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<User> {
        // Validate email
        if !input.email.contains('@') {
            return Err(validation_error("Invalid email format", Some("email")));
        }

        let db = ctx.data::<Database>()?;
        db.create_user(input)
    }

    /// Update a user
    #[graphql(guard = "AuthGuard")]
    async fn update_user(&self, ctx: &Context<'_>, id: ID, input: UpdateUserInput) -> Result<User> {
        let db = ctx.data::<Database>()?;
        db.update_user(id, input)
    }

    /// Delete a user
    #[graphql(guard = "AuthGuard")]
    async fn delete_user(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let db = ctx.data::<Database>()?;
        db.delete_user(&id)
    }

    /// Create a new post
    #[graphql(guard = "AuthGuard")]
    async fn create_post(&self, ctx: &Context<'_>, input: CreatePostInput) -> Result<Post> {
        let auth_user = rf_graphql::auth::get_auth_user(ctx)?;
        let db = ctx.data::<Database>()?;

        db.create_post(ID::from(auth_user.id.to_string()), input)
    }

    /// Update a post
    #[graphql(guard = "AuthGuard")]
    async fn update_post(&self, ctx: &Context<'_>, id: ID, input: UpdatePostInput) -> Result<Post> {
        let db = ctx.data::<Database>()?;
        db.update_post(id, input)
    }

    /// Delete a post
    #[graphql(guard = "AuthGuard")]
    async fn delete_post(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let db = ctx.data::<Database>()?;
        db.delete_post(&id)
    }

    /// Publish a post
    #[graphql(guard = "AuthGuard")]
    async fn publish_post(&self, ctx: &Context<'_>, id: ID) -> Result<Post> {
        let db = ctx.data::<Database>()?;
        db.update_post(
            id,
            UpdatePostInput {
                title: None,
                body: None,
                published: Some(true),
            },
        )
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    println!("🚀 Starting GraphQL User/Post API Example...\n");

    // Create database
    let db = Database::new();

    // Create DataLoader
    let user_loader = DataLoader::new(UserLoader { db: db.clone() }, tokio::spawn);

    // Build schema
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(db)
        .data(user_loader)
        .finish();

    println!("✅ Schema built successfully!");
    println!("📊 Database seeded with sample data\n");

    // Example queries
    println!("=== Example Query: Get All Users ===");
    let query = r#"
        query {
            users {
                id
                name
                email
                postCount
            }
        }
    "#;

    let result = schema.execute(query).await;
    println!(
        "Result: {}\n",
        serde_json::to_string_pretty(&result).unwrap()
    );

    println!("=== Example Query: Get User with Posts ===");
    let query = r#"
        query {
            user(id: "1") {
                id
                name
                email
                posts {
                    id
                    title
                    published
                }
            }
        }
    "#;

    let result = schema.execute(query).await;
    println!(
        "Result: {}\n",
        serde_json::to_string_pretty(&result).unwrap()
    );

    println!("=== Example Query: Search Posts ===");
    let query = r#"
        query {
            searchPosts(query: "Post 1") {
                id
                title
                author {
                    name
                }
            }
        }
    "#;

    let result = schema.execute(query).await;
    println!(
        "Result: {}\n",
        serde_json::to_string_pretty(&result).unwrap()
    );

    println!("=== Example Mutation: Create User ===");
    let mutation = r#"
        mutation {
            createUser(input: {
                name: "Alice",
                email: "alice@example.com"
            }) {
                id
                name
                email
            }
        }
    "#;

    let result = schema.execute(mutation).await;
    println!(
        "Result: {}\n",
        serde_json::to_string_pretty(&result).unwrap()
    );

    println!("=== Example Query: Paginated Users ===");
    let query = r#"
        query {
            usersPaginated(pagination: { page: 0, perPage: 2 }) {
                data {
                    id
                    name
                }
                page
                perPage
                total
                totalPages
                hasNextPage
            }
        }
    "#;

    let result = schema.execute(query).await;
    println!(
        "Result: {}\n",
        serde_json::to_string_pretty(&result).unwrap()
    );

    println!("✨ Example completed successfully!");
    println!("\n💡 To start a GraphQL server, use rf_graphql::graphql_router()");
}
