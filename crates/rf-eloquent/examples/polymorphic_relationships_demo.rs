//! # Polymorphic Relationships Demo
//!
//! Demonstrates all 4 types of polymorphic relationships:
//! - MorphOne: User → Image (one polymorphic)
//! - MorphMany: Post → Comments (many polymorphic)
//! - MorphTo: Comment → Commentable (inverse polymorphic)
//! - MorphToMany: Post/Video → Tags (many-to-many polymorphic)

use rf_eloquent::polymorphic_impl::{
    morph_many::*, morph_one::*, morph_to::*, morph_to_many::*, type_registry::GLOBAL_TYPE_REGISTRY,
};
use std::any::Any;

// ============================================================================
// Example 1: MorphOne (User → Image)
// ============================================================================

/// User has one image (polymorphic)
struct User {
    id: i64,
    name: String,
}

impl User {
    /// Get the user's profile image
    fn image(&self) -> MorphOne<Image> {
        MorphOne::new(self.id, "User", "imageable")
    }
}

/// Image model (can belong to User, Post, Video, etc.)
struct Image {
    id: i64,
    url: String,
    imageable_type: String, // "User", "Post", "Video"
    imageable_id: i64,
}

fn example_morph_one() {
    let user = User {
        id: 1,
        name: "John Doe".to_string(),
    };

    // Create MorphOne relationship
    let image_relation = user.image();

    println!("MorphOne Example:");
    println!("  Parent: User #{}", user.id);
    println!("  Type Column: {}", image_relation.morph_type_column());
    println!("  ID Column: {}", image_relation.morph_id_column());
    println!();

    // In a real app, you'd query the database:
    // let image = image_relation.get(&db, image::Entity, Column::ImageableType, Column::ImageableId).await?;
}

// ============================================================================
// Example 2: MorphMany (Post → Comments)
// ============================================================================

/// Post model
struct Post {
    id: i64,
    title: String,
    body: String,
}

impl Post {
    /// Get all comments for this post
    fn comments(&self) -> MorphMany<Comment> {
        MorphMany::new(self.id, "Post", "commentable")
    }
}

/// Video model (also has comments)
struct Video {
    id: i64,
    title: String,
}

impl Video {
    /// Get all comments for this video
    fn comments(&self) -> MorphMany<Comment> {
        MorphMany::new(self.id, "Video", "commentable")
    }
}

/// Comment model (can belong to Post or Video)
#[derive(Debug, Clone)]
struct Comment {
    id: i64,
    body: String,
    commentable_type: String, // "Post" or "Video"
    commentable_id: i64,
}

fn example_morph_many() {
    let post = Post {
        id: 1,
        title: "My First Post".to_string(),
        body: "Hello world!".to_string(),
    };

    let video = Video {
        id: 2,
        title: "Tutorial Video".to_string(),
    };

    // Both can have comments
    let post_comments = post.comments();
    let video_comments = video.comments();

    println!("MorphMany Example:");
    println!("  Post #{} comments:", post.id);
    println!("    Type: {}", post_comments.parent_type());
    println!("    Relation: {}", post_comments.relation_name());
    println!();
    println!("  Video #{} comments:", video.id);
    println!("    Type: {}", video_comments.parent_type());
    println!("    Relation: {}", video_comments.relation_name());
    println!();

    // With builder for pagination
    let builder = MorphManyBuilder::new(post.comments())
        .order_by("created_at", "desc")
        .limit(10)
        .offset(0);

    println!("  Builder example:");
    println!("    Parent ID: {}", builder.relationship().parent_id());
    println!();

    // In a real app:
    // let comments = post_comments.get(&db, comment::Entity, Column::CommentableType, Column::CommentableId).await?;
}

// ============================================================================
// Example 3: MorphTo (Comment → Commentable)
// ============================================================================

fn example_morph_to() {
    let comment = Comment {
        id: 1,
        body: "Great post!".to_string(),
        commentable_type: "Post".to_string(),
        commentable_id: 1,
    };

    // Get the parent (could be Post or Video)
    let morph_to = MorphTo::<Post>::new(comment.id, "commentable");

    println!("MorphTo Example:");
    println!("  Comment #{}", comment.id);
    println!(
        "  Belongs to: {} #{}",
        comment.commentable_type, comment.commentable_id
    );
    println!("  Type Column: {}", morph_to.morph_type_column());
    println!("  ID Column: {}", morph_to.morph_id_column());
    println!();

    // In a real app, you'd register types and load:
    // GLOBAL_TYPE_REGISTRY.register("Post", |id, db| { ... }).await;
    // let parent = morph_to.get(&db, &comment.commentable_type, comment.commentable_id).await?;
}

// ============================================================================
// Example 4: MorphToMany (Post/Video → Tags)
// ============================================================================

/// Tag model
struct Tag {
    id: i64,
    name: String,
}

impl Post {
    /// Get all tags for this post (many-to-many polymorphic)
    fn tags(&self) -> MorphToMany<Tag> {
        MorphToMany::new(self.id, "Post", "taggable", "taggables")
    }
}

impl Video {
    /// Get all tags for this video (many-to-many polymorphic)
    fn tags(&self) -> MorphToMany<Tag> {
        MorphToMany::new(self.id, "Video", "taggable", "taggables")
    }
}

fn example_morph_to_many() {
    let post = Post {
        id: 1,
        title: "My Post".to_string(),
        body: "Content".to_string(),
    };

    let video = Video {
        id: 2,
        title: "My Video".to_string(),
    };

    let post_tags = post.tags();
    let video_tags = video.tags();

    println!("MorphToMany Example:");
    println!("  Post #{} tags:", post.id);
    println!("    Pivot Table: {}", post_tags.pivot_table());
    println!("    Type: {}", post_tags.parent_type());
    println!();
    println!("  Video #{} tags:", video.id);
    println!("    Pivot Table: {}", video_tags.pivot_table());
    println!("    Type: {}", video_tags.parent_type());
    println!();

    // With pivot columns
    let builder = MorphToManyBuilder::new(post.tags())
        .with_pivot(vec!["created_at".to_string(), "order".to_string()])
        .order_by("name", "asc")
        .limit(20);

    println!("  Builder with pivot:");
    println!("    Parent ID: {}", builder.relationship().parent_id());
    println!();

    // In a real app:
    // let tags = post_tags.get(&db, tag::Entity, "tag_id").await?;
    // post_tags.attach(&db, vec![1, 2, 3], "tag_id").await?;
    // post_tags.sync(&db, vec![1, 2, 3], "tag_id").await?;
}

// ============================================================================
// Example 5: Type Registry
// ============================================================================

#[tokio::main]
async fn example_type_registry() {
    println!("Type Registry Example:");

    // Register Post type
    GLOBAL_TYPE_REGISTRY
        .register("Post", |id, _db| {
            Box::pin(async move {
                // In real app, you'd query the database
                let post = Post {
                    id,
                    title: format!("Post {}", id),
                    body: "Content".to_string(),
                };
                Ok(Box::new(post) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    // Register Video type
    GLOBAL_TYPE_REGISTRY
        .register("Video", |id, _db| {
            Box::pin(async move {
                let video = Video {
                    id,
                    title: format!("Video {}", id),
                };
                Ok(Box::new(video) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    println!("  Registered types: Post, Video");
    println!();

    // Now you can resolve polymorphic relationships dynamically
    let db = sea_orm::DatabaseConnection::default();

    // Example: Resolve a comment's parent
    let comment = Comment {
        id: 1,
        body: "Great content!".to_string(),
        commentable_type: "Post".to_string(),
        commentable_id: 42,
    };

    let morph_to = MorphTo::<Post>::new(comment.id, "commentable");

    // This will use the registry to load the correct type
    // let parent = morph_to.get(&db, &comment.commentable_type, comment.commentable_id).await;

    println!(
        "  Comment belongs to: {} #{}",
        comment.commentable_type, comment.commentable_id
    );
}

// ============================================================================
// Main Demo
// ============================================================================

fn main() {
    println!("====================================");
    println!("Polymorphic Relationships Demo");
    println!("====================================");
    println!();

    example_morph_one();
    example_morph_many();
    example_morph_to();
    example_morph_to_many();

    println!("====================================");
    println!("For type registry example, run:");
    println!("cargo run --example polymorphic_relationships_demo");
    println!("====================================");
}
