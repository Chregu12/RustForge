# Database & ORM Code Snippets

Common database and Eloquent ORM patterns for RustForge.

---

## Basic Queries

### Select All Records

```rust
use crate::models::User;

let users = User::all(db).await?;
```

### Find by ID

```rust
let user = User::find(1, db).await?;

// With error handling
let user = User::find(id, db)
    .await
    .map_err(|_| AppError::NotFound("User not found"))?;
```

### Where Clauses

```rust
// Simple where
let active_users = User::where_eq("active", true, db).await?;

// Multiple conditions
let users = User::query()
    .filter(user::Column::Active.eq(true))
    .filter(user::Column::Role.eq("admin"))
    .all(db)
    .await?;

// Or conditions
use sea_orm::Condition;

let users = User::query()
    .filter(
        Condition::any()
            .add(user::Column::Role.eq("admin"))
            .add(user::Column::Role.eq("moderator"))
    )
    .all(db)
    .await?;
```

---

## Advanced Queries

### Ordering

```rust
// Single column
let users = User::query()
    .order_by_asc(user::Column::Name)
    .all(db)
    .await?;

// Multiple columns
let users = User::query()
    .order_by_asc(user::Column::Role)
    .order_by_desc(user::Column::CreatedAt)
    .all(db)
    .await?;
```

### Pagination

```rust
use rf_orm::Paginator;

// Manual pagination
let users = User::query()
    .limit(10)
    .offset(20) // Page 3 (20 = 2 * 10)
    .all(db)
    .await?;

// Using paginator
let page = req.query("page").unwrap_or(1);
let paginated = User::query()
    .paginate(db, 15) // 15 per page
    .fetch_page(page)
    .await?;

// Access pagination data
println!("Total: {}", paginated.total);
println!("Per page: {}", paginated.per_page);
println!("Current page: {}", paginated.current_page);
println!("Last page: {}", paginated.last_page);
let users = paginated.data;
```

### Aggregates

```rust
// Count
let count = User::query()
    .filter(user::Column::Active.eq(true))
    .count(db)
    .await?;

// Sum
let total = Order::query()
    .filter(order::Column::Status.eq("completed"))
    .sum(order::Column::Total, db)
    .await?;

// Average
let avg = Order::query()
    .avg(order::Column::Total, db)
    .await?;

// Min/Max
let min = Order::min(order::Column::Total, db).await?;
let max = Order::max(order::Column::Total, db).await?;
```

### Grouping

```rust
let results = Order::query()
    .select_only()
    .column(order::Column::Status)
    .column_as(order::Column::Id.count(), "count")
    .group_by(order::Column::Status)
    .into_json()
    .all(db)
    .await?;

// Result: [{"status": "pending", "count": 5}, {"status": "completed", "count": 12}]
```

---

## Relationships

### HasMany

```rust
// Define relationship in User model
impl User {
    pub fn posts(&self) -> HasMany<Post> {
        self.has_many()
    }
}

// Use relationship
let user = User::find(1, db).await?;
let posts = user.posts().get(db).await?;

// Eager loading (prevents N+1)
let users = User::with("posts", db).await?;
for user in users {
    println!("User: {}, Posts: {}", user.name, user.posts.len());
}
```

### BelongsTo

```rust
// Define relationship in Post model
impl Post {
    pub fn author(&self) -> BelongsTo<User> {
        self.belongs_to("user_id")
    }
}

// Use relationship
let post = Post::find(1, db).await?;
let author = post.author().get(db).await?;

// Eager loading
let posts = Post::with("author", db).await?;
```

### BelongsToMany (Many-to-Many)

```rust
// Define relationship in User model
impl User {
    pub fn roles(&self) -> BelongsToMany<Role> {
        self.belongs_to_many("role_user") // pivot table name
    }
}

// Use relationship
let user = User::find(1, db).await?;
let roles = user.roles().get(db).await?;

// Attach role
user.roles().attach(role_id, db).await?;

// Detach role
user.roles().detach(role_id, db).await?;

// Sync roles (replace all)
user.roles().sync(vec![1, 2, 3], db).await?;
```

### HasManyThrough

```rust
// Country -> User -> Post
impl Country {
    pub fn posts(&self) -> HasManyThrough<Post, User> {
        self.has_many_through()
    }
}

let country = Country::find(1, db).await?;
let posts = country.posts().get(db).await?;
```

### Nested Eager Loading

```rust
// Load posts with their comments and authors
let users = User::with("posts.comments", db)
    .with("posts.author", db)
    .await?;
```

---

## Creating Records

### Insert Single Record

```rust
let user = User::create(db, UserData {
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
    password: Hash::make("password")?,
}).await?;
```

### Insert Multiple Records

```rust
let users = User::insert_many(db, vec![
    UserData { name: "Alice".into(), email: "alice@example.com".into() },
    UserData { name: "Bob".into(), email: "bob@example.com".into() },
]).await?;
```

### Create or Update (Upsert)

```rust
let user = User::update_or_create(
    db,
    // Where clause (find by email)
    json!({"email": "alice@example.com"}),
    // Values to update/create
    json!({"name": "Alice Updated"}),
).await?;
```

### First or Create

```rust
let user = User::first_or_create(
    db,
    json!({"email": "alice@example.com"}),
    json!({"name": "Alice", "email": "alice@example.com"}),
).await?;
```

---

## Updating Records

### Update Single Record

```rust
let mut user = User::find(1, db).await?;
user.name = "Updated Name".to_string();
user.save(db).await?;
```

### Update Multiple Records

```rust
User::query()
    .filter(user::Column::Active.eq(false))
    .update(db, user::ActiveModel {
        active: Set(true),
        ..Default::default()
    })
    .await?;
```

### Increment/Decrement

```rust
// Increment
user.increment("login_count", 1, db).await?;

// Decrement
product.decrement("stock", 5, db).await?;
```

---

## Deleting Records

### Delete Single Record

```rust
let user = User::find(1, db).await?;
user.delete(db).await?;
```

### Delete Multiple Records

```rust
User::query()
    .filter(user::Column::Active.eq(false))
    .delete(db)
    .await?;
```

### Soft Deletes

```rust
// Enable soft deletes in model
#[derive(Model)]
#[soft_deletes]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub deleted_at: Option<DateTime<Utc>>,
}

// Soft delete (sets deleted_at timestamp)
post.delete(db).await?;

// Query only non-deleted
let posts = Post::all(db).await?; // Excludes soft-deleted

// Include soft-deleted
let all_posts = Post::with_trashed(db).await?;

// Only soft-deleted
let deleted = Post::only_trashed(db).await?;

// Restore soft-deleted
post.restore(db).await?;

// Force delete (permanent)
post.force_delete(db).await?;
```

---

## Transactions

### Basic Transaction

```rust
use sea_orm::TransactionTrait;

db.transaction::<_, (), DbErr>(|txn| {
    Box::pin(async move {
        // Create user
        let user = User::create(txn, user_data).await?;

        // Create profile
        Profile::create(txn, ProfileData {
            user_id: user.id,
            bio: "Hello!".to_string(),
        }).await?;

        // Update counter
        Counter::increment("total_users", 1, txn).await?;

        Ok(())
    })
}).await?;
```

### Manual Transaction Control

```rust
let txn = db.begin().await?;

match try_operation(&txn).await {
    Ok(result) => {
        txn.commit().await?;
        Ok(result)
    }
    Err(e) => {
        txn.rollback().await?;
        Err(e)
    }
}
```

---

## Raw Queries

### Execute Raw SQL

```rust
use sea_orm::{Statement, DatabaseBackend};

let result = db.execute(Statement::from_string(
    DatabaseBackend::Postgres,
    "UPDATE users SET active = true WHERE created_at > NOW() - INTERVAL '7 days'",
)).await?;
```

### Query with Parameters

```rust
let users = User::find()
    .from_raw_sql(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT * FROM users WHERE email = $1",
        vec!["alice@example.com".into()],
    ))
    .all(db)
    .await?;
```

---

## Query Scopes

### Define Scopes

```rust
impl User {
    // Scope for active users
    pub fn active() -> impl Fn(Select<User>) -> Select<User> {
        |query| query.filter(user::Column::Active.eq(true))
    }

    // Scope for recent users
    pub fn recent() -> impl Fn(Select<User>) -> Select<User> {
        |query| {
            let week_ago = Utc::now() - Duration::days(7);
            query.filter(user::Column::CreatedAt.gte(week_ago))
        }
    }
}

// Use scopes
let active_users = User::query()
    .scope(User::active())
    .all(db)
    .await?;

// Chain scopes
let recent_active = User::query()
    .scope(User::active())
    .scope(User::recent())
    .all(db)
    .await?;
```

---

## Chunking Large Datasets

### Process in Chunks

```rust
User::query()
    .chunk(100, |users| async move {
        for user in users {
            // Process each user
            send_email(&user).await?;
        }
        Ok(())
    })
    .await?;
```

### Cursor-Based Pagination

```rust
let mut cursor = User::query().cursor_by(user::Column::Id);

while let Some(users) = cursor.next(100, db).await? {
    for user in users {
        // Process user
    }
}
```

---

## Subqueries

### Where Exists

```rust
let users_with_posts = User::query()
    .filter(
        user::Column::Id.in_subquery(
            Query::select()
                .column(post::Column::UserId)
                .from(Post::table())
                .to_owned()
        )
    )
    .all(db)
    .await?;
```

### Select Subquery

```rust
let users = User::query()
    .column_as(
        Query::select()
            .expr(Func::count(Expr::col(post::Column::Id)))
            .from(Post::table())
            .and_where(Expr::col(post::Column::UserId).equals(Expr::col(user::Column::Id)))
            .to_owned(),
        "posts_count"
    )
    .all(db)
    .await?;
```

---

## Database Seeding

### Seeder

```rust
use rf_testing::Seeder;

pub struct UserSeeder;

#[async_trait]
impl Seeder for UserSeeder {
    async fn run(&self, db: &DatabaseConnection) -> Result<()> {
        User::insert_many(db, vec![
            UserData {
                name: "Admin".into(),
                email: "admin@example.com".into(),
                password: Hash::make("password")?,
            },
            UserData {
                name: "User".into(),
                email: "user@example.com".into(),
                password: Hash::make("password")?,
            },
        ]).await?;

        Ok(())
    }
}

// Run seeder
UserSeeder.run(db).await?;
```

---

## Factories

### Define Factory

```rust
use rf_testing::Factory;
use fake::{Fake, faker::internet::en::*};

pub struct UserFactory;

impl Factory for UserFactory {
    type Model = User;

    fn definition() -> UserData {
        UserData {
            name: Name().fake(),
            email: SafeEmail().fake(),
            password: Hash::make("password").unwrap(),
            active: true,
        }
    }
}

// Use factory
let user = UserFactory::create(db).await?;
let users = UserFactory::create_many(10, db).await?;

// Override attributes
let admin = UserFactory::new()
    .with("role", "admin")
    .create(db)
    .await?;
```

---

## Database Migrations

### Create Migration

```bash
forge make:migration create_users_table
```

### Migration File

```rust
use sea_orm_migration::prelude::*;

pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::Name).string().not_null())
                    .col(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Password).string().not_null())
                    .col(ColumnDef::new(User::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(User::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum User {
    Table,
    Id,
    Name,
    Email,
    Password,
    CreatedAt,
    UpdatedAt,
}
```

### Run Migrations

```bash
# Run all pending migrations
forge migrate

# Rollback last migration
forge migrate:rollback

# Rollback all migrations
forge migrate:reset

# Refresh database (reset + migrate)
forge migrate:refresh

# Fresh database (drop + migrate)
forge migrate:fresh
```

---

## Performance Tips

### 1. Use Eager Loading

```rust
// ❌ N+1 queries
let users = User::all(db).await?;
for user in users {
    let posts = user.posts().get(db).await?; // N queries!
}

// ✅ 2 queries
let users = User::with("posts", db).await?;
for user in users {
    let posts = &user.posts; // Already loaded!
}
```

### 2. Select Only Needed Columns

```rust
// ❌ Select all columns
let users = User::all(db).await?;

// ✅ Select only needed columns
let users = User::query()
    .select_only()
    .column(user::Column::Id)
    .column(user::Column::Name)
    .into_model::<UserSummary>()
    .all(db)
    .await?;
```

### 3. Use Indexes

```rust
// In migration
.create_index(
    Index::create()
        .name("idx_users_email")
        .table(User::Table)
        .col(User::Email)
        .to_owned()
)
.await?;
```

### 4. Batch Operations

```rust
// ❌ Multiple queries
for user in users {
    user.update(db, ...).await?;
}

// ✅ Single query
User::query()
    .filter(user::Column::Id.is_in(user_ids))
    .update(db, ...)
    .await?;
```

---

These snippets cover the most common database patterns in RustForge!
