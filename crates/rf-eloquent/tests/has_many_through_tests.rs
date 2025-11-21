//! Comprehensive tests for has_many_through relationships
//!
//! These tests verify that has_many_through executes real database queries
//! and correctly loads related models through intermediate tables.

use rf_eloquent::has_many_through;
use sea_orm::{
    entity::prelude::*, Database, DatabaseBackend, DatabaseConnection,
    DbErr, Schema, Set,
};

// ============================================================================
// Test Entities - Country
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "countries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub mod country {
    pub use super::Entity;
    pub use super::Model;
    pub use super::ActiveModel;
    pub use super::Column;
}

// ============================================================================
// Test Entities - User (Through Table)
// ============================================================================

pub mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub country_id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Test Entities - Post (Final Table)
// ============================================================================

pub mod post {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub user_id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Test Entities - Comment (For multi-level through)
// ============================================================================

pub mod comment {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "comments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub content: String,
        pub post_id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Setup an in-memory SQLite database for testing
async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create schema
    let schema = Schema::new(DatabaseBackend::Sqlite);

    // Create countries table
    let stmt = schema.create_table_from_entity(country::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Create users table
    let stmt = schema.create_table_from_entity(user::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Create posts table
    let stmt = schema.create_table_from_entity(post::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Create comments table
    let stmt = schema.create_table_from_entity(comment::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    Ok(db)
}

/// Setup test data with countries, users, and posts
async fn setup_test_data(db: &DatabaseConnection) -> Result<(), DbErr> {
    // Create countries
    let usa = country::ActiveModel {
        id: Set(1),
        name: Set("USA".to_string()),
    };
    usa.insert(db).await?;

    let canada = country::ActiveModel {
        id: Set(2),
        name: Set("Canada".to_string()),
    };
    canada.insert(db).await?;

    let uk = country::ActiveModel {
        id: Set(3),
        name: Set("UK".to_string()),
    };
    uk.insert(db).await?;

    // Create users
    // USA users
    let user1 = user::ActiveModel {
        id: Set(1),
        name: Set("Alice".to_string()),
        country_id: Set(1),
    };
    user1.insert(db).await?;

    let user2 = user::ActiveModel {
        id: Set(2),
        name: Set("Bob".to_string()),
        country_id: Set(1),
    };
    user2.insert(db).await?;

    // Canada users
    let user3 = user::ActiveModel {
        id: Set(3),
        name: Set("Charlie".to_string()),
        country_id: Set(2),
    };
    user3.insert(db).await?;

    // UK user (no posts)
    let user4 = user::ActiveModel {
        id: Set(4),
        name: Set("David".to_string()),
        country_id: Set(3),
    };
    user4.insert(db).await?;

    // Create posts
    // Alice's posts (USA)
    for i in 1..=3 {
        let post = post::ActiveModel {
            id: Set(i),
            title: Set(format!("Alice's Post {}", i)),
            user_id: Set(1),
        };
        post.insert(db).await?;
    }

    // Bob's posts (USA)
    for i in 4..=5 {
        let post = post::ActiveModel {
            id: Set(i),
            title: Set(format!("Bob's Post {}", i - 3)),
            user_id: Set(2),
        };
        post.insert(db).await?;
    }

    // Charlie's posts (Canada)
    for i in 6..=8 {
        let post = post::ActiveModel {
            id: Set(i),
            title: Set(format!("Charlie's Post {}", i - 5)),
            user_id: Set(3),
        };
        post.insert(db).await?;
    }

    // Create comments for posts
    let comment1 = comment::ActiveModel {
        id: Set(1),
        content: Set("Great post!".to_string()),
        post_id: Set(1),
    };
    comment1.insert(db).await?;

    let comment2 = comment::ActiveModel {
        id: Set(2),
        content: Set("Thanks!".to_string()),
        post_id: Set(1),
    };
    comment2.insert(db).await?;

    Ok(())
}

// ============================================================================
// Basic HasManyThrough Tests
// ============================================================================

#[tokio::test]
async fn test_has_many_through_country_to_posts() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get all posts for USA (through users)
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1, // USA country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    // THIS IS THE CRITICAL TEST - posts should NOT be empty!
    assert_eq!(posts.len(), 5, "USA should have 5 posts through its users");

    // Verify post titles
    let titles: Vec<&str> = posts.iter().map(|p| p.title.as_str()).collect();
    assert!(titles.contains(&"Alice's Post 1"));
    assert!(titles.contains(&"Alice's Post 2"));
    assert!(titles.contains(&"Alice's Post 3"));
    assert!(titles.contains(&"Bob's Post 1"));
    assert!(titles.contains(&"Bob's Post 2"));
}

#[tokio::test]
async fn test_has_many_through_canada_to_posts() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get all posts for Canada (through users)
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        2, // Canada country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        posts.len(),
        3,
        "Canada should have 3 posts through its users"
    );

    // Verify all posts belong to Charlie
    for post in &posts {
        assert!(post.title.starts_with("Charlie's"));
    }
}

#[tokio::test]
async fn test_has_many_through_empty_result() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get all posts for UK (has a user but no posts)
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        3, // UK country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        posts.len(),
        0,
        "UK should have 0 posts (user exists but has no posts)"
    );
}

#[tokio::test]
async fn test_has_many_through_non_existent_country() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get all posts for non-existent country
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        999, // Non-existent country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(posts.len(), 0, "Non-existent country should have 0 posts");
}

// ============================================================================
// Multi-Level Through Tests
// ============================================================================

#[tokio::test]
async fn test_has_many_through_multi_level() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get all comments for USA (Country -> Users -> Posts -> Comments)
    // First: Country -> Users -> Posts
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1, // USA country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(posts.len(), 5, "USA should have 5 posts");

    // Second: Posts -> Comments (for first post which has comments)
    let comments = comment::Entity::find()
        .filter(comment::Column::PostId.eq(1))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(
        comments.len(),
        2,
        "Alice's Post 1 should have 2 comments"
    );
}

#[tokio::test]
async fn test_has_many_through_multiple_countries() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Verify each country has the correct number of posts
    let usa_posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1,
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    let canada_posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        2,
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    let uk_posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        3,
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(usa_posts.len(), 5);
    assert_eq!(canada_posts.len(), 3);
    assert_eq!(uk_posts.len(), 0);

    // Total posts should equal sum
    let total = usa_posts.len() + canada_posts.len() + uk_posts.len();
    assert_eq!(total, 8, "Total posts should be 8");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_has_many_through_real_world_scenario() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Scenario: Get all posts from a specific country
    // Useful for: "Show all blog posts from USA authors"

    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1, // USA
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    // Verify we got posts from multiple users in the same country
    let user_ids: Vec<i32> = posts.iter().map(|p| p.user_id).collect();
    assert!(user_ids.contains(&1), "Should have posts from Alice");
    assert!(user_ids.contains(&2), "Should have posts from Bob");

    // Verify posts are ordered by ID (insertion order)
    let post_ids: Vec<i32> = posts.iter().map(|p| p.id).collect();
    assert_eq!(post_ids, vec![1, 2, 3, 4, 5], "Posts should be in order");
}

#[tokio::test]
async fn test_has_many_through_with_additional_filtering() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get posts from USA, then filter manually
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1,
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    // Filter posts from Alice only
    let alice_posts: Vec<_> = posts.iter().filter(|p| p.user_id == 1).collect();
    assert_eq!(alice_posts.len(), 3, "Alice should have 3 posts");
}

#[tokio::test]
async fn test_has_many_through_verifies_sql_correctness() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // This test verifies that the SQL generated is correct by checking
    // that we get the exact posts we expect

    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1, // USA
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    // Manually verify the expected SQL logic:
    // 1. Find users where country_id = 1 (Alice and Bob, IDs 1 and 2)
    // 2. Find posts where user_id IN (1, 2) (posts 1-5)

    let expected_titles = vec![
        "Alice's Post 1",
        "Alice's Post 2",
        "Alice's Post 3",
        "Bob's Post 1",
        "Bob's Post 2",
    ];

    let actual_titles: Vec<String> = posts.iter().map(|p| p.title.clone()).collect();

    for expected in &expected_titles {
        assert!(
            actual_titles.contains(&expected.to_string()),
            "Should contain: {}",
            expected
        );
    }
}

#[tokio::test]
async fn test_has_many_through_preserves_data_integrity() {
    let db = setup_test_db().await.unwrap();
    setup_test_data(&db).await.unwrap();

    // Get posts for Canada
    let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        2,
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await
    .unwrap();

    // Verify all posts belong to users from Canada
    for post in &posts {
        let user = user::Entity::find_by_id(post.user_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            user.country_id, 2,
            "Post should belong to user from Canada"
        );
    }
}
