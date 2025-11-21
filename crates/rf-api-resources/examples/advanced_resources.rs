//! Advanced API Resources Example
//!
//! Demonstrates:
//! - Resource transformation
//! - Conditional attributes
//! - Nested resource loading
//! - Resource collections
//! - Pagination

use rf_api_resources::{
    Collection, NestedResource, PaginatedCollection, PaginationMeta, Resource, ResourceBuilder,
    ResourceTransformer,
};
use serde::Serialize;

// Domain models
#[derive(Clone, Debug)]
struct User {
    id: i64,
    name: String,
    email: String,
    is_admin: bool,
    secret_key: String,
    posts: Option<Vec<Post>>,
}

#[derive(Clone, Debug)]
struct Post {
    id: i64,
    title: String,
    content: String,
    user_id: i64,
}

// Basic resource
#[derive(Serialize)]
struct UserResource {
    id: i64,
    name: String,
    email: String,
}

impl Resource for UserResource {}

impl From<User> for UserResource {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
        }
    }
}

// Resource with conditional fields
#[derive(Serialize)]
struct UserResourceWithConditionals {
    id: i64,
    name: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    admin_badge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_key: Option<String>,
}

impl Resource for UserResourceWithConditionals {}

impl UserResourceWithConditionals {
    fn from_user(user: &User, is_owner: bool) -> Self {
        Self {
            id: user.id,
            name: user.name.clone(),
            email: user.email.clone(),
            admin_badge: if user.is_admin {
                Some("ADMIN".to_string())
            } else {
                None
            },
            secret_key: if is_owner {
                Some(user.secret_key.clone())
            } else {
                None
            },
        }
    }
}

// Resource with nested relations
#[derive(Serialize)]
struct UserWithPostsResource {
    id: i64,
    name: String,
    email: String,
    #[serde(skip_serializing_if = "is_not_loaded")]
    posts: NestedResource<Vec<PostResource>>,
}

fn is_not_loaded<T>(nested: &NestedResource<T>) -> bool {
    !nested.is_loaded()
}

impl Resource for UserWithPostsResource {}

#[derive(Clone, Serialize)]
struct PostResource {
    id: i64,
    title: String,
    content: String,
}

impl Resource for PostResource {}

impl From<Post> for PostResource {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            title: post.title,
            content: post.content,
        }
    }
}

fn example_basic_resource() {
    println!("\n=== Basic Resource ===");

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        is_admin: false,
        secret_key: "secret123".to_string(),
        posts: None,
    };

    let resource = UserResource::from(user);
    let json = resource.to_json().unwrap();

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

fn example_conditional_attributes() {
    println!("\n=== Conditional Attributes ===");

    let admin_user = User {
        id: 1,
        name: "Admin User".to_string(),
        email: "admin@example.com".to_string(),
        is_admin: true,
        secret_key: "admin_secret".to_string(),
        posts: None,
    };

    // As owner (sees secret key)
    let resource_as_owner = UserResourceWithConditionals::from_user(&admin_user, true);
    println!("As owner:");
    println!(
        "{}",
        serde_json::to_string_pretty(&resource_as_owner).unwrap()
    );

    // As guest (doesn't see secret key)
    let resource_as_guest = UserResourceWithConditionals::from_user(&admin_user, false);
    println!("\nAs guest:");
    println!(
        "{}",
        serde_json::to_string_pretty(&resource_as_guest).unwrap()
    );
}

fn example_nested_resources() {
    println!("\n=== Nested Resources ===");

    let posts = vec![
        Post {
            id: 1,
            title: "First Post".to_string(),
            content: "Content 1".to_string(),
            user_id: 1,
        },
        Post {
            id: 2,
            title: "Second Post".to_string(),
            content: "Content 2".to_string(),
            user_id: 1,
        },
    ];

    // Without nested resources loaded
    let resource_without = UserWithPostsResource {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        posts: NestedResource::NotLoaded,
    };

    println!("Without posts loaded:");
    println!(
        "{}",
        serde_json::to_string_pretty(&resource_without).unwrap()
    );

    // With nested resources loaded
    let post_resources: Vec<_> = posts.into_iter().map(PostResource::from).collect();

    let resource_with = UserWithPostsResource {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        posts: NestedResource::loaded(post_resources),
    };

    println!("\nWith posts loaded:");
    println!(
        "{}",
        serde_json::to_string_pretty(&resource_with).unwrap()
    );
}

fn example_resource_builder() {
    println!("\n=== Resource Builder ===");

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        is_admin: true,
        secret_key: "secret123".to_string(),
        posts: None,
    };

    let is_owner = true;

    let resource = ResourceBuilder::new()
        .add("id", user.id)
        .add("name", &user.name)
        .add("email", &user.email)
        .when(user.is_admin, |r| r.add("admin", true))
        .when(is_owner, |r| r.add("secret_key", &user.secret_key))
        .unless(!is_owner, |r| r.add("message", "You are the owner"))
        .build();

    println!("{}", serde_json::to_string_pretty(&resource).unwrap());
}

fn example_collections() {
    println!("\n=== Collections ===");

    let users = vec![
        User {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            is_admin: false,
            secret_key: "".to_string(),
            posts: None,
        },
        User {
            id: 2,
            name: "Jane".to_string(),
            email: "jane@example.com".to_string(),
            is_admin: true,
            secret_key: "".to_string(),
            posts: None,
        },
    ];

    let resources: Vec<_> = users.into_iter().map(UserResource::from).collect();
    let collection = Collection::new(resources);

    println!("Simple collection:");
    println!("{}", serde_json::to_string_pretty(&collection).unwrap());
}

fn example_pagination() {
    println!("\n=== Paginated Collections ===");

    let users = vec![
        User {
            id: 1,
            name: "User 1".to_string(),
            email: "user1@example.com".to_string(),
            is_admin: false,
            secret_key: "".to_string(),
            posts: None,
        },
        User {
            id: 2,
            name: "User 2".to_string(),
            email: "user2@example.com".to_string(),
            is_admin: false,
            secret_key: "".to_string(),
            posts: None,
        },
    ];

    let resources: Vec<_> = users.into_iter().map(UserResource::from).collect();

    // Page 1 of 5 (total 100 items, 20 per page)
    let meta = PaginationMeta::new(1, 20, 100);
    let paginated = PaginatedCollection::new(resources, meta);

    println!("{}", serde_json::to_string_pretty(&paginated).unwrap());
}

fn main() {
    println!("API Resources Examples");
    println!("======================");

    example_basic_resource();
    example_conditional_attributes();
    example_nested_resources();
    example_resource_builder();
    example_collections();
    example_pagination();

    println!("\n=== Done! ===\n");
}
