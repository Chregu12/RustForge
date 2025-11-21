//! Basic usage example for rf-collections

use rf_collections::{collect, collect_lazy, CollectionMethods};

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: i64,
    name: String,
    active: bool,
    score: i32,
}

fn main() {
    println!("=== rf-collections Example ===\n");

    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            active: true,
            score: 100,
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            active: false,
            score: 85,
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            active: true,
            score: 95,
        },
        User {
            id: 4,
            name: "Diana".to_string(),
            active: true,
            score: 90,
        },
    ];

    // Filter and map
    println!("Active users:");
    let active_names: Vec<String> = collect(users.clone())
        .filter(|u| u.active)
        .map(|u| u.name)
        .to_vec();
    println!("{:?}\n", active_names);

    // Group by
    println!("Users grouped by active status:");
    let grouped = collect(users.clone()).group_by(|u| u.active);
    for (active, group) in grouped {
        println!("Active={}: {} users", active, group.len());
    }
    println!();

    // Sort and take
    println!("Top 2 users by score:");
    let top_users: Vec<String> = collect(users.clone())
        .sort_by(|u| -(u.score as i32))
        .take(2)
        .map(|u| format!("{} ({})", u.name, u.score))
        .to_vec();
    for user in top_users {
        println!("  {}", user);
    }
    println!();

    // Lazy collection
    println!("Lazy processing (first 3 active users):");
    let lazy_result: Vec<String> = collect_lazy(users.into_iter())
        .filter(|u| u.active)
        .map(|u| u.name)
        .take(3)
        .collect();
    println!("{:?}", lazy_result);
}
