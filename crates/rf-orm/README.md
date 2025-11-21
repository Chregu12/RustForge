# rf-orm: Laravel-Inspired Type-Safe Database ORM

A powerful, Laravel-Eloquent-inspired ORM built on top of SeaORM, providing a familiar and ergonomic API for Rust developers.

## Features

### Core Features
- ✅ Laravel-style Query Builder API
- ✅ Eloquent-style relationships (BelongsTo, HasMany, BelongsToMany)
- ✅ Model events (creating, created, updating, updated, etc.)
- ✅ Transaction support with automatic rollback
- ✅ Eager loading to prevent N+1 queries
- ✅ Connection pooling and management
- ✅ Soft delete trait
- ✅ Migration support
- ✅ Testing utilities
- ✅ Scopes for reusable query logic
- ✅ Collection helpers

### Advanced Features (Phase 2)
- ✅ **HasOneThrough & HasManyThrough** - Access distant relationships
- ✅ **MorphToMany** - Polymorphic many-to-many relationships
- ✅ **Subquery Support** - Complex filtering with nested queries
- ✅ **Advanced Aggregations** - withCount, withSum, withAvg, etc.
- ✅ **Loading Control** - Lazy, eager, and lazy-eager loading strategies

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-orm = "0.1"
sea-orm = "1.2"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use rf_orm::prelude::*;
use sea_orm::entity::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    let db = DatabaseManager::connect(DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        ..Default::default()
    }).await?;

    // Laravel-style query
    let posts = Post::query(db.connection().clone())
        .where_eq(post::Column::Published, true)
        .where_gt(post::Column::Views, 100)
        .order_by_desc(post::Column::CreatedAt)
        .limit(10)
        .get()
        .await?;

    // Relationships
    for post in &posts {
        let author = post.load_belongs_to::<User>(db.connection()).await?;
        let comments = post.load_has_many::<Comment>(db.connection()).await?;
    }

    Ok(())
}
```

## Documentation

### Guides
- [Basic ORM Guide](../../docs/ORM_GUIDE.md) - Getting started with relationships, queries, and models
- [Advanced ORM Guide](../../docs/ORM_ADVANCED_GUIDE.md) - Through relationships, polymorphic relations, subqueries, and aggregations

### Feature Documentation

#### Query Builder

```rust
// Simple queries
let users = User::query(db)
    .where_eq("active", true)
    .where_like("name", "%John%")
    .order_by("created_at", "desc")
    .limit(10)
    .get()
    .await?;

// Subqueries
let users_with_posts = User::query(db.clone())
    .where_in_subquery(
        "id",
        Subquery::new::<post::Entity>(db.clone())
            .select("user_id")
            .where_eq("published", true)
    )
    .get()
    .await?;

// Chunking for large datasets
Post::query(db)
    .chunk(100, |posts| async {
        for post in posts {
            process_post(post).await?;
        }
        Ok(())
    })
    .await?;
```

#### Basic Relationships

```rust
// BelongsTo
let author = post.load_belongs_to::<User>(&db).await?;

// HasMany
let posts = user.load_has_many::<Post>(&db).await?;

// BelongsToMany
let tags = post.load_many_to_many::<Tag>(&db).await?;
```

#### Advanced Relationships

```rust
// HasManyThrough: Country -> User -> Post
use rf_orm::relationships::through::*;

let posts = has_many_through::<post::Entity, user::Entity>(
    &db,
    country.id,
    "country_id",
    "user_id",
).await?;

// MorphToMany: Polymorphic tags
use rf_orm::relationships::morph_to_many::*;

// Attach a tag to a post
attach_morph(&db, "Post", post.id, "taggables", "taggable", "tag_id", tag.id).await?;

// Load all tags
let tags = morph_to_many::<tag::Entity>(&db, "Post", post.id, "taggables", "taggable").await?;

// Sync tags (replace all)
sync_morph(&db, "Post", post.id, "taggables", "taggable", "tag_id", &[1, 2, 3]).await?;
```

#### Aggregations

```rust
use rf_orm::query::aggregations::*;

// Count related records
let post_count = load_count(&db, "posts", "user_id", user.id).await?;

// Sum values
let total_views = load_sum(&db, "posts", "views", "user_id", user.id).await?;

// Average
let avg_rating = load_avg(&db, "posts", "rating", "user_id", user.id).await?;

// Min/Max
let min_price = load_min(&db, "products", "price", "category_id", category.id).await?;
let max_price = load_max(&db, "products", "price", "category_id", category.id).await?;
```

#### Loading Strategies

```rust
use rf_orm::relationships::loading::*;

// Lazy loading (on-demand)
let posts = user.lazy_load::<post::Entity>(&db).await?;

// Eager loading (prevent N+1)
let users = User::query(db.clone()).get().await?;
let with_posts = eager_load::<user::Entity, post::Entity>(users, &db).await?;

// Lazy eager loading (load for collection after fetching)
let mut users = User::query(db.clone()).get().await?;
users.load::<post::Entity>(&db, "posts").await?;
```

#### Model Events

```rust
use rf_orm::prelude::*;
use async_trait::async_trait;

#[async_trait]
impl ModelEvents for post::ActiveModel {
    async fn before_create(&mut self) -> EventResult {
        // Auto-generate slug
        self.slug = Set(slugify(&self.title));
        Ok(())
    }

    async fn after_create(&self) -> EventResult {
        // Send notification
        notify_new_post(self).await?;
        Ok(())
    }

    async fn before_update(&mut self) -> EventResult {
        // Update timestamp
        self.updated_at = Set(chrono::Utc::now().naive_utc());
        Ok(())
    }
}
```

#### Transactions

```rust
use rf_orm::prelude::*;

db.connection().transaction(|tx| async move {
    // Create user
    let user = User::create(tx, user_data).await?;

    // Create profile
    let profile = Profile::create(tx, profile_data).await?;

    // Both operations succeed or both rollback
    Ok(())
}).await?;

// With savepoints
let mut tx = db.connection().begin().await?;

let sp = tx.savepoint("before_update").await?;

match risky_operation(&tx).await {
    Ok(_) => sp.commit().await?,
    Err(_) => sp.rollback().await?,
}

tx.commit().await?;
```

#### Scopes

```rust
use rf_orm::scopes::*;

// Define reusable scopes
fn published() -> ScopeFn {
    Box::new(|query| {
        query.where_eq("published", true)
    })
}

fn popular() -> ScopeFn {
    Box::new(|query| {
        query.where_gt("views", 1000)
    })
}

// Use scopes
let posts = Post::query(db)
    .apply_scope(published())
    .apply_scope(popular())
    .get()
    .await?;
```

#### Collections

```rust
use rf_orm::collection::*;

let users = User::query(db).get().await?.into_collection();

// Filter
let active = users.filter(|u| u.active);

// Map
let names: Vec<String> = users.map(|u| u.name.clone()).collect();

// Chunk
for chunk in users.chunk(10) {
    process_batch(chunk).await?;
}

// Pluck
let ids: Vec<i64> = users.pluck(|u| u.id);

// Group by
let by_country = users.group_by(|u| u.country_id);
```

#### Soft Deletes

```rust
use rf_orm::soft_delete::*;

// Soft delete a model
post.soft_delete(&db).await?;

// Query excluding soft-deleted
let active_posts = Post::query(db).get().await?;

// Include soft-deleted
let all_posts = Post::query(db).with_trashed().get().await?;

// Only soft-deleted
let deleted_posts = Post::query(db).only_trashed().get().await?;

// Restore
post.restore(&db).await?;

// Force delete (permanent)
post.force_delete(&db).await?;
```

## Laravel Parity

### Basic Features
| Feature | Laravel | RustForge | Status |
|---------|---------|-----------|--------|
| Query Builder | `User::where('active', true)` | `User::query(db).where_eq("active", true)` | ✅ |
| Relationships | `$user->posts()` | `user.load_has_many::<Post>()` | ✅ |
| Eager Loading | `User::with('posts')` | `eager_load::<user::Entity, post::Entity>()` | ✅ |
| Soft Deletes | `$model->delete()` | `model.soft_delete()` | ✅ |
| Scopes | `User::active()` | `User::query().apply_scope(active())` | ✅ |
| Events | `creating`, `created`, etc. | `before_create`, `after_create`, etc. | ✅ |
| Transactions | `DB::transaction()` | `db.transaction()` | ✅ |

### Advanced Features
| Feature | Laravel | RustForge | Status |
|---------|---------|-----------|--------|
| HasManyThrough | `$country->posts()` | `has_many_through()` | ✅ |
| MorphToMany | `$post->tags()` | `morph_to_many()` | ✅ |
| Subqueries | `whereIn('id', $subquery)` | `.where_in_subquery()` | ✅ |
| withCount | `User::withCount('posts')` | `load_count()` | ✅ |
| withSum | `User::withSum('posts', 'views')` | `load_sum()` | ✅ |
| Lazy Eager Loading | `$users->load('posts')` | `.load::<post::Entity>()` | ✅ |

## Architecture

```
rf-orm/
├── src/
│   ├── collection.rs          # Collection helpers
│   ├── config.rs              # Database configuration
│   ├── error.rs               # Error types
│   ├── events.rs              # Model events
│   ├── manager.rs             # Connection management
│   ├── migrations.rs          # Migration support
│   ├── model.rs               # Base model trait
│   ├── polymorphic.rs         # Polymorphic relations (MorphOne, MorphMany)
│   ├── query_builder.rs       # Laravel-style query builder
│   ├── query/
│   │   ├── aggregations.rs    # Advanced aggregations
│   │   └── subquery.rs        # Subquery support
│   ├── relationships/
│   │   ├── basic.rs           # BelongsTo, HasMany, etc.
│   │   ├── through.rs         # HasOneThrough, HasManyThrough
│   │   ├── morph_to_many.rs   # Polymorphic many-to-many
│   │   └── loading.rs         # Eager/lazy loading control
│   ├── scopes.rs              # Query scopes
│   ├── schema_builder.rs      # Schema building
│   ├── soft_delete.rs         # Soft delete trait
│   └── transaction.rs         # Transaction support
└── tests/
    └── advanced_relationships_test.rs  # Integration tests
```

## Testing

```rust
use rf_orm::testing::*;

#[tokio::test]
async fn test_user_creation() {
    let db = setup_test_db().await;

    let user = create_user(&db, "John Doe").await.unwrap();

    assert_eq!(user.name, "John Doe");
}
```

## Performance Considerations

1. **Use Eager Loading**: Prevent N+1 queries by loading relationships upfront
2. **Use Aggregations**: Count/sum in the database instead of loading all records
3. **Use Subqueries**: Filter complex conditions in a single query
4. **Use Chunking**: Process large datasets without loading everything into memory
5. **Use Transactions**: Group related operations for consistency and performance

## Contributing

Contributions are welcome! Please see the main RustForge contributing guide.

## License

This project is part of the RustForge framework.

## Related Crates

- `rf-database` - Database connection and migration management
- `rf-mail` - Email functionality
- `rf-validation` - Form validation
- `rf-testing` - Testing utilities

## Resources

- [SeaORM Documentation](https://www.sea-ql.org/SeaORM/)
- [Laravel Eloquent Documentation](https://laravel.com/docs/eloquent)
- [RustForge Main Repository](https://github.com/yourusername/rust-dx-framework)
