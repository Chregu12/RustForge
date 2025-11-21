# Advanced ORM Features Guide

This guide covers the advanced ORM features available in RustForge, providing Laravel-style relationship types, query capabilities, and loading strategies.

## Table of Contents

1. [HasOneThrough & HasManyThrough](#hasonethrough--hasmanythrough)
2. [MorphToMany (Polymorphic Many-to-Many)](#morphtomany)
3. [Subquery Support](#subquery-support)
4. [Advanced Aggregations](#advanced-aggregations)
5. [Lazy vs Eager Loading](#lazy-vs-eager-loading)

---

## HasOneThrough & HasManyThrough

Through relationships provide a convenient way to access distant relationships via an intermediate model.

### Concept

```text
Country (id)
  └─> User (id, country_id)
      └─> Post (id, user_id)

HasManyThrough: Country.posts() returns all posts where:
  posts.user_id = users.id AND users.country_id = country.id
```

### Basic Usage

```rust
use rf_orm::relationships::through::*;

// Get all posts in a country (through users)
let country = Country::find_by_id(1).one(&db).await?.unwrap();

let posts = has_many_through::<post::Entity, user::Entity>(
    &db,
    country.id,
    "country_id",  // FK in User table
    "user_id",     // FK in Post table
).await?;

println!("Country has {} posts", posts.len());
```

### Advanced Queries

```rust
// Get latest posts with filtering
let latest_posts = has_one_through::<post::Entity, user::Entity>(
    &db,
    country.id,
    "country_id",
    "user_id",
)
.where_raw("posts.published = true")
.order_by_desc("posts.created_at")
.limit(10)
.get()
.await?;
```

### Using Macros

```rust
use rf_orm::has_many_through;

// Define the relationship once
has_many_through!(
    Country,       // Parent model
    Post,          // Target model
    User,          // Intermediate model
    posts,         // Method name
    "country_id",  // FK in intermediate
    "user_id"      // FK in target
);

// Use it anywhere
impl Country {
    pub async fn posts(&self, db: &DatabaseConnection) -> ThroughResult<Vec<post::Model>> {
        has_many_through::<post::Entity, user::Entity>(
            db,
            self.id,
            "country_id",
            "user_id",
        ).await
    }
}
```

---

## MorphToMany

Polymorphic many-to-many relationships allow a model to belong to multiple types on a single association.

### Use Cases

- **Tag System**: Posts, Videos, and Articles can all be tagged
- **Like System**: Users can like different content types
- **Attachment System**: Files can be attached to various models

### Database Schema

```sql
-- Tags table
CREATE TABLE tags (
    id BIGINT PRIMARY KEY,
    name VARCHAR(255)
);

-- Taggables pivot table (polymorphic)
CREATE TABLE taggables (
    tag_id BIGINT,
    taggable_type VARCHAR(255),  -- "Post", "Video", etc.
    taggable_id BIGINT,
    created_at TIMESTAMP,
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);
```

### Basic Usage

```rust
use rf_orm::relationships::morph_to_many::*;

// Mark entities as morphable
morphable!(post::Entity, "Post");
morphable!(video::Entity, "Video");

// Attach tags to a post
let post = Post::find_by_id(1).one(&db).await?.unwrap();
let tag = Tag::find_by_id(1).one(&db).await?.unwrap();

attach_morph(
    &db,
    "Post",       // Morph type
    post.id,      // Parent ID
    "taggables",  // Pivot table
    "taggable",   // Morph name prefix
    "tag_id",     // Related key
    tag.id,       // Related ID
).await?;

// Load all tags for a post
let tags = morph_to_many::<tag::Entity>(
    &db,
    "Post",
    post.id,
    "taggables",
    "taggable",
).await?;

for tag in tags {
    println!("Tag: {}", tag.name);
}
```

### Sync Operation

Replace all current tags with a new set:

```rust
// Sync tags (detach all, then attach new ones)
let new_tag_ids = vec![1, 2, 3];

sync_morph(
    &db,
    "Post",
    post.id,
    "taggables",
    "taggable",
    "tag_id",
    &new_tag_ids,
).await?;
```

### Toggle Operation

Perfect for like/favorite functionality:

```rust
// Toggle a tag (attach if not exists, detach if exists)
let attached = toggle_morph(
    &db,
    "Post",
    post.id,
    "taggables",
    "taggable",
    "tag_id",
    tag.id,
).await?;

if attached {
    println!("Tag attached");
} else {
    println!("Tag detached");
}
```

### Advanced Queries

```rust
use rf_orm::relationships::morph_to_many::MorphToManyBuilder;

let tags = MorphToManyBuilder::<tag::Entity>::new(
    db.clone(),
    "Post",
    post.id,
    "taggables",
    "taggable",
)
.where_raw("tags.status = 'active'")
.order_by("tags.name", "asc")
.limit(10)
.get()
.await?;
```

---

## Subquery Support

Subqueries enable complex filtering by embedding queries within queries.

### WHERE IN with Subquery

Find users who have published posts:

```rust
use rf_orm::query::subquery::Subquery;

let subquery = Subquery::new::<post::Entity>(db.clone())
    .select("user_id")
    .where_eq("published", true)
    .where_gt("views", 100);

let users = User::query(db.clone())
    .where_in_subquery("id", subquery)
    .get()
    .await?;
```

### WHERE EXISTS

Find posts that have comments:

```rust
let subquery = Subquery::new::<comment::Entity>(db.clone())
    .where_raw("comments.post_id = posts.id");

let posts = Post::query(db.clone())
    .where_exists(subquery)
    .get()
    .await?;
```

### Complex Subqueries

```rust
// Find users with more than 10 published posts
let subquery = Subquery::new::<post::Entity>(db.clone())
    .select("user_id")
    .where_eq("published", true)
    .where_raw("COUNT(*) > 10");

let prolific_users = User::query(db)
    .where_in_subquery("id", subquery)
    .get()
    .await?;
```

### SubqueryBuilder (without Entity type)

```rust
use rf_orm::query::subquery::SubqueryBuilder;

let subquery = SubqueryBuilder::new("posts")
    .select("user_id")
    .where_clause("published = true")
    .where_clause("views > 1000")
    .build();

// Use in raw SQL
let sql = format!("SELECT * FROM users WHERE id IN {}", subquery);
```

---

## Advanced Aggregations

Load relationship aggregates alongside your models without N+1 queries.

### withCount

Count related records:

```rust
use rf_orm::query::aggregations::*;

let user = User::find_by_id(1).one(&db).await?.unwrap();

// Get post count for a user
let post_count = load_count(&db, "posts", "user_id", user.id).await?;
println!("User has {} posts", post_count);
```

### withSum

Sum a column across related records:

```rust
// Get total views for a user's posts
let total_views = load_sum(&db, "posts", "views", "user_id", user.id).await?;
println!("Total views: {}", total_views);
```

### withAvg

Average a column:

```rust
// Get average rating for a user's posts
let avg_rating = load_avg(&db, "posts", "rating", "user_id", user.id)
    .await?
    .unwrap_or(0.0);
println!("Average rating: {:.2}", avg_rating);
```

### withMin / withMax

```rust
let min_price = load_min(&db, "products", "price", "category_id", category.id).await?;
let max_price = load_max(&db, "products", "price", "category_id", category.id).await?;

if let (Some(min), Some(max)) = (min_price, max_price) {
    println!("Price range: ${:.2} - ${:.2}", min, max);
}
```

### Multiple Aggregates

```rust
use rf_orm::query::aggregations::AggregationBuilder;

let builder = AggregationBuilder::new(db.clone())
    .add_count("posts")
    .add_sum("posts", "views")
    .add_avg("posts", "rating");

let results = builder.execute::<user::Entity>("users", "id").await?;

// Results is a HashMap<user_id, HashMap<aggregate_name, value>>
for (user_id, aggregates) in results {
    println!("User {}: {} posts, {} total views, {:.2} avg rating",
        user_id,
        aggregates.get("posts_count").unwrap_or(&0.0),
        aggregates.get("posts_views_sum").unwrap_or(&0.0),
        aggregates.get("posts_rating_avg").unwrap_or(&0.0),
    );
}
```

### Filtered Aggregations

```rust
let count = Aggregate::count("posts")
    .with_where("published = true");

// Use in queries...
```

---

## Lazy vs Eager Loading

Control when and how relationships are loaded to optimize performance.

### Lazy Loading

Load relationships on-demand (may cause N+1 queries):

```rust
use rf_orm::relationships::loading::LazyLoad;

// Fetch user without relationships
let user = User::find_by_id(1).one(&db).await?.unwrap();

// Load posts when needed
let posts = user.lazy_load::<post::Entity>(&db).await?;
println!("User has {} posts", posts.len());

// Load a belongs-to relationship
let profile = user.lazy_load_one::<profile::Entity>(&db).await?;
if let Some(profile) = profile {
    println!("Profile: {}", profile.bio);
}
```

### Eager Loading

Load relationships with the main query (prevents N+1):

```rust
use rf_orm::relationships::basic::eager_load;

// Fetch users and their posts in one go
let users = User::query(db.clone()).get().await?;
let with_posts = eager_load::<user::Entity, post::Entity>(users, &db).await?;

for (user, posts) in with_posts {
    println!("{} has {} posts", user.name, posts.len());
}
```

### Lazy Eager Loading

Load relationships for a collection after fetching:

```rust
use rf_orm::relationships::loading::CollectionExt;

// Fetch users first
let mut users = User::query(db.clone()).get().await?;

// Load posts for all users in a single query
users.load::<post::Entity>(&db, "posts").await?;

// Load multiple relationships
users.load_multiple(&db, &["posts", "comments", "profile"]).await?;
```

### Eager Loading Configuration

```rust
use rf_orm::relationships::loading::*;

// Configure which relationships to always/never eager load
let config = EagerLoadConfig::new()
    .always("profile")    // Always load profile
    .never("sessions")    // Never load sessions
    .max_depth(3);        // Max nesting depth

if config.should_load("profile") {
    // Load profile...
}
```

### Using Macros

```rust
use rf_orm::supports_eager_loading;

// Configure eager loading for an entity
supports_eager_loading!(
    user::Entity,
    relations: ["posts", "comments", "profile"],
    always: ["profile"],
    never: ["sessions"]
);
```

---

## Performance Best Practices

### 1. Use Eager Loading for Collections

```rust
// ❌ Bad: N+1 queries
let users = User::query(db.clone()).get().await?;
for user in users {
    let posts = user.lazy_load::<post::Entity>(&db).await?; // N queries!
}

// ✅ Good: Single query
let mut users = User::query(db.clone()).get().await?;
users.load::<post::Entity>(&db, "posts").await?; // 1 query!
```

### 2. Use Aggregations Instead of Loading Everything

```rust
// ❌ Bad: Load all posts to count them
let posts = user.lazy_load::<post::Entity>(&db).await?;
let count = posts.len();

// ✅ Good: Count in database
let count = load_count(&db, "posts", "user_id", user.id).await?;
```

### 3. Use Subqueries for Complex Filtering

```rust
// ✅ Good: Single query with subquery
let users = User::query(db.clone())
    .where_in_subquery(
        "id",
        Subquery::new::<post::Entity>(db.clone())
            .select("user_id")
            .where_eq("published", true)
    )
    .get()
    .await?;
```

### 4. Use Through Relationships for Distant Relations

```rust
// ✅ Good: Single query through intermediate
let posts = has_many_through::<post::Entity, user::Entity>(
    &db,
    country.id,
    "country_id",
    "user_id",
).await?;
```

---

## Complete Example

Here's a complete example combining multiple advanced features:

```rust
use rf_orm::prelude::*;
use rf_orm::relationships::{through::*, morph_to_many::*, loading::*};
use rf_orm::query::{subquery::*, aggregations::*};

async fn advanced_example(db: DatabaseConnection) -> Result<(), DbErr> {
    // 1. Fetch countries with post counts
    let countries = Country::query(db.clone()).get().await?;

    for country in countries {
        // 2. Get post count through users
        let posts = has_many_through::<post::Entity, user::Entity>(
            &db,
            country.id,
            "country_id",
            "user_id",
        )
        .where_raw("posts.published = true")
        .get()
        .await?;

        println!("Country {} has {} published posts", country.name, posts.len());

        // 3. Get tags for all posts (polymorphic)
        for post in &posts {
            let tags = morph_to_many::<tag::Entity>(
                &db,
                "Post",
                post.id,
                "taggables",
                "taggable",
            ).await?;

            println!("  Post '{}' has tags: {:?}",
                post.title,
                tags.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
        }

        // 4. Get aggregations
        let total_views = load_sum(&db, "posts", "views", "country_id", country.id).await?;
        let avg_rating = load_avg(&db, "posts", "rating", "country_id", country.id).await?;

        println!("  Total views: {}", total_views);
        if let Some(avg) = avg_rating {
            println!("  Average rating: {:.2}", avg);
        }
    }

    // 5. Find users with subquery
    let active_users = User::query(db.clone())
        .where_in_subquery(
            "id",
            Subquery::new::<post::Entity>(db.clone())
                .select("user_id")
                .where_eq("published", true)
                .where_gt("views", 1000)
        )
        .get()
        .await?;

    println!("Found {} active users with popular posts", active_users.len());

    Ok(())
}
```

---

## Laravel Comparison

| Laravel | RustForge |
|---------|-----------|
| `$country->posts()` (HasManyThrough) | `has_many_through::<post::Entity, user::Entity>()` |
| `$post->tags()` (MorphToMany) | `morph_to_many::<tag::Entity>()` |
| `$post->tags()->attach($tagId)` | `attach_morph()` |
| `$post->tags()->sync([1,2,3])` | `sync_morph()` |
| `whereIn('id', $subquery)` | `.where_in_subquery("id", subquery)` |
| `withCount('posts')` | `load_count()` |
| `withSum('posts', 'views')` | `load_sum()` |
| `with(['posts', 'comments'])` | `eager_load()` |
| `$users->load('posts')` | `.load::<post::Entity>()` |

---

## Next Steps

- See `crates/rf-orm/tests/advanced_relationships_test.rs` for integration tests
- Check `crates/rf-orm/src/relationships/` for implementation details
- Read the main ORM guide for basic relationship types
- Explore the query builder documentation for more query options
