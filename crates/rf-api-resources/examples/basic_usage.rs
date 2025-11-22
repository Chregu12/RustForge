//! Basic usage example for rf-api-resources

use rf_api_resources::{
    Collection, PaginatedCollection, PaginationMeta, Resource, ResourceCollection,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct User {
    id: i64,
    name: String,
    email: String,
    is_admin: bool,
}

#[derive(Debug, Clone, Serialize)]
struct UserResource {
    id: i64,
    name: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    admin_info: Option<String>,
}

impl Resource for UserResource {}

impl From<User> for UserResource {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            admin_info: if user.is_admin {
                Some("Admin user".to_string())
            } else {
                None
            },
        }
    }
}

fn main() {
    println!("=== rf-api-resources Example ===\n");

    // Single resource
    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        is_admin: true,
    };

    let resource = UserResource::from(user);
    let json = resource.to_json().unwrap();
    println!("Single resource:");
    println!("{}\n", serde_json::to_string_pretty(&json).unwrap());

    // Collection
    let users = vec![
        User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            is_admin: true,
        },
        User {
            id: 2,
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            is_admin: false,
        },
    ];

    let resources: Vec<UserResource> = users.into_iter().map(UserResource::from).collect();
    let collection = Collection::new(resources);
    let json = collection.to_json().unwrap();
    println!("Collection:");
    println!("{}\n", serde_json::to_string_pretty(&json).unwrap());

    // Paginated collection
    let users = vec![User {
        id: 1,
        name: "User 1".to_string(),
        email: "user1@example.com".to_string(),
        is_admin: false,
    }];

    let resources: Vec<UserResource> = users.into_iter().map(UserResource::from).collect();
    let meta = PaginationMeta::new(1, 10, 25);
    let paginated = PaginatedCollection::new(resources, meta);
    let json = paginated.to_json().unwrap();
    println!("Paginated collection:");
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
