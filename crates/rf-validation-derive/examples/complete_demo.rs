//! Complete demonstration of the Validate derive macro
//!
//! This example showcases all supported validation features:
//! - String validation (email, URL, length, patterns)
//! - Number validation
//! - Optional fields
//! - Required optional fields
//! - Nested validation
//! - Custom messages
//! - Multiple rules per field

use rf_validation_derive::Validate;
use validator::Validate as ValidatorValidate;

// Basic string validation
#[derive(Debug, Validate)]
struct CreateUser {
    #[validate(required, email, max = 255)]
    email: String,

    #[validate(required, min = 8, max = 128)]
    password: String,

    #[validate(required, min = 2, max = 100)]
    name: String,
}

// Optional fields
#[derive(Debug, Validate)]
struct UpdateUser {
    #[validate(email, max = 255)]
    email: Option<String>,

    #[validate(min = 8, max = 128)]
    password: Option<String>,

    #[validate(url)]
    website: Option<String>,

    #[validate(min = 10, max = 500)]
    bio: Option<String>,
}

// Required optional fields
#[derive(Debug, Validate)]
struct CreatePost {
    #[validate(required)]
    title: Option<String>,

    #[validate(required)]
    content: Option<String>,

    #[validate(url)]
    featured_image: Option<String>,
}

// String patterns
#[derive(Debug, Validate)]
struct SlugData {
    #[validate(starts_with = "post-", lowercase)]
    post_slug: String,

    #[validate(ends_with = ".jpg")]
    image_filename: String,

    #[validate(alpha)]
    category: String,

    #[validate(alpha_numeric)]
    tag: String,
}

// URL and IP validation
#[derive(Debug, Validate)]
struct NetworkConfig {
    #[validate(url)]
    api_endpoint: String,

    #[validate(ip)]
    server_address: String,

    #[validate(uuid)]
    instance_id: String,
}

// Regex validation
#[derive(Debug, Validate)]
struct PhoneContact {
    #[validate(regex = r"^\+?[1-9]\d{1,14}$")]
    phone: String,

    #[validate(regex = r"^[A-Z]{2}\d{4}$")]
    postal_code: String,
}

// Nested validation
#[derive(Debug, Validate)]
struct Tag {
    #[validate(required, min = 2, max = 50)]
    name: String,

    #[validate(lowercase, alpha_numeric)]
    slug: String,
}

#[derive(Debug, Validate)]
struct BlogPost {
    #[validate(required, min = 3, max = 255)]
    title: String,

    #[validate(required, min = 100)]
    content: String,

    #[validate(required, email)]
    author_email: String,

    #[validate]
    tags: Vec<Tag>,
}

// Length constraints
#[derive(Debug, Validate)]
struct Comment {
    #[validate(required, min = 10, max = 1000)]
    content: String,

    #[validate(required, email)]
    author_email: String,

    #[validate(url)]
    author_website: Option<String>,
}

// Case validation
#[derive(Debug, Validate)]
struct DatabaseConfig {
    #[validate(lowercase)]
    database_name: String,

    #[validate(uppercase)]
    environment: String,
}

// Multiple validations
#[derive(Debug, Validate)]
struct Registration {
    #[validate(required, email, max = 255)]
    email: String,

    #[validate(required, min = 8, max = 128, alpha_numeric)]
    username: String,

    #[validate(required, min = 12)]
    password: String,

    #[validate(url)]
    website: Option<String>,

    #[validate(min = 10, max = 500)]
    bio: Option<String>,
}

fn main() {
    println!("=== Validate Derive Macro Demo ===\n");

    // Test 1: Valid user creation
    println!("Test 1: Create User - Valid");
    let user = CreateUser {
        email: "user@example.com".to_string(),
        password: "securepassword123".to_string(),
        name: "John Doe".to_string(),
    };
    match user.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 2: Invalid email
    println!("Test 2: Create User - Invalid Email");
    let user = CreateUser {
        email: "not-an-email".to_string(),
        password: "securepassword123".to_string(),
        name: "John Doe".to_string(),
    };
    match user.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 3: Password too short
    println!("Test 3: Create User - Password Too Short");
    let user = CreateUser {
        email: "user@example.com".to_string(),
        password: "short".to_string(),
        name: "John Doe".to_string(),
    };
    match user.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 4: Update user with optional fields
    println!("Test 4: Update User - All None");
    let update = UpdateUser {
        email: None,
        password: None,
        website: None,
        bio: None,
    };
    match update.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 5: Update user with valid values
    println!("Test 5: Update User - Valid Values");
    let update = UpdateUser {
        email: Some("newemail@example.com".to_string()),
        password: Some("newsecurepass".to_string()),
        website: Some("https://example.com".to_string()),
        bio: Some("This is a bio with enough characters".to_string()),
    };
    match update.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 6: Required optional field
    println!("Test 6: Create Post - Missing Required Optional");
    let post = CreatePost {
        title: None,
        content: Some("Content here".to_string()),
        featured_image: None,
    };
    match post.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 7: String patterns
    println!("Test 7: Slug Data - Valid Patterns");
    let slug = SlugData {
        post_slug: "post-hello-world".to_string(),
        image_filename: "cover.jpg".to_string(),
        category: "Technology".to_string(),
        tag: "rust123".to_string(),
    };
    match slug.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 8: Network config
    println!("Test 8: Network Config - Valid");
    let config = NetworkConfig {
        api_endpoint: "https://api.example.com".to_string(),
        server_address: "192.168.1.1".to_string(),
        instance_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    };
    match config.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 9: Regex validation
    println!("Test 9: Phone Contact - Valid Regex");
    let contact = PhoneContact {
        phone: "+1234567890".to_string(),
        postal_code: "AB1234".to_string(),
    };
    match contact.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 10: Nested validation - Valid
    println!("Test 10: Blog Post - Valid Nested");
    let post = BlogPost {
        title: "Introduction to Rust".to_string(),
        content: "This is a long content about Rust programming language. ".repeat(10),
        author_email: "author@example.com".to_string(),
        tags: vec![
            Tag {
                name: "Rust".to_string(),
                slug: "rust".to_string(),
            },
            Tag {
                name: "Programming".to_string(),
                slug: "programming".to_string(),
            },
        ],
    };
    match post.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 11: Nested validation - Invalid tag
    println!("Test 11: Blog Post - Invalid Nested Tag");
    let post = BlogPost {
        title: "Introduction to Rust".to_string(),
        content: "This is a long content about Rust programming language. ".repeat(10),
        author_email: "author@example.com".to_string(),
        tags: vec![Tag {
            name: "x".to_string(), // Too short
            slug: "x".to_string(),
        }],
    };
    match post.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    // Test 12: Multiple validations
    println!("Test 12: Registration - All Valid");
    let registration = Registration {
        email: "user@example.com".to_string(),
        username: "johndoe123".to_string(),
        password: "verysecurepassword123".to_string(),
        website: Some("https://johndoe.com".to_string()),
        bio: Some("I am a software developer passionate about Rust".to_string()),
    };
    match registration.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }
    println!();

    println!("=== Demo Complete ===");
}
