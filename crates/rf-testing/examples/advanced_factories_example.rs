//! Advanced Factory Features Example
//!
//! Demonstrates factory states, sequences, and relationships

use async_trait::async_trait;
use rf_testing::{
    factory::{Factory, FactoryDefinition},
    factory_advanced::{EnhancedFactory, Sequence},
    FactoryError, Fake,
};
use serde::{Deserialize, Serialize};

// Models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    role: String,
    is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
    id: i32,
    user_id: i32,
    title: String,
    body: String,
    published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Comment {
    id: i32,
    post_id: i32,
    user_id: i32,
    body: String,
}

// User Factory
struct UserFactory {
    model: User,
    sequence: &'static Sequence,
}

impl Default for UserFactory {
    fn default() -> Self {
        static SEQ: once_cell::sync::Lazy<Sequence> = once_cell::sync::Lazy::new(Sequence::new);
        Self {
            model: Self::definition(),
            sequence: &SEQ,
        }
    }
}

impl FactoryDefinition for UserFactory {
    type Model = User;

    fn definition() -> Self::Model {
        static SEQ: once_cell::sync::Lazy<Sequence> = once_cell::sync::Lazy::new(Sequence::new);
        let id = SEQ.next() as i32;

        User {
            id,
            name: Fake::name(),
            email: format!("user{}@example.com", id),
            role: "user".to_string(),
            is_verified: false,
        }
    }
}

rf_testing::impl_factory!(UserFactory, User);

// Post Factory
struct PostFactory {
    model: Post,
    user: Option<User>,
}

impl Default for PostFactory {
    fn default() -> Self {
        Self {
            model: Self::definition(),
            user: None,
        }
    }
}

impl FactoryDefinition for PostFactory {
    type Model = Post;

    fn definition() -> Self::Model {
        static SEQ: once_cell::sync::Lazy<Sequence> = once_cell::sync::Lazy::new(Sequence::new);
        let id = SEQ.next() as i32;

        Post {
            id,
            user_id: 1, // Default user
            title: Fake::sentence(5),
            body: Fake::paragraph(3),
            published: false,
        }
    }
}

rf_testing::impl_factory!(PostFactory, Post);

impl PostFactory {
    /// Set the user for this post
    fn for_user(mut self, user: &User) -> Self {
        self.model.user_id = user.id;
        self
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏭 Advanced Factory Features Example\n");

    // 1. Basic factory usage
    println!("1️⃣  Basic factory usage:");
    let user = UserFactory::new().create().await?;
    println!("   Created user: {} ({})", user.name, user.email);

    // 2. Factory states
    println!("\n2️⃣  Factory states:");

    let admin = UserFactory::new()
        .state(|u| {
            u.role = "admin".to_string();
            u.is_verified = true;
        })
        .create()
        .await?;
    println!(
        "   Created admin: {} - Role: {}, Verified: {}",
        admin.name, admin.role, admin.is_verified
    );

    let unverified = UserFactory::new()
        .state(|u| {
            u.is_verified = false;
        })
        .create()
        .await?;
    println!(
        "   Created unverified user: {} - Verified: {}",
        unverified.name, unverified.is_verified
    );

    // 3. Sequences
    println!("\n3️⃣  Sequence demonstration:");

    let seq = Sequence::new();
    println!(
        "   Sequence values: {}, {}, {}",
        seq.next(),
        seq.next(),
        seq.next()
    );
    println!("   Current value: {}", seq.current());
    seq.reset();
    println!("   After reset: {}", seq.next());

    let custom_seq = Sequence::starting_at(100);
    println!(
        "   Custom sequence starting at 100: {}, {}, {}",
        custom_seq.next(),
        custom_seq.next(),
        custom_seq.next()
    );

    // 4. Batch creation
    println!("\n4️⃣  Batch creation:");

    let users = UserFactory::create_many(5).await?;
    println!("   Created {} users:", users.len());
    for user in &users {
        println!("     - {} ({})", user.name, user.email);
    }

    // 5. Factory relationships
    println!("\n5️⃣  Factory relationships:");

    let author = UserFactory::new()
        .state(|u| {
            u.name = "John Doe".to_string();
            u.role = "author".to_string();
        })
        .create()
        .await?;

    let post = PostFactory::new()
        .for_user(&author)
        .state(|p| {
            p.title = "My First Post".to_string();
            p.published = true;
        })
        .create()
        .await?;

    println!("   Created post by {}:", author.name);
    println!("     Title: {}", post.title);
    println!("     User ID: {}", post.user_id);
    println!("     Published: {}", post.published);

    // 6. Enhanced factory with states
    println!("\n6️⃣  Enhanced factory with predefined states:");

    let enhanced = EnhancedFactory::<UserFactory>::new()
        .define_state("admin", |u| {
            u.role = "admin".to_string();
            u.is_verified = true;
        })
        .define_state("moderator", |u| {
            u.role = "moderator".to_string();
            u.is_verified = true;
        });

    // Would use: enhanced.as_state("admin").create().await?
    println!("   ✓ Enhanced factory configured with admin and moderator states");

    // 7. Multiple relationships
    println!("\n7️⃣  Complex relationships:");

    let user1 = UserFactory::new().create().await?;
    let user2 = UserFactory::new().create().await?;

    let post1 = PostFactory::new()
        .for_user(&user1)
        .state(|p| p.published = true)
        .create()
        .await?;

    let post2 = PostFactory::new()
        .for_user(&user2)
        .state(|p| p.published = true)
        .create()
        .await?;

    println!("   Created posts:");
    println!("     Post {} by user {}", post1.id, post1.user_id);
    println!("     Post {} by user {}", post2.id, post2.user_id);

    // 8. Factory builder pattern
    println!("\n8️⃣  Factory builder pattern:");

    let count_users = UserFactory::count(3).create().await?;
    println!("   Created {} users using count builder", count_users.len());

    // 9. Conditional states
    println!("\n9️⃣  Conditional states:");

    let is_admin = true;
    let conditional_user = if is_admin {
        UserFactory::new()
            .state(|u| {
                u.role = "admin".to_string();
                u.is_verified = true;
            })
            .create()
            .await?
    } else {
        UserFactory::new().create().await?
    };

    println!(
        "   Created conditional user: {} - Role: {}",
        conditional_user.name, conditional_user.role
    );

    // 10. Unique emails with sequence
    println!("\n🔟 Unique emails with sequences:");

    static EMAIL_SEQ: once_cell::sync::Lazy<Sequence> =
        once_cell::sync::Lazy::new(|| Sequence::starting_at(1000));

    let unique_users: Vec<User> = (0..3)
        .map(|_| {
            let id = EMAIL_SEQ.next();
            UserFactory::new().state(move |u| {
                u.email = format!("unique.user.{}@example.com", id);
            })
        })
        .collect::<Vec<_>>();

    println!("   Created users with unique emails:");
    for factory in unique_users {
        let user = factory.create().await?;
        println!("     - {}", user.email);
    }

    println!("\n✅ All advanced factory examples completed successfully!");

    Ok(())
}
