// # Polymorphic Relationships Example
//
// This example demonstrates all 5 polymorphic relationship types in RustForge:
// 1. MorphTo - Belongs to multiple model types
// 2. MorphMany - Has many of a polymorphic model
// 3. MorphOne - Has one of a polymorphic model
// 4. MorphToMany - Many-to-many polymorphic
// 5. MorphedByMany - Inverse of MorphToMany
//
// This is a comprehensive guide showing Laravel-equivalent polymorphic relationships in Rust.

use rf_eloquent::relationships::{
    morph_many::MorphMany, morph_one::MorphOne, morph_to::MorphTo, morph_to_many::MorphToMany,
    morphed_by_many::MorphedByMany, type_registry::GLOBAL_TYPE_REGISTRY,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// ============================================================================
// Example 1: Comments System (MorphTo + MorphMany)
// ============================================================================

/// Comment can belong to Post OR Video
#[derive(Debug, Clone)]
pub struct Comment {
    pub id: i64,
    pub commentable_type: String, // "Post" or "Video"
    pub commentable_id: i64,
    pub body: String,
    pub created_at: String,
}

impl Comment {
    /// Get the polymorphic parent (Post or Video)
    pub fn commentable<T>(&self) -> MorphTo<T> {
        MorphTo::new(self.id, "commentable")
    }

    /// Example usage:
    /// ```rust,no_run
    /// let comment = Comment::find(1).await?;
    ///
    /// // Get as Post
    /// let post = comment.commentable::<Post>()
    ///     .get(&db, &comment.commentable_type, comment.commentable_id)
    ///     .await?;
    ///
    /// // Or get as Video
    /// let video = comment.commentable::<Video>()
    ///     .get(&db, &comment.commentable_type, comment.commentable_id)
    ///     .await?;
    /// ```
}

/// Post model
#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
}

impl Post {
    /// Post has many comments (polymorphic)
    pub fn comments(&self) -> MorphMany<Comment> {
        MorphMany::new(self.id, "Post", "commentable")
    }

    /// Example usage:
    /// ```rust,no_run
    /// let post = Post::find(1).await?;
    /// let comments = post.comments()
    ///     .get(&db, comment::Entity, comment::Column::CommentableType, comment::Column::CommentableId)
    ///     .await?;
    ///
    /// println!("Post has {} comments", comments.len());
    /// ```
}

/// Video model
#[derive(Debug, Clone)]
pub struct Video {
    pub id: i64,
    pub title: String,
    pub url: String,
}

impl Video {
    /// Video has many comments (polymorphic)
    pub fn comments(&self) -> MorphMany<Comment> {
        MorphMany::new(self.id, "Video", "commentable")
    }
}

// ============================================================================
// Example 2: Images System (MorphOne)
// ============================================================================

/// Image can belong to Post OR User
#[derive(Debug, Clone)]
pub struct Image {
    pub id: i64,
    pub imageable_type: String, // "Post" or "User"
    pub imageable_id: i64,
    pub url: String,
    pub width: i32,
    pub height: i32,
}

impl Image {
    /// Get the polymorphic parent (Post or User)
    pub fn imageable<T>(&self) -> MorphTo<T> {
        MorphTo::new(self.id, "imageable")
    }
}

impl Post {
    /// Post has one image (polymorphic)
    pub fn image(&self) -> MorphOne<Image> {
        MorphOne::new(self.id, "Post", "imageable")
    }

    /// Example usage:
    /// ```rust,no_run
    /// let post = Post::find(1).await?;
    /// let image = post.image()
    ///     .get(&db, image::Entity, image::Column::ImageableType, image::Column::ImageableId)
    ///     .await?;
    ///
    /// if let Some(img) = image {
    ///     println!("Post image: {}", img.url);
    /// }
    /// ```
}

/// User model
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

impl User {
    /// User has one avatar (polymorphic image)
    pub fn avatar(&self) -> MorphOne<Image> {
        MorphOne::new(self.id, "User", "imageable")
    }

    /// Example usage:
    /// ```rust,no_run
    /// let user = User::find(1).await?;
    /// let avatar = user.avatar()
    ///     .get(&db, image::Entity, image::Column::ImageableType, image::Column::ImageableId)
    ///     .await?;
    /// ```
}

// ============================================================================
// Example 3: Tagging System (MorphToMany + MorphedByMany)
// ============================================================================

/// Tag model
#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

impl Tag {
    /// Tag can have many Posts (inverse polymorphic)
    pub fn posts(&self) -> MorphedByMany<Post> {
        MorphedByMany::new(self.id, "Post", "taggable", "taggables")
    }

    /// Tag can have many Videos (inverse polymorphic)
    pub fn videos(&self) -> MorphedByMany<Video> {
        MorphedByMany::new(self.id, "Video", "taggable", "taggables")
    }

    /// Example usage:
    /// ```rust,no_run
    /// let tag = Tag::find(1).await?; // "technology" tag
    ///
    /// // Get all posts with this tag
    /// let posts = tag.posts()
    ///     .get(&db, post::Entity, "tag_id")
    ///     .await?;
    ///
    /// // Get all videos with this tag
    /// let videos = tag.videos()
    ///     .get(&db, video::Entity, "tag_id")
    ///     .await?;
    ///
    /// println!("Tag '{}' has {} posts and {} videos",
    ///     tag.name, posts.len(), videos.len());
    /// ```
}

impl Post {
    /// Post can have many tags (polymorphic)
    pub fn tags(&self) -> MorphToMany<Tag> {
        MorphToMany::new(self.id, "Post", "taggable", "taggables")
    }

    /// Example usage with attach/detach:
    /// ```rust,no_run
    /// let post = Post::find(1).await?;
    ///
    /// // Attach tags to post
    /// post.tags().attach(&db, vec![1, 2, 3], "tag_id").await?;
    ///
    /// // Get all tags
    /// let tags = post.tags()
    ///     .get(&db, tag::Entity, "tag_id")
    ///     .await?;
    ///
    /// // Detach specific tags
    /// post.tags().detach(&db, vec![2], "tag_id").await?;
    ///
    /// // Sync tags (add missing, remove extras)
    /// post.tags().sync(&db, vec![1, 3, 4], "tag_id").await?;
    /// ```
}

impl Video {
    /// Video can have many tags (polymorphic)
    pub fn tags(&self) -> MorphToMany<Tag> {
        MorphToMany::new(self.id, "Video", "taggable", "taggables")
    }
}

// ============================================================================
// Database Schema Examples
// ============================================================================

/// Example database schema for polymorphic relationships
///
/// ```sql
/// -- Comments table (MorphTo/MorphMany)
/// CREATE TABLE comments (
///     id BIGINT PRIMARY KEY,
///     commentable_type VARCHAR(255) NOT NULL, -- "Post" or "Video"
///     commentable_id BIGINT NOT NULL,
///     body TEXT NOT NULL,
///     created_at TIMESTAMP
/// );
/// CREATE INDEX idx_commentable ON comments(commentable_type, commentable_id);
///
/// -- Images table (MorphOne)
/// CREATE TABLE images (
///     id BIGINT PRIMARY KEY,
///     imageable_type VARCHAR(255) NOT NULL, -- "Post" or "User"
///     imageable_id BIGINT NOT NULL,
///     url VARCHAR(255) NOT NULL,
///     width INT,
///     height INT
/// );
/// CREATE UNIQUE INDEX idx_imageable ON images(imageable_type, imageable_id);
///
/// -- Taggables pivot table (MorphToMany/MorphedByMany)
/// CREATE TABLE taggables (
///     tag_id BIGINT NOT NULL,
///     taggable_type VARCHAR(255) NOT NULL, -- "Post" or "Video"
///     taggable_id BIGINT NOT NULL,
///     created_at TIMESTAMP,
///     PRIMARY KEY (tag_id, taggable_type, taggable_id)
/// );
/// CREATE INDEX idx_taggable ON taggables(taggable_type, taggable_id);
/// ```

// ============================================================================
// Type Registry Setup
// ============================================================================

/// Setup the type registry for polymorphic relationships
/// This must be called during application initialization
pub async fn setup_type_registry() {
    use std::any::Any;

    // Register Post type
    GLOBAL_TYPE_REGISTRY
        .register("Post", |id, db| {
            Box::pin(async move {
                // In real implementation, query the database
                // let post = post::Entity::find_by_id(id).one(&*db).await?;
                // Ok(Box::new(post) as Box<dyn Any + Send + Sync>)

                // Placeholder for example
                let post = Post {
                    id,
                    title: format!("Post {}", id),
                    content: "Content".to_string(),
                };
                Ok(Box::new(post) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    // Register Video type
    GLOBAL_TYPE_REGISTRY
        .register("Video", |id, db| {
            Box::pin(async move {
                let video = Video {
                    id,
                    title: format!("Video {}", id),
                    url: "https://example.com/video".to_string(),
                };
                Ok(Box::new(video) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    // Register User type
    GLOBAL_TYPE_REGISTRY
        .register("User", |id, db| {
            Box::pin(async move {
                let user = User {
                    id,
                    name: format!("User {}", id),
                    email: format!("user{}@example.com", id),
                };
                Ok(Box::new(user) as Box<dyn Any + Send + Sync>)
            })
        })
        .await;

    println!("✓ Type registry configured with Post, Video, and User types");
}

// ============================================================================
// Complete Usage Example
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================================");
    println!("  Polymorphic Relationships Example");
    println!("=================================================\n");

    // Setup type registry
    setup_type_registry().await;

    // Example 1: Comments (MorphTo + MorphMany)
    println!("\n--- Example 1: Comments System ---");
    println!("✓ Post has many comments (MorphMany)");
    println!("✓ Video has many comments (MorphMany)");
    println!("✓ Comment belongs to Post OR Video (MorphTo)");

    let post = Post {
        id: 1,
        title: "My First Post".to_string(),
        content: "Hello World".to_string(),
    };
    let post_comments = post.comments();
    println!("  Post.comments() -> MorphMany<Comment> ready");

    let comment = Comment {
        id: 1,
        commentable_type: "Post".to_string(),
        commentable_id: 1,
        body: "Great post!".to_string(),
        created_at: "2024-01-01".to_string(),
    };
    let commentable = comment.commentable::<Post>();
    println!("  Comment.commentable() -> MorphTo<Post> ready");

    // Example 2: Images (MorphOne)
    println!("\n--- Example 2: Images System ---");
    println!("✓ Post has one image (MorphOne)");
    println!("✓ User has one avatar (MorphOne)");
    println!("✓ Image belongs to Post OR User (MorphTo)");

    let post_image = post.image();
    println!("  Post.image() -> MorphOne<Image> ready");

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };
    let user_avatar = user.avatar();
    println!("  User.avatar() -> MorphOne<Image> ready");

    // Example 3: Tags (MorphToMany + MorphedByMany)
    println!("\n--- Example 3: Tagging System ---");
    println!("✓ Post has many tags (MorphToMany)");
    println!("✓ Video has many tags (MorphToMany)");
    println!("✓ Tag has many posts (MorphedByMany)");
    println!("✓ Tag has many videos (MorphedByMany)");

    let post_tags = post.tags();
    println!("  Post.tags() -> MorphToMany<Tag> ready");

    let tag = Tag {
        id: 1,
        name: "technology".to_string(),
        slug: "technology".to_string(),
    };
    let tag_posts = tag.posts();
    let tag_videos = tag.videos();
    println!("  Tag.posts() -> MorphedByMany<Post> ready");
    println!("  Tag.videos() -> MorphedByMany<Video> ready");

    // Relationship operations
    println!("\n--- Relationship Operations ---");
    println!("Available operations:");
    println!("  • get()       - Load related models");
    println!("  • count()     - Count related models");
    println!("  • exists()    - Check if any related models exist");
    println!("  • attach()    - Attach related models (MorphToMany)");
    println!("  • detach()    - Detach related models (MorphToMany)");
    println!("  • sync()      - Sync related models (MorphToMany)");
    println!("  • toggle()    - Toggle related models (MorphToMany)");

    // Database schema
    println!("\n--- Database Schema ---");
    println!("Polymorphic columns pattern:");
    println!("  {{relation}}_type VARCHAR(255) -- Model type name");
    println!("  {{relation}}_id   BIGINT        -- Model primary key");
    println!("\nExamples:");
    println!("  • commentable_type, commentable_id");
    println!("  • imageable_type, imageable_id");
    println!("  • taggable_type, taggable_id");

    // Laravel comparison
    println!("\n--- Laravel Comparison ---");
    println!("RustForge polymorphic relationships match Laravel's API:");
    println!();
    println!("Laravel:                    RustForge:");
    println!("$comment->commentable;      comment.commentable::<Post>().get(&db, ...)");
    println!("$post->comments;            post.comments().get(&db, ...)");
    println!("$post->tags;                post.tags().get(&db, ...)");
    println!("$tag->posts;                tag.posts().get(&db, ...)");
    println!("$post->tags()->attach([]);  post.tags().attach(&db, vec![], ...)");

    println!("\n=================================================");
    println!("  ✓ All polymorphic relationship types ready!");
    println!("=================================================\n");

    Ok(())
}
