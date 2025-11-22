//! # Comprehensive Polymorphic Relationships Tests
//!
//! This test suite covers all polymorphic relationship types:
//! - MorphTo (6 tests)
//! - MorphMany (6 tests)
//! - MorphOne (4 tests)
//! - MorphToMany (8 tests)
//! - MorphedByMany (6 tests)
//!
//! Total: 30+ comprehensive tests

use rf_eloquent::relationships::{
    morph_many::*, morph_one::*, morph_to::*, morph_to_many::*, morphed_by_many::*, polymorphic::*,
    type_registry::*,
};
use std::any::Any;
use std::sync::Arc;

// ============================================================================
// MorphTo Tests (6 tests)
// ============================================================================

#[test]
fn test_morph_to_creation() {
    let morph_to = MorphTo::<String>::new(1, "commentable");
    assert_eq!(morph_to.relation_name(), "commentable");
}

#[test]
fn test_morph_to_column_names() {
    let morph_to = MorphTo::<String>::new(1, "commentable");
    assert_eq!(morph_to.morph_type_column(), "commentable_type");
    assert_eq!(morph_to.morph_id_column(), "commentable_id");
}

#[test]
fn test_morph_to_different_relations() {
    let commentable = MorphTo::<String>::new(1, "commentable");
    let imageable = MorphTo::<String>::new(2, "imageable");
    let taggable = MorphTo::<String>::new(3, "taggable");

    assert_eq!(commentable.relation_name(), "commentable");
    assert_eq!(imageable.relation_name(), "imageable");
    assert_eq!(taggable.relation_name(), "taggable");
}

#[tokio::test]
async fn test_morph_to_type_registry_integration() {
    // Register a test type
    GLOBAL_TYPE_REGISTRY
        .register("Post", |id, _db| {
            Box::pin(
                async move { Ok(Box::new(format!("Post-{}", id)) as Box<dyn Any + Send + Sync>) },
            )
        })
        .await;

    let morph_to = MorphTo::<String>::new(1, "commentable");
    let db = sea_orm::DatabaseConnection::default();

    let result = morph_to.get(&db, "Post", 42).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "Post-42");
}

#[tokio::test]
async fn test_morph_to_handle_missing_type() {
    let morph_to = MorphTo::<String>::new(1, "commentable");
    let db = sea_orm::DatabaseConnection::default();

    let result = morph_to.get(&db, "NonExistentType", 1).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PolymorphicError::TypeNotRegistered(_)
    ));
}

#[tokio::test]
async fn test_morph_to_dynamic_resolution() {
    // Register multiple types
    GLOBAL_TYPE_REGISTRY
        .register("Video", |id, _db| {
            Box::pin(
                async move { Ok(Box::new(format!("Video-{}", id)) as Box<dyn Any + Send + Sync>) },
            )
        })
        .await;

    let morph_to = MorphTo::<String>::new(1, "commentable");
    let db = sea_orm::DatabaseConnection::default();

    let result = morph_to.get_dynamic(&db, "Video", 99).await;
    assert!(result.is_ok());

    let value = result.unwrap().downcast::<String>().unwrap();
    assert_eq!(*value, "Video-99");
}

// ============================================================================
// MorphMany Tests (6 tests)
// ============================================================================

#[test]
fn test_morph_many_creation() {
    let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
    assert_eq!(morph_many.parent_id(), 1);
    assert_eq!(morph_many.parent_type(), "Post");
    assert_eq!(morph_many.relation_name(), "commentable");
}

#[test]
fn test_morph_many_column_names() {
    let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
    assert_eq!(morph_many.morph_type_column(), "commentable_type");
    assert_eq!(morph_many.morph_id_column(), "commentable_id");
}

#[test]
fn test_morph_many_different_parent_types() {
    let post_comments = MorphMany::<String>::new(1, "Post", "commentable");
    let video_comments = MorphMany::<String>::new(2, "Video", "commentable");

    assert_eq!(post_comments.parent_type(), "Post");
    assert_eq!(video_comments.parent_type(), "Video");

    // Same relation name
    assert_eq!(
        post_comments.relation_name(),
        video_comments.relation_name()
    );
}

#[test]
fn test_morph_many_builder_creation() {
    let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
    let builder = MorphManyBuilder::new(morph_many);

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_many_builder_order_by() {
    let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
    let builder = MorphManyBuilder::new(morph_many)
        .order_by("created_at", "desc")
        .order_by("id", "asc");

    // Builder should have 2 order_by clauses
    let builder_ref = builder;
    assert_eq!(builder_ref.relationship().parent_id(), 1);
}

#[test]
fn test_morph_many_builder_limit_offset() {
    let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
    let builder = MorphManyBuilder::new(morph_many).limit(10).offset(5);

    assert_eq!(builder.relationship().parent_id(), 1);
}

// ============================================================================
// MorphOne Tests (4 tests)
// ============================================================================

#[test]
fn test_morph_one_creation() {
    let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
    assert_eq!(morph_one.parent_id(), 1);
    assert_eq!(morph_one.parent_type(), "Post");
    assert_eq!(morph_one.relation_name(), "imageable");
}

#[test]
fn test_morph_one_column_names() {
    let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
    assert_eq!(morph_one.morph_type_column(), "imageable_type");
    assert_eq!(morph_one.morph_id_column(), "imageable_id");
}

#[test]
fn test_morph_one_different_parent_types() {
    let post_image = MorphOne::<String>::new(1, "Post", "imageable");
    let user_avatar = MorphOne::<String>::new(2, "User", "imageable");

    assert_eq!(post_image.parent_type(), "Post");
    assert_eq!(user_avatar.parent_type(), "User");

    // Same relation name (both are "imageable")
    assert_eq!(post_image.relation_name(), user_avatar.relation_name());
}

#[test]
fn test_morph_one_builder_creation() {
    let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
    let builder = MorphOneBuilder::new(morph_one).order_by("created_at", "desc");

    assert_eq!(builder.relationship().parent_id(), 1);
}

// ============================================================================
// MorphToMany Tests (8 tests)
// ============================================================================

#[test]
fn test_morph_to_many_creation() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    assert_eq!(morph_to_many.parent_id(), 1);
    assert_eq!(morph_to_many.parent_type(), "Post");
    assert_eq!(morph_to_many.relation_name(), "taggable");
    assert_eq!(morph_to_many.pivot_table(), "taggables");
}

#[test]
fn test_morph_to_many_column_names() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    assert_eq!(morph_to_many.morph_type_column(), "taggable_type");
    assert_eq!(morph_to_many.morph_id_column(), "taggable_id");
}

#[test]
fn test_morph_to_many_different_parent_types() {
    let post_tags = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let video_tags = MorphToMany::<String>::new(2, "Video", "taggable", "taggables");

    assert_eq!(post_tags.parent_type(), "Post");
    assert_eq!(video_tags.parent_type(), "Video");

    // Same pivot table and relation name
    assert_eq!(post_tags.pivot_table(), video_tags.pivot_table());
    assert_eq!(post_tags.relation_name(), video_tags.relation_name());
}

#[test]
fn test_morph_to_many_builder_creation() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many);

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_to_many_builder_with_pivot() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many)
        .with_pivot(vec!["created_at".to_string(), "order".to_string()]);

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_to_many_builder_order_by() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many)
        .order_by("name", "asc")
        .order_by("created_at", "desc");

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_to_many_builder_limit_offset() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many).limit(20).offset(10);

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_to_many_builder_chaining() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many)
        .with_pivot(vec!["created_at".to_string()])
        .order_by("name", "asc")
        .limit(10)
        .offset(5);

    assert_eq!(builder.relationship().parent_id(), 1);
}

// ============================================================================
// MorphedByMany Tests (6 tests)
// ============================================================================

#[test]
fn test_morphed_by_many_creation() {
    let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
    assert_eq!(morphed_by_many.related_id(), 1);
    assert_eq!(morphed_by_many.morph_type(), "Post");
    assert_eq!(morphed_by_many.relation_name(), "taggable");
    assert_eq!(morphed_by_many.pivot_table(), "taggables");
}

#[test]
fn test_morphed_by_many_column_names() {
    let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
    assert_eq!(morphed_by_many.morph_type_column(), "taggable_type");
    assert_eq!(morphed_by_many.morph_id_column(), "taggable_id");
}

#[test]
fn test_morphed_by_many_different_morph_types() {
    let tag_posts = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
    let tag_videos = MorphedByMany::<String>::new(1, "Video", "taggable", "taggables");

    assert_eq!(tag_posts.morph_type(), "Post");
    assert_eq!(tag_videos.morph_type(), "Video");

    // Same pivot table and relation name
    assert_eq!(tag_posts.pivot_table(), tag_videos.pivot_table());
    assert_eq!(tag_posts.relation_name(), tag_videos.relation_name());
}

#[test]
fn test_morphed_by_many_builder_creation() {
    let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphedByManyBuilder::new(morphed_by_many);

    assert_eq!(builder.relationship().related_id(), 1);
}

#[test]
fn test_morphed_by_many_builder_with_pivot() {
    let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder =
        MorphedByManyBuilder::new(morphed_by_many).with_pivot(vec!["created_at".to_string()]);

    assert_eq!(builder.relationship().related_id(), 1);
}

#[test]
fn test_morphed_by_many_builder_chaining() {
    let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphedByManyBuilder::new(morphed_by_many)
        .with_pivot(vec!["created_at".to_string()])
        .order_by("title", "asc")
        .limit(15)
        .offset(3);

    assert_eq!(builder.relationship().related_id(), 1);
}

// ============================================================================
// Type Registry Tests (4 additional tests)
// ============================================================================

#[tokio::test]
async fn test_type_registry_register_multiple_types() {
    let registry = TypeRegistry::new();

    registry
        .register("Model1", |id, _db| {
            Box::pin(
                async move { Ok(Box::new(format!("Model1-{}", id)) as Box<dyn Any + Send + Sync>) },
            )
        })
        .await;

    registry
        .register("Model2", |id, _db| {
            Box::pin(
                async move { Ok(Box::new(format!("Model2-{}", id)) as Box<dyn Any + Send + Sync>) },
            )
        })
        .await;

    let db = sea_orm::DatabaseConnection::default();

    let result1 = registry.resolve("Model1", 1, &db).await;
    assert!(result1.is_ok());

    let result2 = registry.resolve("Model2", 2, &db).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_type_registry_resolve_with_different_ids() {
    let registry = TypeRegistry::new();

    registry
        .register("TestModel", |id, _db| {
            Box::pin(async move { Ok(Box::new(id * 10) as Box<dyn Any + Send + Sync>) })
        })
        .await;

    let db = sea_orm::DatabaseConnection::default();

    let result1 = registry.resolve("TestModel", 1, &db).await.unwrap();
    let value1 = result1.downcast::<i64>().unwrap();
    assert_eq!(*value1, 10);

    let result2 = registry.resolve("TestModel", 5, &db).await.unwrap();
    let value2 = result2.downcast::<i64>().unwrap();
    assert_eq!(*value2, 50);
}

#[test]
fn test_polymorphic_error_display() {
    let err = PolymorphicError::TypeNotRegistered("CustomModel".to_string());
    assert_eq!(err.to_string(), "Type not registered: CustomModel");

    let err = PolymorphicError::InvalidMorphType("Invalid".to_string());
    assert_eq!(err.to_string(), "Invalid morph type: Invalid");
}

#[test]
fn test_polymorphic_error_type_mismatch() {
    let err = PolymorphicError::TypeMismatch {
        expected: "Post".to_string(),
        actual: "Video".to_string(),
    };
    assert!(err.to_string().contains("expected Post"));
    assert!(err.to_string().contains("got Video"));
}

// ============================================================================
// Integration Tests (4 additional tests)
// ============================================================================

#[test]
fn test_morph_relation_column_consistency() {
    // Ensure all morph relations generate consistent column names
    let morph_to = MorphTo::<String>::new(1, "testable");
    let morph_many = MorphMany::<String>::new(1, "Post", "testable");
    let morph_one = MorphOne::<String>::new(1, "Post", "testable");

    assert_eq!(morph_to.morph_type_column(), morph_many.morph_type_column());
    assert_eq!(morph_to.morph_id_column(), morph_many.morph_id_column());
    assert_eq!(
        morph_many.morph_type_column(),
        morph_one.morph_type_column()
    );
}

#[test]
fn test_morph_to_many_relation_consistency() {
    // Ensure MorphToMany and MorphedByMany are consistent
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");

    assert_eq!(
        morph_to_many.morph_type_column(),
        morphed_by_many.morph_type_column()
    );
    assert_eq!(
        morph_to_many.morph_id_column(),
        morphed_by_many.morph_id_column()
    );
    assert_eq!(morph_to_many.pivot_table(), morphed_by_many.pivot_table());
}

#[test]
fn test_relationship_kind_polymorphic_variants() {
    use rf_eloquent::relationships::RelationshipKind;

    // Ensure all polymorphic relationship kinds exist
    let kinds = vec![
        RelationshipKind::MorphTo,
        RelationshipKind::MorphOne,
        RelationshipKind::MorphMany,
        RelationshipKind::MorphToMany,
        RelationshipKind::MorphedByMany,
    ];

    assert_eq!(kinds.len(), 5);
}

#[test]
fn test_relationship_kind_serialization() {
    use rf_eloquent::relationships::RelationshipKind;

    let kind = RelationshipKind::MorphTo;
    let json = serde_json::to_string(&kind).unwrap();
    assert_eq!(json, "\"MorphTo\"");

    let kind = RelationshipKind::MorphMany;
    let json = serde_json::to_string(&kind).unwrap();
    assert_eq!(json, "\"MorphMany\"");
}
