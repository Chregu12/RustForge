//! # Basic Usage Example for rf-eloquent
//!
//! This example demonstrates the basic features of the rf-eloquent crate:
//! - Relationships (HasOne, HasMany, BelongsTo, BelongsToMany)
//! - Eager Loading
//! - Attribute Casting
//! - Accessors & Mutators
//! - Model Events

use rf_eloquent::prelude::*;
use async_trait::async_trait;

#[derive(Clone, Debug)]
struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Clone, Debug)]
struct Post {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub published: bool,
}

// Implement HasAccessors for User to add computed attributes
impl HasAccessors for User {
    fn get_attribute(&self, key: &str) -> Option<AttributeValue> {
        match key {
            "full_name" => Some(AttributeValue::String(format!(
                "{} {}",
                self.first_name, self.last_name
            ))),
            "display_name" => Some(AttributeValue::String(
                common_accessors::title_case(&self.first_name),
            )),
            _ => None,
        }
    }

    fn has_accessor(&self, key: &str) -> bool {
        matches!(key, "full_name" | "display_name")
    }
}

// Implement HasMutators for User to transform data on set
impl HasMutators for User {
    fn set_attribute(&mut self, key: &str, value: AttributeValue) -> AttributeResult<()> {
        match key {
            "password" => {
                if let Ok(pwd) = value.as_string() {
                    self.password_hash = common_mutators::hash_password(&pwd);
                    Ok(())
                } else {
                    Err(AttributeError::InvalidValue("Invalid password".to_string()))
                }
            }
            "email" => {
                if let Ok(email) = value.as_string() {
                    self.email = email.to_lowercase();
                    Ok(())
                } else {
                    Err(AttributeError::InvalidValue("Invalid email".to_string()))
                }
            }
            _ => Ok(()),
        }
    }

    fn has_mutator(&self, key: &str) -> bool {
        matches!(key, "password" | "email")
    }
}

// Implement attribute casting for Post
impl HasCasts for Post {
    fn casts() -> CastRegistry {
        CastRegistry::new()
            .cast("published", CastType::Boolean)
            .cast("user_id", CastType::Integer)
    }
}

// Implement model events for User
#[async_trait]
impl ModelEvents for User {
    async fn creating(&mut self) -> EventResult {
        // Normalize email before creating
        self.email = self.email.to_lowercase();
        println!("Creating user: {}", self.email);
        Ok(())
    }

    async fn created(&self) -> EventResult {
        // Log after creation
        println!("User created with ID: {}", self.id);
        Ok(())
    }

    async fn updating(&mut self) -> EventResult {
        println!("Updating user: {}", self.id);
        Ok(())
    }

    async fn updated(&self) -> EventResult {
        println!("User updated: {}", self.id);
        Ok(())
    }
}

// Implement model events for Post
#[async_trait]
impl ModelEvents for Post {
    async fn creating(&mut self) -> EventResult {
        // Validate before creating
        if self.title.is_empty() {
            return Err(EventError::ValidationFailed("Title is required".to_string()));
        }
        println!("Creating post: {}", self.title);
        Ok(())
    }

    async fn created(&self) -> EventResult {
        println!("Post created with ID: {}", self.id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== rf-eloquent Basic Usage Example ===\n");

    // Example 1: Accessors
    println!("1. Accessors Example:");
    let user = User {
        id: 1,
        first_name: "john".to_string(),
        last_name: "doe".to_string(),
        email: "john@example.com".to_string(),
        password_hash: String::new(),
    };

    if let Some(full_name) = user.get_attribute("full_name") {
        println!("   Full name: {}", full_name.as_string().unwrap());
    }
    if let Some(display_name) = user.get_attribute("display_name") {
        println!("   Display name: {}", display_name.as_string().unwrap());
    }

    // Example 2: Mutators
    println!("\n2. Mutators Example:");
    let mut user = user;
    user.set_attribute("password", AttributeValue::String("secret123".to_string()))?;
    user.set_attribute("email", AttributeValue::String("JOHN@EXAMPLE.COM".to_string()))?;
    println!("   Email (lowercased): {}", user.email);
    println!("   Password hash: {}", &user.password_hash[..20]);

    // Example 3: Attribute Casting
    println!("\n3. Attribute Casting Example:");
    let post = Post {
        id: 1,
        user_id: 1,
        title: "My First Post".to_string(),
        content: "Hello, World!".to_string(),
        published: true,
    };

    let casts = Post::casts();
    println!("   Post has {} cast definitions", casts.all().len());
    println!("   Published is cast to: {:?}", casts.get("published"));

    // Example 4: Relationship Builders
    println!("\n4. Relationship Builders Example:");
    let has_many_rel = HasMany::<User, Post>::new("user_id");
    println!("   User HasMany Posts via foreign key: {}", has_many_rel.foreign_key());

    let belongs_to_rel = BelongsTo::<Post, User>::new("user_id");
    println!("   Post BelongsTo User via foreign key: {}", belongs_to_rel.foreign_key());

    let belongs_to_many_rel = BelongsToMany::<Post, ()>::new("post_tag", "post_id", "tag_id");
    println!("   Post BelongsToMany Tags via pivot table: {}", belongs_to_many_rel.pivot_table());

    // Example 5: Eager Loading Relations
    println!("\n5. Eager Loading Example:");
    let relation = EagerLoadRelation::from_path("posts.comments.author");
    println!("   Eager load path: {} -> {} -> {}",
        relation.name,
        relation.nested.first().map(|r| r.name.as_str()).unwrap_or(""),
        relation.nested.first()
            .and_then(|r| r.nested.first())
            .map(|r| r.name.as_str())
            .unwrap_or("")
    );

    // Example 6: Model Events
    println!("\n6. Model Events Example:");
    let mut new_user = User {
        id: 2,
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        email: "JANE@EXAMPLE.COM".to_string(),
        password_hash: String::new(),
    };

    // Trigger creating event
    new_user.creating().await?;
    // Trigger created event
    new_user.created().await?;

    // Example 7: Event Observer
    println!("\n7. Event Observer Example:");
    let observer = EventObserver::new();

    // Register event listeners
    observer.creating("User", |ctx| {
        println!("   Event listener triggered: {} at {}", ctx.event.name(), ctx.timestamp);
        Ok(())
    }).await;

    // Fire an event
    let context = EventContext::new(ModelEvent::Creating, "User")
        .with_metadata("user_id", "123");
    observer.fire(context).await?;

    // Example 8: Attribute Bag (for storing virtual attributes)
    println!("\n8. Attribute Bag Example:");
    let mut bag = AttributeBag::new();
    bag.set("computed_field", AttributeValue::String("computed value".to_string()));
    bag.set("score", AttributeValue::Integer(100));

    println!("   Bag has {} attributes", bag.len());
    if let Some(value) = bag.get("computed_field") {
        println!("   Computed field: {}", value.as_string().unwrap());
    }

    // Example 9: Common Accessors
    println!("\n9. Common Accessors Example:");
    let text = "hello world";
    println!("   Original: {}", text);
    println!("   Uppercase: {}", common_accessors::uppercase(text));
    println!("   Title case: {}", common_accessors::title_case(text));
    println!("   Truncate: {}", common_accessors::truncate(text, 5));

    // Example 10: Common Mutators
    println!("\n10. Common Mutators Example:");
    let slug = common_mutators::slugify("Hello World! This is a Test");
    println!("   Slugified: {}", slug);

    let encrypted = common_mutators::encrypt("secret");
    println!("   Encrypted: {}", encrypted);
    let decrypted = common_mutators::decrypt(&encrypted)?;
    println!("   Decrypted: {}", decrypted);

    println!("\n=== Example Complete! ===");
    Ok(())
}
