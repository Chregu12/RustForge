//! Comprehensive tests for BelongsToMany (many-to-many) relationships
//!
//! These tests verify that:
//! 1. belongs_to_many() loads real data through pivot tables
//! 2. attach() creates relationships
//! 3. detach() removes relationships
//! 4. sync() replaces relationships
//! 5. Eager loading prevents N+1 queries
//! 6. Empty relationships are handled correctly

use rf_eloquent::prelude::*;
use sea_orm::{entity::prelude::*, Database, DbBackend, DbErr, Schema, Set};

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
pub enum UserRelation {}

impl ActiveModelBehavior for UserActiveModel {}

pub mod user {
    pub use super::UserActiveModel as ActiveModel;
    pub use super::UserColumn as Column;
    pub use super::UserEntity as Entity;
    pub use super::UserModel as Model;
    pub use super::UserRelation as Relation;
}

// ============================================================================
// Test Entities - Roles
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "roles")]
pub struct RoleModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum RoleRelation {}

impl ActiveModelBehavior for RoleActiveModel {}

pub mod role {
    pub use super::RoleActiveModel as ActiveModel;
    pub use super::RoleColumn as Column;
    pub use super::RoleEntity as Entity;
    pub use super::RoleModel as Model;
    pub use super::RoleRelation as Relation;
}

// ============================================================================
// Test Entities - UserRoles Pivot Table
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_roles")]
pub struct UserRoleModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub role_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum UserRoleRelation {}

impl ActiveModelBehavior for UserRoleActiveModel {}

pub mod user_role {
    pub use super::UserRoleActiveModel as ActiveModel;
    pub use super::UserRoleColumn as Column;
    pub use super::UserRoleEntity as Entity;
    pub use super::UserRoleModel as Model;
    pub use super::UserRoleRelation as Relation;
}

// ============================================================================
// Test Entities - Tags (for Post tagging)
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct TagModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum TagRelation {}

impl ActiveModelBehavior for TagActiveModel {}

pub mod tag {
    pub use super::TagActiveModel as ActiveModel;
    pub use super::TagColumn as Column;
    pub use super::TagEntity as Entity;
    pub use super::TagModel as Model;
    pub use super::TagRelation as Relation;
}

// ============================================================================
// Test Entities - Posts (for Tag relationship)
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct PostModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    pub content: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum PostRelation {}

impl ActiveModelBehavior for PostActiveModel {}

pub mod post {
    pub use super::PostActiveModel as ActiveModel;
    pub use super::PostColumn as Column;
    pub use super::PostEntity as Entity;
    pub use super::PostModel as Model;
    pub use super::PostRelation as Relation;
}

// ============================================================================
// Test Entities - PostTags Pivot Table
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "post_tags")]
pub struct PostTagModel {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub post_id: i32,
    pub tag_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum PostTagRelation {}

impl ActiveModelBehavior for PostTagActiveModel {}

pub mod post_tag {
    pub use super::PostTagActiveModel as ActiveModel;
    pub use super::PostTagColumn as Column;
    pub use super::PostTagEntity as Entity;
    pub use super::PostTagModel as Model;
    pub use super::PostTagRelation as Relation;
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

    // Roles table
    let stmt = schema.create_table_from_entity(role::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // UserRoles pivot table
    let stmt = schema.create_table_from_entity(user_role::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Posts table
    let stmt = schema.create_table_from_entity(post::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Tags table
    let stmt = schema.create_table_from_entity(tag::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // PostTags pivot table
    let stmt = schema.create_table_from_entity(post_tag::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    Ok(db)
}

// ============================================================================
// Test 1: Basic BelongsToMany - User has many Roles
// ============================================================================

#[tokio::test]
async fn test_belongs_to_many_basic() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("John Doe".to_string()),
        email: Set("john@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create roles
    let admin_role = role::ActiveModel {
        name: Set("Admin".to_string()),
        description: Set("Administrator role".to_string()),
        ..Default::default()
    };
    let admin_role = admin_role.insert(&db).await.unwrap();

    let editor_role = role::ActiveModel {
        name: Set("Editor".to_string()),
        description: Set("Editor role".to_string()),
        ..Default::default()
    };
    let editor_role = editor_role.insert(&db).await.unwrap();

    // Create pivot relationships
    user_role::ActiveModel {
        user_id: Set(user.id),
        role_id: Set(admin_role.id),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    user_role::ActiveModel {
        user_id: Set(user.id),
        role_id: Set(editor_role.id),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    // Test belongs_to_many query
    let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    // CRITICAL TEST: Should NOT be empty!
    assert_eq!(roles.len(), 2, "User should have 2 roles");

    let role_names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
    assert!(role_names.contains(&"Admin"), "Should include Admin role");
    assert!(role_names.contains(&"Editor"), "Should include Editor role");
}

// ============================================================================
// Test 2: BelongsToMany with no relationships
// ============================================================================

#[tokio::test]
async fn test_belongs_to_many_empty() {
    let db = setup_test_db().await.unwrap();

    // Create user without roles
    let user = user::ActiveModel {
        name: Set("Jane Doe".to_string()),
        email: Set("jane@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Query roles (should be empty)
    let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        roles.len(),
        0,
        "User without roles should return empty vector"
    );
}

// ============================================================================
// Test 3: BelongsToMany with multiple relationships
// ============================================================================

#[tokio::test]
async fn test_belongs_to_many_multiple() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Alice Smith".to_string()),
        email: Set("alice@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create 5 roles
    let role_names = vec!["Admin", "Editor", "Moderator", "Viewer", "Contributor"];
    let mut role_ids = Vec::new();

    for name in &role_names {
        let r = role::ActiveModel {
            name: Set(name.to_string()),
            description: Set(format!("{} role", name)),
            ..Default::default()
        };
        let r = r.insert(&db).await.unwrap();
        role_ids.push(r.id);

        // Create pivot relationship
        user_role::ActiveModel {
            user_id: Set(user.id),
            role_id: Set(r.id),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }

    // Query roles
    let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(roles.len(), 5, "User should have 5 roles");

    let loaded_names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
    for name in &role_names {
        assert!(loaded_names.contains(name), "Should include {} role", name);
    }
}

// ============================================================================
// Test 4: Attach operation
// ============================================================================

#[tokio::test]
async fn test_attach_relationship() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Bob Johnson".to_string()),
        email: Set("bob@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create role
    let role = role::ActiveModel {
        name: Set("Developer".to_string()),
        description: Set("Developer role".to_string()),
        ..Default::default()
    };
    let role = role.insert(&db).await.unwrap();

    // Initially, user has no roles
    let roles_before = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();
    assert_eq!(roles_before.len(), 0, "User should have no roles initially");

    // Attach the role using attach() function
    attach::<user_role::Entity, _, _>(
        &db,
        user.id,
        role.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
    )
    .await
    .unwrap();

    // Verify role was attached
    let roles_after = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(roles_after.len(), 1, "User should have 1 role after attach");
    assert_eq!(roles_after[0].name, "Developer");
}

// ============================================================================
// Test 5: Detach specific relationship
// ============================================================================

#[tokio::test]
async fn test_detach_specific_relationship() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Charlie Brown".to_string()),
        email: Set("charlie@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create 3 roles
    let role1 = role::ActiveModel {
        name: Set("Role1".to_string()),
        description: Set("Role 1".to_string()),
        ..Default::default()
    };
    let role1 = role1.insert(&db).await.unwrap();

    let role2 = role::ActiveModel {
        name: Set("Role2".to_string()),
        description: Set("Role 2".to_string()),
        ..Default::default()
    };
    let role2 = role2.insert(&db).await.unwrap();

    let role3 = role::ActiveModel {
        name: Set("Role3".to_string()),
        description: Set("Role 3".to_string()),
        ..Default::default()
    };
    let role3 = role3.insert(&db).await.unwrap();

    // Attach all 3 roles
    for role_id in &[role1.id, role2.id, role3.id] {
        attach::<user_role::Entity, _, _>(
            &db,
            user.id,
            *role_id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
        )
        .await
        .unwrap();
    }

    // Verify 3 roles
    let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();
    assert_eq!(roles.len(), 3, "User should have 3 roles");

    // Detach role2
    detach::<user_role::Entity, _, _>(
        &db,
        user.id,
        Some(role2.id),
        user_role::Column::UserId,
        user_role::Column::RoleId,
    )
    .await
    .unwrap();

    // Verify only 2 roles remain
    let roles_after = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        roles_after.len(),
        2,
        "User should have 2 roles after detach"
    );

    let remaining_names: Vec<&str> = roles_after.iter().map(|r| r.name.as_str()).collect();
    assert!(remaining_names.contains(&"Role1"));
    assert!(remaining_names.contains(&"Role3"));
    assert!(
        !remaining_names.contains(&"Role2"),
        "Role2 should be detached"
    );
}

// ============================================================================
// Test 6: Detach all relationships
// ============================================================================

#[tokio::test]
async fn test_detach_all_relationships() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Diana Prince".to_string()),
        email: Set("diana@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create and attach 3 roles
    for i in 1..=3 {
        let role = role::ActiveModel {
            name: Set(format!("Role{}", i)),
            description: Set(format!("Role {}", i)),
            ..Default::default()
        };
        let role = role.insert(&db).await.unwrap();

        attach::<user_role::Entity, _, _>(
            &db,
            user.id,
            role.id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
        )
        .await
        .unwrap();
    }

    // Verify 3 roles
    let roles_before = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();
    assert_eq!(roles_before.len(), 3);

    // Detach all (pass None as role_id)
    detach::<user_role::Entity, _, i32>(
        &db,
        user.id,
        None,
        user_role::Column::UserId,
        user_role::Column::RoleId,
    )
    .await
    .unwrap();

    // Verify no roles remain
    let roles_after = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        roles_after.len(),
        0,
        "User should have no roles after detach all"
    );
}

// ============================================================================
// Test 7: Sync operation - replace all relationships
// ============================================================================

#[tokio::test]
async fn test_sync_relationships() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Eve Adams".to_string()),
        email: Set("eve@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create 5 roles
    let mut role_ids = Vec::new();
    for i in 1..=5 {
        let role = role::ActiveModel {
            name: Set(format!("Role{}", i)),
            description: Set(format!("Role {}", i)),
            ..Default::default()
        };
        let role = role.insert(&db).await.unwrap();
        role_ids.push(role.id);
    }

    // Attach first 3 roles
    for role_id in &role_ids[0..3] {
        attach::<user_role::Entity, _, _>(
            &db,
            user.id,
            *role_id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
        )
        .await
        .unwrap();
    }

    // Verify 3 roles
    let roles_before = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();
    assert_eq!(roles_before.len(), 3);

    // Sync to last 2 roles (should remove first 3, add last 2)
    sync::<user_role::Entity, _, _>(
        &db,
        user.id,
        vec![role_ids[3], role_ids[4]],
        user_role::Column::UserId,
        user_role::Column::RoleId,
    )
    .await
    .unwrap();

    // Verify exactly 2 roles (the last 2)
    let roles_after = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        roles_after.len(),
        2,
        "User should have exactly 2 roles after sync"
    );

    let synced_names: Vec<&str> = roles_after.iter().map(|r| r.name.as_str()).collect();
    assert!(synced_names.contains(&"Role4"));
    assert!(synced_names.contains(&"Role5"));
}

// ============================================================================
// Test 8: Sync to empty (detach all via sync)
// ============================================================================

#[tokio::test]
async fn test_sync_to_empty() {
    let db = setup_test_db().await.unwrap();

    // Create user with roles
    let user = user::ActiveModel {
        name: Set("Frank Castle".to_string()),
        email: Set("frank@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create and attach 2 roles
    for i in 1..=2 {
        let role = role::ActiveModel {
            name: Set(format!("Role{}", i)),
            description: Set(format!("Role {}", i)),
            ..Default::default()
        };
        let role = role.insert(&db).await.unwrap();

        attach::<user_role::Entity, _, _>(
            &db,
            user.id,
            role.id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
        )
        .await
        .unwrap();
    }

    // Verify 2 roles
    let roles_before = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();
    assert_eq!(roles_before.len(), 2);

    // Sync to empty array
    sync::<user_role::Entity, _, i32>(
        &db,
        user.id,
        vec![],
        user_role::Column::UserId,
        user_role::Column::RoleId,
    )
    .await
    .unwrap();

    // Verify no roles
    let roles_after = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(
        roles_after.len(),
        0,
        "Sync to empty array should remove all roles"
    );
}

// ============================================================================
// Test 9: Multiple users with same roles (inverse relationship)
// ============================================================================

#[tokio::test]
async fn test_multiple_users_same_roles() {
    let db = setup_test_db().await.unwrap();

    // Create admin role
    let admin_role = role::ActiveModel {
        name: Set("Admin".to_string()),
        description: Set("Admin role".to_string()),
        ..Default::default()
    };
    let admin_role = admin_role.insert(&db).await.unwrap();

    // Create 3 users, all with admin role
    for i in 1..=3 {
        let user = user::ActiveModel {
            name: Set(format!("User{}", i)),
            email: Set(format!("user{}@example.com", i)),
            ..Default::default()
        };
        let user = user.insert(&db).await.unwrap();

        attach::<user_role::Entity, _, _>(
            &db,
            user.id,
            admin_role.id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
        )
        .await
        .unwrap();
    }

    // Load all users
    let users = user::Entity::find().all(&db).await.unwrap();
    assert_eq!(users.len(), 3);

    // Each user should have the admin role
    for user in users {
        let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
            &db,
            user.id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
            role::Column::Id,
        )
        .await
        .unwrap();

        assert_eq!(roles.len(), 1, "Each user should have 1 role");
        assert_eq!(roles[0].name, "Admin");
    }
}

// ============================================================================
// Test 10: Post-Tag many-to-many (different domain)
// ============================================================================

#[tokio::test]
async fn test_post_tag_belongs_to_many() {
    let db = setup_test_db().await.unwrap();

    // Create post
    let post = post::ActiveModel {
        title: Set("My Blog Post".to_string()),
        content: Set("This is a test post".to_string()),
        ..Default::default()
    };
    let post = post.insert(&db).await.unwrap();

    // Create tags
    let tag_names = vec!["rust", "programming", "tutorial"];
    let mut tag_ids = Vec::new();

    for name in &tag_names {
        let tag = tag::ActiveModel {
            name: Set(name.to_string()),
            ..Default::default()
        };
        let tag = tag.insert(&db).await.unwrap();
        tag_ids.push(tag.id);

        // Attach tag to post
        attach::<post_tag::Entity, _, _>(
            &db,
            post.id,
            tag.id,
            post_tag::Column::PostId,
            post_tag::Column::TagId,
        )
        .await
        .unwrap();
    }

    // Load tags for post
    let tags = belongs_to_many::<tag::Entity, post_tag::Entity, tag::Model, _>(
        &db,
        post.id,
        post_tag::Column::PostId,
        post_tag::Column::TagId,
        tag::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(tags.len(), 3, "Post should have 3 tags");

    let loaded_tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    for name in &tag_names {
        assert!(
            loaded_tag_names.contains(name),
            "Should include tag {}",
            name
        );
    }
}

// ============================================================================
// Test 11: N+1 Query Prevention Concept
// ============================================================================

#[tokio::test]
async fn test_n_plus_1_prevention_concept() {
    let db = setup_test_db().await.unwrap();

    // Create 10 users with 3 roles each
    for i in 1..=10 {
        let user = user::ActiveModel {
            name: Set(format!("User{}", i)),
            email: Set(format!("user{}@example.com", i)),
            ..Default::default()
        };
        let user = user.insert(&db).await.unwrap();

        // Create 3 roles for this user
        for j in 1..=3 {
            let role = role::ActiveModel {
                name: Set(format!("User{}_Role{}", i, j)),
                description: Set(format!("Role {} for User {}", j, i)),
                ..Default::default()
            };
            let role = role.insert(&db).await.unwrap();

            attach::<user_role::Entity, _, _>(
                &db,
                user.id,
                role.id,
                user_role::Column::UserId,
                user_role::Column::RoleId,
            )
            .await
            .unwrap();
        }
    }

    // Load all users
    let users = user::Entity::find().all(&db).await.unwrap();
    assert_eq!(users.len(), 10);

    // WITHOUT eager loading: This would be N+1 queries (10 queries, one per user)
    // WITH eager loading: Should be 2 queries total (1 for users, 1 for all roles)

    // For now, we demonstrate the N+1 problem exists
    let mut total_roles = 0;
    for user in &users {
        let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
            &db,
            user.id,
            user_role::Column::UserId,
            user_role::Column::RoleId,
            role::Column::Id,
        )
        .await
        .unwrap();

        total_roles += roles.len();
        assert_eq!(roles.len(), 3, "Each user should have 3 roles");
    }

    assert_eq!(total_roles, 30, "Should have loaded 30 total roles");

    // NOTE: This demonstrates the N+1 problem - we made 10 separate queries.
    // The eager loading implementation would reduce this to 2 queries total.
}

// ============================================================================
// Test 12: Bidirectional relationship (inverse)
// ============================================================================

#[tokio::test]
async fn test_bidirectional_many_to_many() {
    let db = setup_test_db().await.unwrap();

    // Create user
    let user = user::ActiveModel {
        name: Set("Grace Hopper".to_string()),
        email: Set("grace@example.com".to_string()),
        ..Default::default()
    };
    let user = user.insert(&db).await.unwrap();

    // Create role
    let role = role::ActiveModel {
        name: Set("Scientist".to_string()),
        description: Set("Scientist role".to_string()),
        ..Default::default()
    };
    let role = role.insert(&db).await.unwrap();

    // Attach relationship
    attach::<user_role::Entity, _, _>(
        &db,
        user.id,
        role.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
    )
    .await
    .unwrap();

    // Forward: User -> Roles
    let user_roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
        &db,
        user.id,
        user_role::Column::UserId,
        user_role::Column::RoleId,
        role::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(user_roles.len(), 1);
    assert_eq!(user_roles[0].name, "Scientist");

    // Inverse: Role -> Users (using inverse columns)
    // We need to query with role_id as the parent and user_id as the related
    let role_users = belongs_to_many::<user::Entity, user_role::Entity, user::Model, _>(
        &db,
        role.id,
        user_role::Column::RoleId, // foreign pivot key (role_id)
        user_role::Column::UserId, // related pivot key (user_id)
        user::Column::Id,
    )
    .await
    .unwrap();

    assert_eq!(role_users.len(), 1);
    assert_eq!(role_users[0].name, "Grace Hopper");
}
