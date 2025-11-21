# rf-graphql: Complete GraphQL Support for RustForge

Production-ready GraphQL implementation providing a flexible, type-safe API layer alongside REST.

## 🚀 Features

- **Complete GraphQL Support**: Queries, mutations, and subscriptions
- **DataLoader**: N+1 query prevention with efficient batch loading
- **Relationships**: Built-in support for has-many, belongs-to, and many-to-many relationships
- **Pagination**: Both cursor-based and offset-based pagination
- **Authentication**: Guards and middleware for securing resolvers
- **Error Handling**: Type-safe error codes and extensions
- **GraphQL Playground**: Interactive API explorer
- **Type Safety**: Leverages Rust's type system for compile-time guarantees

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-graphql = "0.1"
async-graphql = "7.0"
async-graphql-axum = "7.0"
axum = "0.8"
tokio = { version = "1.0", features = ["full"] }
```

## 🎯 Quick Start

### Basic Setup

```rust
use rf_graphql::*;
use async_graphql::{Object, SimpleObject, Result};

// Define your types
#[derive(SimpleObject)]
struct User {
    id: ID,
    name: String,
    email: String,
}

// Query root
struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn user(&self, id: ID) -> Result<User> {
        Ok(User {
            id,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        })
    }

    async fn users(&self) -> Result<Vec<User>> {
        // Load from database
        Ok(vec![])
    }
}

// Mutation root
struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_user(&self, name: String, email: String) -> Result<User> {
        // Insert into database
        Ok(User {
            id: ID::from("123"),
            name,
            email,
        })
    }
}

#[tokio::main]
async fn main() {
    // Build schema
    let schema = build_schema(QueryRoot, MutationRoot);

    // Create router
    let app = Router::new()
        .merge(graphql_router(schema))
        .merge(graphql_playground_router());

    // Start server
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## 📚 Core Concepts

### 1. Queries

Queries fetch data from your API:

```rust
#[Object]
impl QueryRoot {
    /// Get user by ID
    async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<User> {
        let db = ctx.data::<Database>()?;
        db.get_user(&id)
            .ok_or_else(|| not_found_error("User", &id))
    }

    /// Get all users
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_all_users())
    }

    /// Search users
    async fn search_users(&self, ctx: &Context<'_>, query: String) -> Result<Vec<User>> {
        let db = ctx.data::<Database>()?;
        Ok(db.search_users(&query))
    }
}
```

GraphQL query examples:

```graphql
# Get single user
query {
  user(id: "1") {
    id
    name
    email
  }
}

# Get all users
query {
  users {
    id
    name
  }
}

# Search users
query {
  searchUsers(query: "john") {
    id
    name
    email
  }
}
```

### 2. Mutations

Mutations modify data:

```rust
#[Object]
impl MutationRoot {
    /// Create user
    async fn create_user(
        &self,
        ctx: &Context<'_>,
        input: CreateUserInput,
    ) -> Result<User> {
        // Validate
        if !input.email.contains('@') {
            return Err(validation_error("Invalid email", Some("email")));
        }

        // Insert into database
        let db = ctx.data::<Database>()?;
        db.create_user(input)
    }

    /// Update user
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateUserInput,
    ) -> Result<User> {
        let db = ctx.data::<Database>()?;
        db.update_user(id, input)
    }

    /// Delete user
    async fn delete_user(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let db = ctx.data::<Database>()?;
        db.delete_user(&id)
    }
}
```

GraphQL mutation examples:

```graphql
# Create user
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

# Update user
mutation {
  updateUser(id: "1", input: {
    name: "Alice Updated"
  }) {
    id
    name
  }
}

# Delete user
mutation {
  deleteUser(id: "1")
}
```

### 3. Relationships

Define relationships between types:

```rust
#[derive(SimpleObject)]
#[graphql(complex)]
struct User {
    id: ID,
    name: String,
}

#[ComplexObject]
impl User {
    /// Has many posts
    async fn posts(&self, ctx: &Context<'_>) -> Result<Vec<Post>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_posts_by_user(&self.id))
    }

    /// Post count
    async fn post_count(&self, ctx: &Context<'_>) -> Result<i64> {
        let db = ctx.data::<Database>()?;
        Ok(db.count_posts_by_user(&self.id))
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Post {
    id: ID,
    title: String,
    user_id: ID,
}

#[ComplexObject]
impl Post {
    /// Belongs to user
    async fn author(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let loader = ctx.data::<DataLoader<UserLoader>>()?;
        Ok(loader.load_one(self.user_id.clone()).await?)
    }
}
```

Query with relationships:

```graphql
query {
  user(id: "1") {
    id
    name
    posts {
      id
      title
    }
    postCount
  }
}
```

### 4. DataLoader (N+1 Prevention)

Prevent N+1 queries with efficient batch loading:

```rust
use rf_graphql::dataloader::BatchLoader;

struct UserLoader {
    db: Database,
}

#[async_trait::async_trait]
impl BatchLoader<ID, User> for UserLoader {
    type Error = String;

    async fn batch_load(&self, keys: &[ID]) -> Result<HashMap<ID, User>, Self::Error> {
        // Single database query for all keys
        self.db.get_users_by_ids(keys)
            .map(|users| users.into_iter()
                .map(|u| (u.id.clone(), u))
                .collect())
    }
}

// Register in schema
let loader = DataLoader::new(UserLoader { db: db.clone() }, tokio::spawn);
let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .data(loader)
    .finish();
```

### 5. Pagination

#### Offset-Based Pagination

```rust
#[Object]
impl QueryRoot {
    async fn users_paginated(
        &self,
        ctx: &Context<'_>,
        pagination: Option<OffsetPaginationInput>,
    ) -> Result<PaginatedResult<User>> {
        let db = ctx.data::<Database>()?;
        let pagination = pagination.unwrap_or_default();

        let users = db.get_users_paginated(
            pagination.offset(),
            pagination.limit(),
        );

        let total = db.count_users();

        Ok(PaginatedResult::new(
            users,
            pagination.page.unwrap_or(0),
            pagination.per_page.unwrap_or(10),
            total,
        ))
    }
}
```

Query:

```graphql
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
```

#### Cursor-Based Pagination

```rust
use rf_graphql::pagination::{Connection, Edge, encode_cursor};

async fn users_connection(&self) -> Result<Connection<User>> {
    let users = get_users();

    let edges = users.into_iter()
        .map(|user| Edge {
            cursor: encode_cursor(user.id),
            node: user,
        })
        .collect();

    let page_info = PageInfo {
        has_next_page: true,
        has_previous_page: false,
        start_cursor: Some(edges.first().map(|e| e.cursor.clone()).unwrap_or_default()),
        end_cursor: Some(edges.last().map(|e| e.cursor.clone()).unwrap_or_default()),
    };

    Ok(Connection::new(edges, page_info))
}
```

### 6. Authentication & Authorization

```rust
use rf_graphql::auth::{AuthGuard, AuthUser, RoleGuard};

#[Object]
impl QueryRoot {
    /// Requires authentication
    #[graphql(guard = "AuthGuard")]
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let auth_user = get_auth_user(ctx)?;
        // Load user from database
        Ok(get_user_by_id(auth_user.id))
    }

    /// Requires admin role
    #[graphql(guard = "RoleGuard::single(\"admin\")")]
    async fn admin_data(&self) -> Result<String> {
        Ok("Sensitive admin data".to_string())
    }
}

// Add auth user to context
let auth_user = AuthUser {
    id: 1,
    username: "john".to_string(),
    roles: vec!["user".to_string(), "admin".to_string()],
};

let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .data(auth_user)
    .finish();
```

### 7. Error Handling

```rust
use rf_graphql::errors::{
    ErrorCode, validation_error, not_found_error,
    unauthorized_error, forbidden_error
};

#[Object]
impl MutationRoot {
    async fn create_user(&self, input: CreateUserInput) -> Result<User> {
        // Validation error
        if !input.email.contains('@') {
            return Err(validation_error("Invalid email format", Some("email")));
        }

        // Not found error
        if user_exists(&input.email) {
            return Err(error_with_code(
                "User already exists",
                ErrorCode::BadRequest
            ));
        }

        // Database operation
        db.create_user(input)
            .to_graphql_result(ErrorCode::DatabaseError)
    }
}
```

## 🎨 Examples

### Complete User/Post API

See `examples/user_posts_api.rs` for a full-featured example with:
- User and Post models
- CRUD operations
- Relationships
- DataLoader
- Pagination
- Authentication
- Error handling

Run the example:

```bash
cargo run --example user_posts_api
```

### Frontend Integration

See `examples/frontend_client.html` for a complete HTML/JavaScript client demonstrating:
- Query execution
- Mutation execution
- Variable handling
- Error handling
- Real-time updates

## 🧪 Testing

The crate includes comprehensive tests covering:
- Query execution
- Mutations
- Relationships
- DataLoader
- Pagination
- Authentication
- Error handling
- Performance

Run tests:

```bash
cargo test
```

Run with coverage:

```bash
cargo test -- --test-threads=1 --nocapture
```

## 📊 GraphQL vs REST Comparison

| Feature | GraphQL | REST |
|---------|---------|------|
| **Data Fetching** | Request exactly what you need | Fixed endpoints return fixed data |
| **Over-fetching** | ✅ Never | ❌ Common |
| **Under-fetching** | ✅ Never | ❌ Requires multiple requests |
| **Endpoints** | Single endpoint | Multiple endpoints |
| **Versioning** | ✅ Schema evolution | Versioned URLs |
| **Type System** | ✅ Strong typing | Depends on implementation |
| **Documentation** | ✅ Self-documenting | Manual |
| **Real-time** | ✅ Subscriptions | Requires WebSocket setup |

## ⚡ Performance Tips

1. **Use DataLoader**: Always use DataLoader for relationships to prevent N+1 queries

2. **Limit Query Depth**: Set depth limits to prevent deep nested queries

```rust
let schema = SchemaBuilder::new(QueryRoot, MutationRoot)
    .depth_limit(10)
    .complexity_limit(100)
    .build();
```

3. **Pagination**: Use pagination for large datasets

4. **Field-level Caching**: Cache expensive field resolvers

5. **Database Optimization**: Use proper indexes and optimize queries

## 🔒 Security Best Practices

1. **Authentication**: Always validate authentication for protected resolvers

2. **Authorization**: Use guards to enforce role-based access control

3. **Input Validation**: Validate all input data

4. **Rate Limiting**: Implement rate limiting to prevent abuse

5. **Query Complexity**: Limit query depth and complexity

6. **Error Messages**: Don't expose sensitive information in error messages

## 📖 GraphQL Schema Example

```graphql
type User {
  id: ID!
  name: String!
  email: String!
  posts: [Post!]!
  postCount: Int!
}

type Post {
  id: ID!
  title: String!
  body: String!
  published: Boolean!
  author: User!
}

input CreateUserInput {
  name: String!
  email: String!
}

input UpdateUserInput {
  name: String
  email: String
}

type Query {
  user(id: ID!): User
  users: [User!]!
  usersPaginated(pagination: OffsetPaginationInput): PaginatedUserResult!
  post(id: ID!): Post
  posts(publishedOnly: Boolean): [Post!]!
  searchPosts(query: String!): [Post!]!
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
  deleteUser(id: ID!): Boolean!
  createPost(input: CreatePostInput!): Post!
  updatePost(id: ID!, input: UpdatePostInput!): Post!
  deletePost(id: ID!): Boolean!
}
```

## 🛠️ Advanced Features

### Custom Scalars

```rust
use async_graphql::Scalar;

#[derive(Clone)]
struct DateTime(chrono::DateTime<chrono::Utc>);

#[Scalar]
impl ScalarType for DateTime {
    fn parse(value: Value) -> InputValueResult<Self> {
        // Parse implementation
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_rfc3339())
    }
}
```

### Subscriptions

```rust
use async_graphql::{Subscription, futures_util::Stream};

struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn user_created(&self) -> impl Stream<Item = User> {
        // Return stream of user creation events
    }
}
```

### Directives

```rust
#[derive(SimpleObject)]
struct User {
    id: ID,
    name: String,
    #[graphql(deprecation = "Use email instead")]
    email_address: String,
    email: String,
}
```

## 🚀 Production Deployment

1. **Enable Production Mode**: Set optimizations in Cargo.toml

```toml
[profile.release]
lto = true
codegen-units = 1
```

2. **CORS Configuration**: Configure CORS for production

```rust
use tower_http::cors::CorsLayer;

let app = Router::new()
    .merge(graphql_router(schema))
    .layer(CorsLayer::permissive());
```

3. **Monitoring**: Add metrics and logging

4. **Rate Limiting**: Implement rate limiting middleware

5. **SSL/TLS**: Use HTTPS in production

## 📝 License

MIT OR Apache-2.0

## 🤝 Contributing

Contributions welcome! Please read our contributing guidelines.

## 📚 Further Reading

- [GraphQL Official Documentation](https://graphql.org/)
- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [RustForge Documentation](https://rustforge.dev/)

## ✨ Credits

Built with ❤️ for the RustForge framework using [async-graphql](https://github.com/async-graphql/async-graphql).
