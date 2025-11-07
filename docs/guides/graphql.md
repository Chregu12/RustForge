# GraphQL Support - RustForge Framework

RustForge comes with built-in GraphQL support powered by async-graphql, providing a modern and type-safe API layer.

## Features

- **Type-safe Schema** - Fully typed GraphQL schema with Rust's type system
- **Sea-ORM Integration** - Seamless integration with database models
- **Query & Mutations** - Complete CRUD operations
- **GraphQL Playground** - Interactive API explorer for development
- **DataLoader Support** - Efficient batch loading to prevent N+1 queries
- **Input Validation** - Built-in validation for all inputs
- **Error Handling** - Structured error responses with codes

## Quick Start

### 1. Start the Server

```bash
rustforge serve
```

### 2. Access GraphQL Playground

Open your browser and navigate to:

```
http://localhost:8000/graphql/playground
```

### 3. Run Your First Query

```graphql
query {
  products(limit: 5) {
    id
    name
    price
    stock
  }
}
```

## Schema Overview

### Types

#### Product

```graphql
type Product {
  id: Int!
  name: String!
  description: String
  price: String!
  stock: Int!
  sku: String!
  active: Boolean!
  createdAt: DateTime!
  updatedAt: DateTime!
}
```

#### Account

```graphql
type Account {
  id: Int!
  email: String!
  name: String!
  role: String!
  active: Boolean!
  createdAt: DateTime!
  updatedAt: DateTime!
}
```

## Queries

### Get Products

Get all products with pagination:

```graphql
query GetProducts {
  products(offset: 0, limit: 10) {
    id
    name
    price
    stock
    sku
  }
}
```

### Get Single Product

```graphql
query GetProduct {
  product(id: 1) {
    id
    name
    description
    price
    stock
    sku
    active
  }
}
```

### Search Products

```graphql
query SearchProducts {
  searchProducts(query: "laptop", limit: 5) {
    id
    name
    price
  }
}
```

### Get Active Products

```graphql
query ActiveProducts {
  activeProducts(limit: 10) {
    id
    name
    price
    stock
  }
}
```

### Count Products

```graphql
query CountProducts {
  productsCount
}
```

### Get Accounts

```graphql
query GetAccounts {
  accounts(offset: 0, limit: 10) {
    id
    email
    name
    role
    active
  }
}
```

### Get Account by Email

```graphql
query GetAccountByEmail {
  accountByEmail(email: "user@example.com") {
    id
    email
    name
    role
  }
}
```

### Get Accounts by Role

```graphql
query AdminAccounts {
  accountsByRole(role: "admin", limit: 10) {
    id
    email
    name
  }
}
```

## Mutations

### Create Product

```graphql
mutation CreateProduct {
  createProduct(input: {
    name: "Gaming Laptop"
    description: "High-performance laptop for gaming"
    price: "1299.99"
    stock: 50
    sku: "LAPTOP-001"
    active: true
  }) {
    id
    name
    price
    sku
    createdAt
  }
}
```

### Update Product

```graphql
mutation UpdateProduct {
  updateProduct(
    id: 1,
    input: {
      name: "Gaming Laptop Pro"
      price: "1399.99"
      stock: 45
    }
  ) {
    id
    name
    price
    stock
    updatedAt
  }
}
```

### Delete Product

```graphql
mutation DeleteProduct {
  deleteProduct(id: 1)
}
```

### Create Account

```graphql
mutation CreateAccount {
  createAccount(input: {
    email: "newuser@example.com"
    name: "New User"
    role: "user"
    active: true
  }) {
    id
    email
    name
    role
    createdAt
  }
}
```

### Update Account

```graphql
mutation UpdateAccount {
  updateAccount(
    id: 1,
    input: {
      name: "Updated Name"
      role: "admin"
    }
  ) {
    id
    email
    name
    role
    updatedAt
  }
}
```

### Delete Account

```graphql
mutation DeleteAccount {
  deleteAccount(id: 1)
}
```

## Advanced Usage

### Variables

Use GraphQL variables for dynamic queries:

```graphql
query GetProduct($id: Int!) {
  product(id: $id) {
    id
    name
    price
  }
}
```

Variables:
```json
{
  "id": 1
}
```

### Fragments

Reuse common fields with fragments:

```graphql
fragment ProductBasics on Product {
  id
  name
  price
  sku
}

query GetProducts {
  products(limit: 5) {
    ...ProductBasics
    stock
    active
  }
}
```

### Aliases

Query the same field with different arguments:

```graphql
query MultipleQueries {
  cheapProducts: products(limit: 5) {
    id
    name
    price
  }
  allProducts: products(limit: 100) {
    id
    name
  }
}
```

## Error Handling

GraphQL errors include structured information:

```json
{
  "errors": [
    {
      "message": "Product with id 999 not found",
      "extensions": {
        "code": "NOT_FOUND"
      }
    }
  ]
}
```

Error codes:
- `NOT_FOUND` - Resource not found
- `VALIDATION_ERROR` - Input validation failed
- `DATABASE_ERROR` - Database operation failed
- `INTERNAL_ERROR` - Server error

## Code Generation

### Generate New GraphQL Type

```bash
rustforge make:graphql-type Order --model --migration
```

This creates:
- GraphQL type definition
- Query and mutation resolvers
- Sea-ORM model (optional)
- Database migration (optional)

### Custom Type Example

```rust
// crates/foundry-graphql/src/types/order.rs
use async_graphql::{InputObject, SimpleObject};

#[derive(SimpleObject)]
pub struct Order {
    pub id: i64,
    pub user_id: i64,
    pub total: String,
    pub status: String,
}

#[derive(InputObject)]
pub struct OrderInput {
    pub user_id: i64,
    pub total: String,
}
```

### Custom Resolver Example

```rust
// crates/foundry-graphql/src/resolvers/order.rs
use async_graphql::{Context, Object, Result};

#[derive(Default)]
pub struct OrderQuery;

#[Object]
impl OrderQuery {
    async fn orders(&self, ctx: &Context<'_>) -> Result<Vec<Order>> {
        // Your logic here
        Ok(vec![])
    }
}
```

## Integration with Axum

### Add GraphQL Routes

```rust
use foundry_graphql::{build_schema, graphql_routes};
use foundry_infra::db::connect;

#[tokio::main]
async fn main() {
    // Connect to database
    let db = connect(&config).await.unwrap();

    // Build GraphQL schema
    let schema = build_schema(db);

    // Create routes
    let graphql_router = graphql_routes(schema);

    // Merge with existing routes
    let app = Router::new()
        .merge(graphql_router)
        .route("/health", get(health));

    // Start server
    axum::serve(listener, app).await.unwrap();
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use foundry_graphql::build_schema;

    #[tokio::test]
    async fn test_product_query() {
        let db = TestDatabase::new().await.unwrap();
        let schema = build_schema(db.connection().clone());

        let query = r#"
            query {
                products {
                    id
                    name
                }
            }
        "#;

        let response = schema.execute(query).await;
        assert!(response.errors.is_empty());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_create_product() {
    let db = TestDatabase::new().await.unwrap();
    db.create_test_schema().await.unwrap();

    let schema = build_schema(db.connection().clone());

    let mutation = r#"
        mutation {
            createProduct(input: {
                name: "Test Product"
                price: "19.99"
                stock: 100
                sku: "TEST-001"
            }) {
                id
                name
            }
        }
    "#;

    let response = schema.execute(mutation).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    assert_eq!(data["createProduct"]["name"], "Test Product");
}
```

## Performance Optimization

### DataLoader

Use DataLoader to batch and cache database queries:

```rust
use async_graphql::dataloader::*;

struct ProductLoader {
    db: DatabaseConnection,
}

#[async_trait::async_trait]
impl Loader<i64> for ProductLoader {
    type Value = Product;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[i64]) -> Result<HashMap<i64, Self::Value>> {
        // Batch load products by IDs
        let products = ProductEntity::find()
            .filter(Column::Id.is_in(keys))
            .all(&self.db)
            .await?;

        Ok(products.into_iter()
            .map(|p| (p.id, p.into()))
            .collect())
    }
}
```

### Query Complexity

Limit query complexity to prevent abuse:

```rust
use async_graphql::*;

let schema = Schema::build(query, mutation, subscription)
    .limit_depth(10)
    .limit_complexity(100)
    .finish();
```

## Production Deployment

### Enable Production Mode

```rust
let schema = Schema::build(query, mutation, subscription)
    .disable_introspection()  // Disable in production
    .finish();
```

### Rate Limiting

Use middleware to rate limit GraphQL requests:

```rust
use tower::limit::RateLimitLayer;

let graphql_router = graphql_routes(schema)
    .layer(RateLimitLayer::new(100, Duration::from_secs(60)));
```

### Monitoring

Add tracing to monitor GraphQL queries:

```rust
use async_graphql::extensions::Tracing;

let schema = Schema::build(query, mutation, subscription)
    .extension(Tracing::default())
    .finish();
```

## Best Practices

1. **Use Input Types** - Always use InputObject for mutations
2. **Validate Early** - Validate inputs before database operations
3. **Handle Errors** - Use structured error codes
4. **Batch Queries** - Use DataLoader for related data
5. **Limit Complexity** - Set depth and complexity limits
6. **Cache Results** - Use Redis for caching expensive queries
7. **Monitor Performance** - Track slow queries and optimize
8. **Document Schema** - Add descriptions to all types and fields

## Resources

- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [GraphQL Specification](https://spec.graphql.org/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)

---

For more information, see the main [README.md](README.md) or visit the documentation.
