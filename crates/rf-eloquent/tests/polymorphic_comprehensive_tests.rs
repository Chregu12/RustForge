//! # Comprehensive Polymorphic Relationships Tests (NEW)
//!
//! This test suite provides 20+ real-world tests for polymorphic relationships:
//! - MorphOne (5 tests)
//! - MorphMany (5 tests)
//! - MorphTo (5 tests)
//! - MorphToMany (5 tests)
//! - Integration Tests (5 tests)

use rf_eloquent::polymorphic_impl::{
    morph_many::*, morph_one::*, morph_to::*, morph_to_many::*,
    polymorphic::*, type_registry::*,
};
use std::any::Any;
use std::sync::Arc;

// ============================================================================
// MorphOne Tests (5 comprehensive tests)
// ============================================================================

#[test]
fn test_morph_one_basic_creation() {
    // User → Image (polymorphic)
    let user_image = MorphOne::<String>::new(1, "User", "imageable");
    assert_eq!(user_image.parent_id(), 1);
    assert_eq!(user_image.parent_type(), "User");
    assert_eq!(user_image.relation_name(), "imageable");
}

#[test]
fn test_morph_one_column_name_generation() {
    let morph_one = MorphOne::<String>::new(42, "Post", "imageable");

    // Should generate correct column names
    assert_eq!(morph_one.morph_type_column(), "imageable_type");
    assert_eq!(morph_one.morph_id_column(), "imageable_id");

    // Verify getters
    assert_eq!(morph_one.parent_id(), 42);
    assert_eq!(morph_one.parent_type(), "Post");
}

#[test]
fn test_morph_one_multiple_parent_types() {
    // Post → Image
    let post_image = MorphOne::<String>::new(1, "Post", "imageable");

    // User → Avatar (also uses imageable)
    let user_avatar = MorphOne::<String>::new(2, "User", "imageable");

    // Video → Thumbnail
    let video_thumbnail = MorphOne::<String>::new(3, "Video", "imageable");

    // Different parent types
    assert_eq!(post_image.parent_type(), "Post");
    assert_eq!(user_avatar.parent_type(), "User");
    assert_eq!(video_thumbnail.parent_type(), "Video");

    // Same relation name
    assert_eq!(post_image.relation_name(), user_avatar.relation_name());
    assert_eq!(post_image.relation_name(), video_thumbnail.relation_name());
}

#[test]
fn test_morph_one_builder_pattern() {
    let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
    let builder = MorphOneBuilder::new(morph_one)
        .order_by("created_at", "desc");

    assert_eq!(builder.relationship().parent_id(), 1);
    assert_eq!(builder.relationship().parent_type(), "Post");
}

#[test]
fn test_morph_one_empty_relationship() {
    // Test with non-existent parent
    let morph_one = MorphOne::<String>::new(999999, "NonExistent", "imageable");
    assert_eq!(morph_one.parent_id(), 999999);
    assert_eq!(morph_one.parent_type(), "NonExistent");
}

// ============================================================================
// MorphMany Tests (5 comprehensive tests)
// ============================================================================

#[test]
fn test_morph_many_basic_creation() {
    // Post → Comments (polymorphic)
    let post_comments = MorphMany::<String>::new(1, "Post", "commentable");
    assert_eq!(post_comments.parent_id(), 1);
    assert_eq!(post_comments.parent_type(), "Post");
    assert_eq!(post_comments.relation_name(), "commentable");
}

#[test]
fn test_morph_many_column_name_generation() {
    let morph_many = MorphMany::<String>::new(10, "Video", "commentable");

    assert_eq!(morph_many.morph_type_column(), "commentable_type");
    assert_eq!(morph_many.morph_id_column(), "commentable_id");
}

#[test]
fn test_morph_many_multiple_parent_types() {
    // Post → Comments
    let post_comments = MorphMany::<String>::new(1, "Post", "commentable");

    // Video → Comments
    let video_comments = MorphMany::<String>::new(2, "Video", "commentable");

    // Photo → Comments
    let photo_comments = MorphMany::<String>::new(3, "Photo", "commentable");

    // Different parent types
    assert_eq!(post_comments.parent_type(), "Post");
    assert_eq!(video_comments.parent_type(), "Video");
    assert_eq!(photo_comments.parent_type(), "Photo");

    // Same relation name (all commentable)
    assert_eq!(post_comments.relation_name(), video_comments.relation_name());
    assert_eq!(video_comments.relation_name(), photo_comments.relation_name());
}

#[test]
fn test_morph_many_builder_with_pagination() {
    let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
    let builder = MorphManyBuilder::new(morph_many)
        .order_by("created_at", "desc")
        .limit(10)
        .offset(20);

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_many_builder_complex_ordering() {
    let morph_many = MorphMany::<String>::new(5, "Article", "commentable");
    let builder = MorphManyBuilder::new(morph_many)
        .order_by("created_at", "desc")
        .order_by("votes", "desc")
        .order_by("id", "asc")
        .limit(50);

    assert_eq!(builder.relationship().parent_id(), 5);
    assert_eq!(builder.relationship().parent_type(), "Article");
}

// ============================================================================
// MorphTo Tests (5 comprehensive tests)
// ============================================================================

#[test]
fn test_morph_to_basic_creation() {
    // Comment → Commentable (Post or Video)
    let morph_to = MorphTo::<String>::new(1, "commentable");
    assert_eq!(morph_to.relation_name(), "commentable");
}

#[test]
fn test_morph_to_column_names() {
    let morph_to = MorphTo::<String>::new(42, "taggable");

    assert_eq!(morph_to.morph_type_column(), "taggable_type");
    assert_eq!(morph_to.morph_id_column(), "taggable_id");
}

#[test]
fn test_morph_to_different_relations() {
    // Different polymorphic inverse relations
    let commentable = MorphTo::<String>::new(1, "commentable");
    let imageable = MorphTo::<String>::new(2, "imageable");
    let taggable = MorphTo::<String>::new(3, "taggable");
    let likeable = MorphTo::<String>::new(4, "likeable");

    assert_eq!(commentable.relation_name(), "commentable");
    assert_eq!(imageable.relation_name(), "imageable");
    assert_eq!(taggable.relation_name(), "taggable");
    assert_eq!(likeable.relation_name(), "likeable");
}

#[tokio::test]
async fn test_morph_to_with_type_registry() {
    // Register Post type
    GLOBAL_TYPE_REGISTRY
        .register("Post", |id, _db| {
            Box::pin(async move {
                Ok(Box::new(format!("Post-{}", id)) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    // Register Video type
    GLOBAL_TYPE_REGISTRY
        .register("Video", |id, _db| {
            Box::pin(async move {
                Ok(Box::new(format!("Video-{}", id)) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    let morph_to = MorphTo::<String>::new(1, "commentable");
    let db = sea_orm::DatabaseConnection::default();

    // Test loading Post
    let post_result = morph_to.get(&db, "Post", 100).await;
    assert!(post_result.is_ok());
    assert_eq!(post_result.unwrap().unwrap(), "Post-100");

    // Test loading Video
    let video_result = morph_to.get(&db, "Video", 200).await;
    assert!(video_result.is_ok());
    assert_eq!(video_result.unwrap().unwrap(), "Video-200");
}

#[tokio::test]
async fn test_morph_to_type_not_registered() {
    let morph_to = MorphTo::<String>::new(1, "commentable");
    let db = sea_orm::DatabaseConnection::default();

    // Should fail for unregistered type
    let result = morph_to.get(&db, "UnknownModel", 1).await;
    assert!(result.is_err());
}

// ============================================================================
// MorphToMany Tests (5 comprehensive tests)
// ============================================================================

#[test]
fn test_morph_to_many_basic_creation() {
    // Post → Tags (polymorphic many-to-many)
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");

    assert_eq!(morph_to_many.parent_id(), 1);
    assert_eq!(morph_to_many.parent_type(), "Post");
    assert_eq!(morph_to_many.relation_name(), "taggable");
    assert_eq!(morph_to_many.pivot_table(), "taggables");
}

#[test]
fn test_morph_to_many_column_names() {
    let morph_to_many = MorphToMany::<String>::new(5, "Video", "taggable", "taggables");

    assert_eq!(morph_to_many.morph_type_column(), "taggable_type");
    assert_eq!(morph_to_many.morph_id_column(), "taggable_id");
}

#[test]
fn test_morph_to_many_shared_pivot_table() {
    // Post → Tags
    let post_tags = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");

    // Video → Tags (same pivot table)
    let video_tags = MorphToMany::<String>::new(2, "Video", "taggable", "taggables");

    // Article → Tags
    let article_tags = MorphToMany::<String>::new(3, "Article", "taggable", "taggables");

    // All use same pivot table
    assert_eq!(post_tags.pivot_table(), "taggables");
    assert_eq!(video_tags.pivot_table(), "taggables");
    assert_eq!(article_tags.pivot_table(), "taggables");

    // All use same relation name
    assert_eq!(post_tags.relation_name(), video_tags.relation_name());
    assert_eq!(video_tags.relation_name(), article_tags.relation_name());
}

#[test]
fn test_morph_to_many_builder_with_pivot() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many)
        .with_pivot(vec!["created_at".to_string(), "order".to_string(), "status".to_string()])
        .order_by("name", "asc")
        .limit(25);

    assert_eq!(builder.relationship().parent_id(), 1);
}

#[test]
fn test_morph_to_many_builder_complex_query() {
    let morph_to_many = MorphToMany::<String>::new(10, "Video", "taggable", "taggables");
    let builder = MorphToManyBuilder::new(morph_to_many)
        .with_pivot(vec!["pivot_data".to_string()])
        .order_by("popularity", "desc")
        .order_by("name", "asc")
        .limit(100)
        .offset(50);

    assert_eq!(builder.relationship().parent_type(), "Video");
    assert_eq!(builder.relationship().pivot_table(), "taggables");
}

// ============================================================================
// Integration & Edge Case Tests (5 tests)
// ============================================================================

#[test]
fn test_polymorphic_column_name_consistency() {
    // All polymorphic relationships with same name should generate same column names
    let morph_to = MorphTo::<String>::new(1, "testable");
    let morph_one = MorphOne::<String>::new(1, "Post", "testable");
    let morph_many = MorphMany::<String>::new(1, "Post", "testable");

    assert_eq!(morph_to.morph_type_column(), "testable_type");
    assert_eq!(morph_to.morph_id_column(), "testable_id");

    assert_eq!(morph_one.morph_type_column(), "testable_type");
    assert_eq!(morph_one.morph_id_column(), "testable_id");

    assert_eq!(morph_many.morph_type_column(), "testable_type");
    assert_eq!(morph_many.morph_id_column(), "testable_id");
}

#[test]
fn test_morph_to_many_pivot_consistency() {
    let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");

    // Verify all getters work correctly
    assert_eq!(morph_to_many.parent_id(), 1);
    assert_eq!(morph_to_many.parent_type(), "Post");
    assert_eq!(morph_to_many.relation_name(), "taggable");
    assert_eq!(morph_to_many.pivot_table(), "taggables");
    assert_eq!(morph_to_many.morph_type_column(), "taggable_type");
    assert_eq!(morph_to_many.morph_id_column(), "taggable_id");
}

#[tokio::test]
async fn test_multiple_morphable_types_registered() {
    // Register multiple types
    GLOBAL_TYPE_REGISTRY
        .register("Article", |id, _db| {
            Box::pin(async move {
                Ok(Box::new(format!("Article-{}", id)) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    GLOBAL_TYPE_REGISTRY
        .register("Page", |id, _db| {
            Box::pin(async move {
                Ok(Box::new(format!("Page-{}", id)) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    let morph_to = MorphTo::<String>::new(1, "commentable");
    let db = sea_orm::DatabaseConnection::default();

    // Both should work
    let article = morph_to.get(&db, "Article", 1).await.unwrap().unwrap();
    let page = morph_to.get(&db, "Page", 2).await.unwrap().unwrap();

    assert_eq!(article, "Article-1");
    assert_eq!(page, "Page-2");
}

#[test]
fn test_polymorphic_error_types() {
    // Test error type conversions
    let err = PolymorphicError::TypeNotRegistered("CustomType".to_string());
    assert!(err.to_string().contains("CustomType"));

    let err = PolymorphicError::InvalidMorphType("BadType".to_string());
    assert!(err.to_string().contains("BadType"));

    let err = PolymorphicError::TypeMismatch {
        expected: "Post".to_string(),
        actual: "Video".to_string(),
    };
    assert!(err.to_string().contains("Post"));
    assert!(err.to_string().contains("Video"));
}

#[test]
fn test_polymorphic_builder_edge_cases() {
    // Test builder with zero items
    let morph_many = MorphMany::<String>::new(0, "EmptyType", "relation");
    let builder = MorphManyBuilder::new(morph_many);
    assert_eq!(builder.relationship().parent_id(), 0);

    // Test with very long strings
    let long_type = "Very".repeat(100);
    let morph_one = MorphOne::<String>::new(1, &long_type, "relation");
    assert_eq!(morph_one.parent_type().len(), long_type.len());
}

// ============================================================================
// Type Registry Advanced Tests (5 additional tests)
// ============================================================================

#[tokio::test]
async fn test_type_registry_concurrent_registration() {
    use std::sync::Arc;
    use tokio::task;

    let registry = Arc::new(TypeRegistry::new());

    // Register multiple types concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let reg = Arc::clone(&registry);
            task::spawn(async move {
                reg.register(&format!("Type{}", i), move |id, _db| {
                    Box::pin(async move {
                        Ok(Box::new(id * i) as Box<dyn Any + Send + Sync>)
                    })
                })
                .await;
            })
        })
        .collect();

    // Wait for all registrations
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all types are registered
    let db = sea_orm::DatabaseConnection::default();
    for i in 0..5 {
        let result = registry.resolve(&format!("Type{}", i), 10, &db).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_morph_to_dynamic_resolution() {
    // Test dynamic type resolution
    GLOBAL_TYPE_REGISTRY
        .register("DynamicModel", |id, _db| {
            Box::pin(async move {
                Ok(Box::new(vec![id, id * 2, id * 3]) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    let morph_to = MorphTo::<Vec<i64>>::new(1, "testable");
    let db = sea_orm::DatabaseConnection::default();

    let result = morph_to.get_dynamic(&db, "DynamicModel", 5).await;
    assert!(result.is_ok());

    let value = result.unwrap().downcast::<Vec<i64>>().unwrap();
    assert_eq!(*value, vec![5, 10, 15]);
}

#[test]
fn test_polymorphic_relationships_type_safety() {
    // Ensure type safety at compile time
    let _morph_one_string: MorphOne<String> = MorphOne::new(1, "Post", "imageable");
    let _morph_one_i64: MorphOne<i64> = MorphOne::new(1, "Post", "imageable");
    let _morph_many_vec: MorphMany<Vec<String>> = MorphMany::new(1, "Post", "tags");

    // Should compile with different generic types
}

#[test]
fn test_builder_method_chaining_comprehensive() {
    // Test all builder methods chain correctly
    let morph_many = MorphMany::<String>::new(1, "Post", "comments");
    let builder = MorphManyBuilder::new(morph_many)
        .order_by("created_at", "desc")
        .order_by("votes", "desc")
        .limit(50)
        .offset(100);

    assert_eq!(builder.relationship().parent_id(), 1);

    let morph_to_many = MorphToMany::<String>::new(2, "Video", "tags", "taggables");
    let builder2 = MorphToManyBuilder::new(morph_to_many)
        .with_pivot(vec!["order".to_string(), "status".to_string()])
        .order_by("name", "asc")
        .limit(25)
        .offset(10);

    assert_eq!(builder2.relationship().parent_id(), 2);
}

#[test]
fn test_polymorphic_relation_naming_conventions() {
    // Test various naming conventions
    let snake_case = MorphMany::<String>::new(1, "Post", "comment_able");
    assert_eq!(snake_case.morph_type_column(), "comment_able_type");

    let camel_case = MorphMany::<String>::new(1, "Post", "commentAble");
    assert_eq!(camel_case.morph_type_column(), "commentAble_type");

    let single_word = MorphMany::<String>::new(1, "Post", "owner");
    assert_eq!(single_word.morph_type_column(), "owner_type");

    let with_numbers = MorphMany::<String>::new(1, "Post", "attachment2");
    assert_eq!(with_numbers.morph_type_column(), "attachment2_type");
}
