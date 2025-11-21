//! Integration tests for HasOne relationship
//!
//! These tests verify that HasOne relationships load REAL data from the database,
//! executing actual SeaORM queries.

use rf_eloquent::prelude::*;
use sea_orm::{
    entity::prelude::*, Database, DbBackend, DbErr, Schema, Set,
};

// ============================================================================
// Test Entities - Users
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct UserModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum UserRelation {
    #[sea_orm(has_one = "profile::Entity")]
    Profile,
}

impl Related<profile::Entity> for UserEntity {
    fn to() -> RelationDef {
        UserRelation::Profile.def()
    }
}

impl ActiveModelBehavior for UserActiveModel {}

// Create user module
pub mod user {
    pub use super::UserEntity as Entity;
    pub use super::UserModel as Model;
    pub use super::UserActiveModel as ActiveModel;
    pub use super::UserColumn as Column;
    pub use super::UserRelation as Relation;
}

// ============================================================================
// Test Entities - Profiles (HasOne relationship to Users)
// ============================================================================

pub mod profile {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "profiles")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        pub bio: String,
        pub avatar_url: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::UserId",
            to = "super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Test Entities - Addresses (Another HasOne relationship)
// ============================================================================

pub mod address {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "addresses")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        pub street: String,
        pub city: String,
        pub country: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::UserId",
            to = "super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Setup in-memory SQLite database for testing
async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create schema
    let schema = Schema::new(DbBackend::Sqlite);

    // Users table
    let stmt = schema.create_table_from_entity(user::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Profiles table
    let stmt = schema.create_table_from_entity(profile::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Addresses table
    let stmt = schema.create_table_from_entity(address::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    Ok(db)
}

// ============================================================================
// Tests - HasOne Relationship
// ============================================================================

#[tokio::test]
async fn test_has_one_loads_related_model() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("John Doe".to_string()),
        email: Set("john@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create profile for this user
    let profile = profile::ActiveModel {
        user_id: Set(user.id),
        bio: Set("Software Engineer".to_string()),
        avatar_url: Set(Some("https://example.com/avatar.jpg".to_string())),
        ..Default::default()
    };
    let profile = profile.insert(&db).await.unwrap();

    // Load relationship using SeaORM's built-in find_related
    let loaded_profile = user
        .find_related(profile::Entity)
        .one(&db)
        .await
        .unwrap();

    // Verify profile was loaded - THIS IS THE KEY TEST!
    assert!(loaded_profile.is_some(), "Should load the user's profile");
    let loaded_profile = loaded_profile.unwrap();
    assert_eq!(loaded_profile.id, profile.id);
    assert_eq!(loaded_profile.user_id, user.id);
    assert_eq!(loaded_profile.bio, "Software Engineer");
    assert_eq!(loaded_profile.avatar_url, Some("https://example.com/avatar.jpg".to_string()));
}

#[tokio::test]
async fn test_has_one_using_query_helper() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Jane Smith".to_string()),
        email: Set("jane@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create profile for this user
    let profile = profile::ActiveModel {
        user_id: Set(user.id),
        bio: Set("Product Manager".to_string()),
        avatar_url: Set(None),
        ..Default::default()
    };
    let profile = profile.insert(&db).await.unwrap();

    // Test the has_one query helper function
    use rf_eloquent::has_one;
    let loaded_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        user.id,
        profile::Column::UserId
    ).await.unwrap();

    // THIS IS THE CRITICAL TEST - should NOT be None!
    assert!(loaded_profile.is_some(), "Should load the user's profile");
    let loaded_profile = loaded_profile.unwrap();
    assert_eq!(loaded_profile.id, profile.id);
    assert_eq!(loaded_profile.user_id, user.id);
    assert_eq!(loaded_profile.bio, "Product Manager");
    assert_eq!(loaded_profile.avatar_url, None);
}

#[tokio::test]
async fn test_has_one_returns_none_for_user_without_profile() {
    let db = setup_test_db().await.unwrap();

    // Create user WITHOUT profile
    let user = user::ActiveModel {
        name: Set("Bob Johnson".to_string()),
        email: Set("bob@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Load relationship using query helper
    use rf_eloquent::has_one;
    let loaded_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        user.id,
        profile::Column::UserId
    ).await.unwrap();

    // Verify None returned
    assert!(loaded_profile.is_none(), "Should return None for user without profile");
}

#[tokio::test]
async fn test_has_one_with_multiple_users() {
    let db = setup_test_db().await.unwrap();

    // Create multiple users with profiles
    let user1 = user::ActiveModel {
        name: Set("Alice".to_string()),
        email: Set("alice@example.com".to_string()),
        ..Default::default()
    };
    let user1 = user1.insert(&db).await.unwrap();

    let user2 = user::ActiveModel {
        name: Set("Charlie".to_string()),
        email: Set("charlie@example.com".to_string()),
        ..Default::default()
    };
    let user2 = user2.insert(&db).await.unwrap();

    // Create profile for user1
    let profile1 = profile::ActiveModel {
        user_id: Set(user1.id),
        bio: Set("Designer".to_string()),
        avatar_url: Set(Some("https://example.com/alice.jpg".to_string())),
        ..Default::default()
    };
    profile1.insert(&db).await.unwrap();

    // Create profile for user2
    let profile2 = profile::ActiveModel {
        user_id: Set(user2.id),
        bio: Set("Developer".to_string()),
        avatar_url: Set(Some("https://example.com/charlie.jpg".to_string())),
        ..Default::default()
    };
    profile2.insert(&db).await.unwrap();

    // Load profiles for both users using query helper
    use rf_eloquent::has_one;
    let alice_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        user1.id,
        profile::Column::UserId
    ).await.unwrap();

    let charlie_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        user2.id,
        profile::Column::UserId
    ).await.unwrap();

    // Verify each user gets their correct profile
    assert!(alice_profile.is_some());
    assert!(charlie_profile.is_some());

    let alice_profile = alice_profile.unwrap();
    let charlie_profile = charlie_profile.unwrap();

    assert_eq!(alice_profile.bio, "Designer");
    assert_eq!(alice_profile.user_id, user1.id);

    assert_eq!(charlie_profile.bio, "Developer");
    assert_eq!(charlie_profile.user_id, user2.id);

    // Verify profiles are distinct
    assert_ne!(alice_profile.id, charlie_profile.id);
}

#[tokio::test]
async fn test_has_one_with_different_relationship_type() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("David Lee".to_string()),
        email: Set("david@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create address (another HasOne relationship)
    let address = address::ActiveModel {
        user_id: Set(user.id),
        street: Set("123 Main St".to_string()),
        city: Set("San Francisco".to_string()),
        country: Set("USA".to_string()),
        ..Default::default()
    };
    let address = address.insert(&db).await.unwrap();

    // Load address using query helper
    use rf_eloquent::has_one;
    let loaded_address = has_one::<address::Entity, address::Model, _>(
        &db,
        user.id,
        address::Column::UserId
    ).await.unwrap();

    // Verify address was loaded correctly
    assert!(loaded_address.is_some(), "Should load the user's address");
    let loaded_address = loaded_address.unwrap();
    assert_eq!(loaded_address.id, address.id);
    assert_eq!(loaded_address.user_id, user.id);
    assert_eq!(loaded_address.street, "123 Main St");
    assert_eq!(loaded_address.city, "San Francisco");
    assert_eq!(loaded_address.country, "USA");
}

#[tokio::test]
async fn test_has_one_query_only_returns_one_result() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Test User".to_string()),
        email: Set("test@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create multiple profiles for the same user (violates HasOne constraint but tests behavior)
    let profile1 = profile::ActiveModel {
        user_id: Set(user.id),
        bio: Set("First Profile".to_string()),
        avatar_url: Set(None),
        ..Default::default()
    };
    profile1.insert(&db).await.unwrap();

    let profile2 = profile::ActiveModel {
        user_id: Set(user.id),
        bio: Set("Second Profile".to_string()),
        avatar_url: Set(None),
        ..Default::default()
    };
    profile2.insert(&db).await.unwrap();

    // Load using has_one - should return only ONE profile, not multiple
    use rf_eloquent::has_one;
    let loaded_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        user.id,
        profile::Column::UserId
    ).await.unwrap();

    // Verify only one result is returned (has_one uses .one() not .all())
    assert!(loaded_profile.is_some(), "Should load one profile");
    // The key is that it's Option<Model>, not Vec<Model>
}

#[tokio::test]
async fn test_has_one_with_non_existent_foreign_key() {
    let db = setup_test_db().await.unwrap();

    // Try to load profile for a user that doesn't exist
    use rf_eloquent::has_one;
    let loaded_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        999, // Non-existent user ID
        profile::Column::UserId
    ).await.unwrap();

    // Verify None returned
    assert!(loaded_profile.is_none(), "Should return None for non-existent foreign key");
}

#[tokio::test]
async fn test_has_one_executes_real_database_query() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Query Test User".to_string()),
        email: Set("query@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create profile
    let profile = profile::ActiveModel {
        user_id: Set(user.id),
        bio: Set("Testing real queries".to_string()),
        avatar_url: Set(Some("https://example.com/test.jpg".to_string())),
        ..Default::default()
    };
    let inserted_profile = profile.insert(&db).await.unwrap();

    // Load using has_one helper
    use rf_eloquent::has_one;
    let loaded_profile = has_one::<profile::Entity, profile::Model, _>(
        &db,
        user.id,
        profile::Column::UserId
    ).await.unwrap();

    // Verify the data matches what was inserted (proving it came from DB)
    assert!(loaded_profile.is_some());
    let loaded_profile = loaded_profile.unwrap();

    // Compare all fields to ensure data integrity
    assert_eq!(loaded_profile.id, inserted_profile.id, "ID should match");
    assert_eq!(loaded_profile.user_id, inserted_profile.user_id, "User ID should match");
    assert_eq!(loaded_profile.bio, inserted_profile.bio, "Bio should match");
    assert_eq!(loaded_profile.avatar_url, inserted_profile.avatar_url, "Avatar URL should match");

    // This proves we're not returning stub data - we're executing real queries
}
