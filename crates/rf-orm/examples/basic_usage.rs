//! # rf-orm Basic Usage Example
//!
//! This example demonstrates the Laravel-style ORM API provided by rf-orm.
//!
//! Note: This is a conceptual example showing the API patterns.
//! In a real application, you would:
//! 1. Set up a real database connection using DatabaseManager
//! 2. Define your entities with #[derive(DeriveEntityModel)]
//! 3. Run migrations to create tables

fn main() {
    println!("=== rf-orm API Examples ===\n");

    // 1. Database Connection
    print_example(
        "1. Database Connection",
        r#"
use rf_orm::prelude::*;

let db = DatabaseManager::connect(DatabaseConfig {
    url: "postgres://user:pass@localhost/db".to_string(),
    ..Default::default()
}).await?;
"#,
    );

    // 2. Define Entities
    print_example(
        "2. Define Entity (SeaORM)",
        r#"
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    pub published: bool,
    pub views: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#,
    );

    // 3. Query Builder - Laravel-style
    print_example(
        "3. Query Builder (Laravel-style)",
        r#"
use rf_orm::Model; // Import Model trait

// Fluent query building
let posts = Post::query(db)
    .where_eq(post::Column::Published, true)
    .where_gt(post::Column::Views, 100)
    .order_by_desc(post::Column::CreatedAt)
    .limit(10)
    .get()
    .await?;
"#,
    );

    // 4. First Result
    print_example(
        "4. Get First Result",
        r#"
let post = Post::query(db)
    .where_eq(post::Column::Id, 1)
    .first()
    .await?;
"#,
    );

    // 5. Multiple Where Conditions
    print_example(
        "5. Multiple Where Conditions",
        r#"
let posts = Post::query(db)
    .where_eq(post::Column::Published, true)
    .where_like(post::Column::Title, "%Laravel%")
    .where_in(post::Column::Category, vec!["Tech", "Programming"])
    .await?;
"#,
    );

    // 6. Ordering
    print_example(
        "6. Ordering Results",
        r#"
let posts = Post::query(db)
    .order_by_asc(post::Column::Title)
    .order_by_desc(post::Column::CreatedAt)
    .get()
    .await?;
"#,
    );

    // 7. Model Trait Methods
    print_example(
        "7. Model Trait Helper Methods",
        r#"
// Get all models
let all_posts = Post::all(&db).await?;

// Use the query builder for more complex queries
let query = Post::query(db);
"#,
    );

    // 8. Transactions
    print_example(
        "8. Transaction Support",
        r#"
use rf_orm::TransactionExt;

// Automatic commit/rollback
db.transaction(|tx| async move {
    // Create user
    let user = user::ActiveModel {
        name: Set("John".to_string()),
        ..Default::default()
    }.insert(tx).await?;

    // Create profile
    let profile = profile::ActiveModel {
        user_id: Set(user.id),
        bio: Set("Developer".to_string()),
        ..Default::default()
    }.insert(tx).await?;

    // If any error occurs, transaction will rollback automatically
    Ok(())
}).await?;
"#,
    );

    // 9. Relationships
    print_example(
        "9. Relationship Helpers",
        r#"
use rf_orm::RelationshipHelpers;

// BelongsTo - Load related model
let author = post.load_belongs_to::<user::Entity>(&db).await?;

// HasMany - Load related collection
let posts = user.load_has_many::<post::Entity>(&db).await?;

// Eager Loading - Prevent N+1 queries
let posts_with_authors = eager_load::<post::Entity, user::Entity>(posts, &db).await?;
for (post, authors) in posts_with_authors {
    println!("{}: {:?}", post.title, authors);
}
"#,
    );

    // 10. Model Events
    print_example(
        "10. Model Lifecycle Events",
        r#"
use rf_orm::ModelEvents;
use async_trait::async_trait;

#[async_trait]
impl ModelEvents for post::ActiveModel {
    async fn before_create(&mut self) -> EventResult {
        // Auto-generate slug from title
        if self.slug.is_not_set() {
            self.slug = Set(slugify(&self.title));
        }

        // Set timestamps
        let now = chrono::Utc::now();
        self.created_at = Set(now);
        self.updated_at = Set(now);

        Ok(())
    }

    async fn before_update(&mut self) -> EventResult {
        // Update timestamp
        self.updated_at = Set(chrono::Utc::now());
        Ok(())
    }
}

// Or use the timestamps! macro
timestamps!(post::ActiveModel, created_at, updated_at);
"#,
    );

    // 11. Advanced SeaORM Access
    print_example(
        "11. Access Full SeaORM API",
        r#"
// Get the underlying Select for advanced operations
let (select, db) = Post::query(db).into_select();

// Use SeaORM's full query API
let posts = select
    .join(JoinType::InnerJoin, /* ... */)
    .group_by(/* ... */)
    .having(/* ... */)
    .all(&db)
    .await?;
"#,
    );

    println!("\n=== Summary ===");
    println!("✓ Query Builder with method chaining");
    println!("✓ Laravel-style where(), order_by(), limit()");
    println!("✓ Relationship helpers (BelongsTo, HasMany)");
    println!("✓ Eager loading to prevent N+1 queries");
    println!("✓ Model lifecycle events");
    println!("✓ Transaction support with automatic rollback");
    println!("✓ Full SeaORM compatibility");
    println!("\nrf-orm provides a familiar Laravel/Eloquent-like API");
    println!("while maintaining Rust's type safety and async performance!");
}

fn print_example(title: &str, code: &str) {
    println!("=== {} ===", title);
    println!("{}", code.trim());
    println!();
}
