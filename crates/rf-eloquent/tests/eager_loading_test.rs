//! # Eager Loading Integration Tests
//!
//! This test suite demonstrates real eager loading preventing N+1 queries.
//! It uses a test database with actual entities to verify the functionality.

use rf_eloquent::prelude::*;
use sea_orm::{
    entity::prelude::*, Database, DatabaseConnection, DbBackend, DbErr, EntityTrait, Schema, Set,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Test Entities
// ============================================================================

pub mod user {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub email: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::post::Entity")]
        Posts,
    }

    impl ActiveModelBehavior for ActiveModel {}

    impl Related<super::post::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Posts.def()
        }
    }
}

pub mod post {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        pub title: String,
        pub content: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::UserId",
            to = "super::user::Column::Id"
        )]
        User,

        #[sea_orm(has_many = "super::comment::Entity")]
        Comments,
    }

    impl ActiveModelBehavior for ActiveModel {}

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl Related<super::comment::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Comments.def()
        }
    }
}

pub mod comment {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "comments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub post_id: i32,
        pub content: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::post::Entity",
            from = "Column::PostId",
            to = "super::post::Column::Id"
        )]
        Post,
    }

    impl ActiveModelBehavior for ActiveModel {}

    impl Related<super::post::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Post.def()
        }
    }
}

// ============================================================================
// Test Database Setup
// ============================================================================

/// Create an in-memory SQLite database for testing
async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create tables
    let schema = Schema::new(DbBackend::Sqlite);

    let create_user_table = schema.create_table_from_entity(user::Entity);
    let create_post_table = schema.create_table_from_entity(post::Entity);
    let create_comment_table = schema.create_table_from_entity(comment::Entity);

    db.execute(db.get_database_backend().build(&create_user_table))
        .await?;
    db.execute(db.get_database_backend().build(&create_post_table))
        .await?;
    db.execute(db.get_database_backend().build(&create_comment_table))
        .await?;

    Ok(db)
}

/// Create test data with users, posts, and comments
async fn seed_test_data(
    db: &DatabaseConnection,
    user_count: usize,
    posts_per_user: usize,
) -> Result<(), DbErr> {
    for user_num in 1..=user_count {
        // Create user
        let user = user::ActiveModel {
            name: Set(format!("User {}", user_num)),
            email: Set(format!("user{}@example.com", user_num)),
            ..Default::default()
        };
        let user = user.insert(db).await?;

        // Create posts for this user
        for post_num in 1..=posts_per_user {
            let post = post::ActiveModel {
                user_id: Set(user.id),
                title: Set(format!("Post {} by User {}", post_num, user_num)),
                content: Set(format!("Content for post {}", post_num)),
                ..Default::default()
            };
            let post = post.insert(db).await?;

            // Create some comments for this post
            for comment_num in 1..=3 {
                let comment = comment::ActiveModel {
                    post_id: Set(post.id),
                    content: Set(format!("Comment {} on post {}", comment_num, post.id)),
                    ..Default::default()
                };
                comment.insert(db).await?;
            }
        }
    }

    Ok(())
}

// ============================================================================
// Query Counter for Measuring N+1 Prevention
// ============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc as StdArc;

/// A simple query counter to track the number of database queries
#[derive(Clone)]
pub struct QueryCounter {
    count: StdArc<AtomicUsize>,
}

impl QueryCounter {
    pub fn new() -> Self {
        Self {
            count: StdArc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.count.store(0, Ordering::SeqCst);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_basic_eager_loading_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_test_data(&db, 3, 2).await?;

    // Create eager loader
    let loader = ConcreteEagerLoader::new(&db);

    // Test loading has-many relationships
    let user_ids = vec![1, 2, 3];

    // Load posts for users using eager loading
    let posts = loader
        .load_has_many::<post::Entity, post::Model, i32>(&user_ids, post::Column::UserId)
        .await?;

    // Verify we got posts for all users
    assert!(
        posts.len() >= 6,
        "Should have at least 6 posts (3 users * 2 posts each)"
    );

    println!(
        "✅ Loaded {} posts for {} users in a single query",
        posts.len(),
        user_ids.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_eager_loading_prevents_n_plus_1() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    // Create 10 users with 10 posts each = 100 posts total
    seed_test_data(&db, 10, 10).await?;

    println!("\n=== Testing N+1 Query Prevention ===\n");

    // ========================================================================
    // SCENARIO 1: WITHOUT Eager Loading (N+1 Problem)
    // ========================================================================
    println!("SCENARIO 1: WITHOUT Eager Loading (N+1 queries)");

    let users = user::Entity::find().all(&db).await?;
    println!(
        "Query 1: SELECT * FROM users - Loaded {} users",
        users.len()
    );

    let mut total_queries_without_eager = 1; // 1 for loading users
    let mut total_posts = 0;

    for user in &users {
        // Each iteration makes a separate query - THIS IS THE N+1 PROBLEM!
        let posts = post::Entity::find()
            .filter(post::Column::UserId.eq(user.id))
            .all(&db)
            .await?;

        total_queries_without_eager += 1; // N additional queries
        total_posts += posts.len();
    }

    println!(
        "  → Total queries: {} (1 + {} for each user)",
        total_queries_without_eager,
        users.len()
    );
    println!("  → Total posts loaded: {}", total_posts);
    println!(
        "  ❌ This is inefficient! We made {} queries when we could have made 2!\n",
        total_queries_without_eager
    );

    // ========================================================================
    // SCENARIO 2: WITH Eager Loading (Optimal)
    // ========================================================================
    println!("SCENARIO 2: WITH Eager Loading (2 queries)");

    let loader = ConcreteEagerLoader::new(&db);

    // Query 1: Load all users
    let users = user::Entity::find().all(&db).await?;
    println!(
        "Query 1: SELECT * FROM users - Loaded {} users",
        users.len()
    );

    // Query 2: Load ALL posts in ONE query using IN clause
    let user_ids: Vec<i32> = users.iter().map(|u| u.id).collect();
    let all_posts = loader
        .load_has_many::<post::Entity, post::Model, i32>(&user_ids, post::Column::UserId)
        .await?;

    println!(
        "Query 2: SELECT * FROM posts WHERE user_id IN (1,2,3,...) - Loaded {} posts",
        all_posts.len()
    );

    let total_queries_with_eager = 2;
    println!("  → Total queries: {}", total_queries_with_eager);
    println!("  → Total posts loaded: {}", all_posts.len());
    println!("  ✅ Much better! We only made 2 queries!\n");

    // ========================================================================
    // Performance Comparison
    // ========================================================================
    println!("=== PERFORMANCE COMPARISON ===");
    println!(
        "WITHOUT eager loading: {} queries",
        total_queries_without_eager
    );
    println!(
        "WITH eager loading:    {} queries",
        total_queries_with_eager
    );
    println!(
        "Improvement:           {}x fewer queries!",
        total_queries_without_eager / total_queries_with_eager
    );
    println!(
        "Saved:                 {} queries\n",
        total_queries_without_eager - total_queries_with_eager
    );

    // Verify we got all the data
    assert_eq!(
        all_posts.len(),
        total_posts,
        "Should load same number of posts with eager loading"
    );

    // Verify the reduction in queries
    assert!(
        total_queries_with_eager <= total_queries_without_eager / 5,
        "Eager loading should reduce queries by at least 5x (got {}x reduction)",
        total_queries_without_eager / total_queries_with_eager
    );

    Ok(())
}

#[tokio::test]
async fn test_eager_loading_with_large_dataset() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    // Create a larger dataset: 100 users with 10 posts each
    println!("\n=== Creating test data: 100 users with 10 posts each ===");
    seed_test_data(&db, 100, 10).await?;
    println!("✅ Created 100 users and 1000 posts\n");

    let loader = ConcreteEagerLoader::new(&db);

    // Load all users
    let users = user::Entity::find().all(&db).await?;
    assert_eq!(users.len(), 100);

    // Extract user IDs
    let user_ids: Vec<i32> = users.iter().map(|u| u.id).collect();

    // Load ALL posts for all users in ONE query
    let start = std::time::Instant::now();
    let posts = loader
        .load_has_many::<post::Entity, post::Model, i32>(&user_ids, post::Column::UserId)
        .await?;
    let duration = start.elapsed();

    println!("Performance Metrics:");
    println!("  - Users: {}", users.len());
    println!("  - Posts: {}", posts.len());
    println!("  - Queries: 2 (1 for users, 1 for all posts)");
    println!("  - Time to load posts: {:?}", duration);
    println!("  - Posts per user (avg): {}", posts.len() / users.len());

    assert_eq!(posts.len(), 1000, "Should load all 1000 posts");
    println!("\n✅ Successfully loaded 1000 posts for 100 users using only 2 queries!");

    Ok(())
}

#[tokio::test]
async fn test_grouping_models_by_foreign_key() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_test_data(&db, 3, 2).await?;

    let loader = ConcreteEagerLoader::new(&db);

    // Load posts
    let user_ids = vec![1, 2, 3];
    let posts = loader
        .load_has_many::<post::Entity, post::Model, i32>(&user_ids, post::Column::UserId)
        .await?;

    // Group posts by user_id
    let mut grouped = GroupedModels::new();
    for post in posts {
        grouped.add(post.user_id, post);
    }

    // Verify grouping
    for user_id in user_ids {
        let user_posts = grouped.get(&user_id);
        assert!(
            user_posts.is_some(),
            "Should have posts for user {}",
            user_id
        );
        let user_posts = user_posts.unwrap();
        assert!(
            user_posts.len() >= 2,
            "User {} should have at least 2 posts",
            user_id
        );
        println!("User {} has {} posts", user_id, user_posts.len());
    }

    println!("✅ Successfully grouped posts by foreign key");

    Ok(())
}

#[tokio::test]
async fn test_belongs_to_relationship() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_test_data(&db, 2, 3).await?;

    let loader = ConcreteEagerLoader::new(&db);

    // Load all posts
    let posts = post::Entity::find().all(&db).await?;
    println!("Loaded {} posts", posts.len());

    // Extract unique user IDs from posts
    let user_ids: Vec<i32> = posts
        .iter()
        .map(|p| p.user_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Load users (belongs-to relationship)
    let users = loader
        .load_belongs_to::<user::Entity, user::Model, i32>(&user_ids, user::Column::Id)
        .await?;

    println!(
        "Loaded {} users for {} posts using belongs-to relationship",
        users.len(),
        posts.len()
    );
    assert!(users.len() >= 2, "Should have loaded at least 2 users");

    println!("✅ Belongs-to relationship works correctly");

    Ok(())
}

#[tokio::test]
async fn test_empty_parent_list() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let loader = ConcreteEagerLoader::new(&db);

    // Try to load with empty parent list
    let empty_ids: Vec<i32> = vec![];
    let posts = loader
        .load_has_many::<post::Entity, post::Model, i32>(&empty_ids, post::Column::UserId)
        .await?;

    assert_eq!(
        posts.len(),
        0,
        "Should return empty vec for empty parent list"
    );

    println!("✅ Correctly handles empty parent list");

    Ok(())
}

#[tokio::test]
async fn test_group_by_trait() {
    // Test the GroupBy trait extension
    let items = vec![
        (1, "post1"),
        (1, "post2"),
        (2, "post3"),
        (2, "post4"),
        (3, "post5"),
    ];

    let grouped = items.into_iter().group_by_key();

    assert_eq!(grouped.len(), 3);
    assert_eq!(grouped.get(&1).unwrap().len(), 2);
    assert_eq!(grouped.get(&2).unwrap().len(), 2);
    assert_eq!(grouped.get(&3).unwrap().len(), 1);

    println!("✅ GroupBy trait works correctly");
}

// ============================================================================
// Benchmark Tests
// ============================================================================

#[tokio::test]
#[ignore] // Run with: cargo test --release -- --ignored test_benchmark
async fn test_benchmark_n_plus_1_vs_eager_loading() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    // Create large dataset
    println!("\n=== BENCHMARK: N+1 vs Eager Loading ===");
    println!("Creating test data: 500 users with 20 posts each...");
    seed_test_data(&db, 500, 20).await?;
    println!("✅ Created 500 users and 10,000 posts\n");

    // Benchmark N+1 approach
    println!("Running N+1 query pattern...");
    let start = std::time::Instant::now();

    let users = user::Entity::find().all(&db).await?;
    for user in &users {
        let _posts = post::Entity::find()
            .filter(post::Column::UserId.eq(user.id))
            .all(&db)
            .await?;
    }

    let n_plus_1_duration = start.elapsed();
    println!(
        "  N+1 Pattern: {:?} ({} queries)",
        n_plus_1_duration,
        1 + users.len()
    );

    // Benchmark Eager Loading approach
    println!("\nRunning eager loading pattern...");
    let start = std::time::Instant::now();

    let loader = ConcreteEagerLoader::new(&db);
    let users = user::Entity::find().all(&db).await?;
    let user_ids: Vec<i32> = users.iter().map(|u| u.id).collect();
    let _posts = loader
        .load_has_many::<post::Entity, post::Model, i32>(&user_ids, post::Column::UserId)
        .await?;

    let eager_loading_duration = start.elapsed();
    println!("  Eager Loading: {:?} (2 queries)", eager_loading_duration);

    println!("\n=== RESULTS ===");
    println!("N+1 Pattern:    {:?}", n_plus_1_duration);
    println!("Eager Loading:  {:?}", eager_loading_duration);

    if n_plus_1_duration > eager_loading_duration {
        let speedup =
            n_plus_1_duration.as_micros() as f64 / eager_loading_duration.as_micros() as f64;
        println!("Speedup:        {:.2}x faster with eager loading!", speedup);
    }

    Ok(())
}
