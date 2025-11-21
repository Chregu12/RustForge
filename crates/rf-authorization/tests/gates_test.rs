//! Comprehensive tests for Gates

use rf_authorization::gates::{Gate, GateCallback};
use std::sync::Arc;

#[derive(Clone)]
struct User {
    id: i64,
    is_admin: bool,
    permissions: Vec<String>,
}

impl User {
    fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    fn new_admin(id: i64) -> Self {
        Self {
            id,
            is_admin: true,
            permissions: vec![],
        }
    }

    fn new_regular(id: i64, permissions: Vec<String>) -> Self {
        Self {
            id,
            is_admin: false,
            permissions,
        }
    }
}

#[test]
fn test_gate_allows() {
    let mut gate = Gate::new();
    gate.define("create-post", Arc::new(|user: &User, _| {
        user.is_admin || user.has_permission("create-post")
    }));

    let admin = User::new_admin(1);
    let regular_with_permission = User::new_regular(2, vec!["create-post".to_string()]);
    let regular_without_permission = User::new_regular(3, vec![]);

    assert!(gate.allows(&admin, "create-post"));
    assert!(gate.allows(&regular_with_permission, "create-post"));
    assert!(!gate.allows(&regular_without_permission, "create-post"));
}

#[test]
fn test_gate_denies() {
    let mut gate = Gate::new();
    gate.define("delete-post", Arc::new(|user: &User, _| user.is_admin));

    let admin = User::new_admin(1);
    let regular = User::new_regular(2, vec![]);

    assert!(!gate.denies(&admin, "delete-post"));
    assert!(gate.denies(&regular, "delete-post"));
}

#[test]
fn test_gate_authorize_success() {
    let mut gate = Gate::new();
    gate.define("view-dashboard", Arc::new(|user: &User, _| {
        user.has_permission("view-dashboard")
    }));

    let user = User::new_regular(1, vec!["view-dashboard".to_string()]);

    assert!(gate.authorize(&user, "view-dashboard").is_ok());
}

#[test]
fn test_gate_authorize_failure() {
    let mut gate = Gate::new();
    gate.define("view-dashboard", Arc::new(|user: &User, _| {
        user.has_permission("view-dashboard")
    }));

    let user = User::new_regular(1, vec![]);

    let result = gate.authorize(&user, "view-dashboard");
    assert!(result.is_err());
}

#[test]
fn test_gate_default_deny() {
    let gate: Gate<User> = Gate::new();
    let user = User::new_admin(1);

    // Non-existent ability should deny
    assert!(!gate.allows(&user, "undefined-ability"));
    assert!(gate.denies(&user, "undefined-ability"));
}

#[test]
fn test_gate_has() {
    let mut gate = Gate::new();
    gate.define("test-ability", Arc::new(|_: &User, _| true));

    assert!(gate.has("test-ability"));
    assert!(!gate.has("non-existent"));
}

#[test]
fn test_gate_forget() {
    let mut gate = Gate::new();
    gate.define("temporary", Arc::new(|_: &User, _| true));

    assert!(gate.has("temporary"));

    gate.forget("temporary");

    assert!(!gate.has("temporary"));
}

#[test]
fn test_gate_all() {
    let mut gate = Gate::new();
    gate.define("ability1", Arc::new(|_: &User, _| true));
    gate.define("ability2", Arc::new(|_: &User, _| false));

    let abilities = gate.all();
    assert!(abilities.len() >= 2);
    assert!(abilities.contains(&"ability1".to_string()));
    assert!(abilities.contains(&"ability2".to_string()));
}

#[test]
fn test_gate_define_many() {
    let mut gate = Gate::new();

    let definitions: Vec<(&str, GateCallback<User>)> = vec![
        ("ability1", Arc::new(|_: &User, _| true)),
        ("ability2", Arc::new(|_: &User, _| false)),
        ("ability3", Arc::new(|user: &User, _| user.is_admin)),
    ];

    gate.define_many(definitions);

    assert!(gate.has("ability1"));
    assert!(gate.has("ability2"));
    assert!(gate.has("ability3"));
}

#[test]
fn test_gate_allows_all() {
    let mut gate = Gate::new();
    gate.define("read", Arc::new(|user: &User, _| {
        user.has_permission("read")
    }));
    gate.define("write", Arc::new(|user: &User, _| {
        user.has_permission("write")
    }));

    let user_with_all = User::new_regular(1, vec!["read".to_string(), "write".to_string()]);
    let user_with_one = User::new_regular(2, vec!["read".to_string()]);
    let user_with_none = User::new_regular(3, vec![]);

    assert!(gate.allows_all(&user_with_all, &["read", "write"]));
    assert!(!gate.allows_all(&user_with_one, &["read", "write"]));
    assert!(!gate.allows_all(&user_with_none, &["read", "write"]));
}

#[test]
fn test_gate_allows_any() {
    let mut gate = Gate::new();
    gate.define("read", Arc::new(|user: &User, _| {
        user.has_permission("read")
    }));
    gate.define("write", Arc::new(|user: &User, _| {
        user.has_permission("write")
    }));

    let user_with_read = User::new_regular(1, vec!["read".to_string()]);
    let user_with_write = User::new_regular(2, vec!["write".to_string()]);
    let user_with_none = User::new_regular(3, vec![]);

    assert!(gate.allows_any(&user_with_read, &["read", "write"]));
    assert!(gate.allows_any(&user_with_write, &["read", "write"]));
    assert!(!gate.allows_any(&user_with_none, &["read", "write"]));
}

#[test]
fn test_gate_clone() {
    let mut gate = Gate::new();
    gate.define("test", Arc::new(|_: &User, _| true));

    let cloned = gate.clone();

    assert!(cloned.has("test"));

    let user = User::new_admin(1);
    assert!(cloned.allows(&user, "test"));
}

#[test]
fn test_gate_with_ability_parameter() {
    let mut gate = Gate::new();

    // Callback that uses both user and ability parameters
    gate.define("dynamic", Arc::new(|user: &User, ability: &str| {
        user.has_permission(ability)
    }));

    let user = User::new_regular(1, vec!["dynamic".to_string()]);

    assert!(gate.allows(&user, "dynamic"));
}

#[test]
fn test_gate_complex_logic() {
    let mut gate = Gate::new();

    gate.define("publish-post", Arc::new(|user: &User, _| {
        user.is_admin || (
            user.has_permission("posts.publish") &&
            user.has_permission("posts.create")
        )
    }));

    let admin = User::new_admin(1);
    let editor = User::new_regular(2, vec![
        "posts.publish".to_string(),
        "posts.create".to_string(),
    ]);
    let writer = User::new_regular(3, vec!["posts.create".to_string()]);

    assert!(gate.allows(&admin, "publish-post"));
    assert!(gate.allows(&editor, "publish-post"));
    assert!(!gate.allows(&writer, "publish-post"));
}

#[test]
fn test_gate_multiple_users() {
    let mut gate = Gate::new();
    gate.define("admin-only", Arc::new(|user: &User, _| user.is_admin));

    let users = vec![
        User::new_admin(1),
        User::new_regular(2, vec![]),
        User::new_admin(3),
        User::new_regular(4, vec![]),
    ];

    let admin_count = users.iter()
        .filter(|u| gate.allows(u, "admin-only"))
        .count();

    assert_eq!(admin_count, 2);
}

#[test]
fn test_gate_thread_safety() {
    use std::thread;

    let mut gate = Gate::new();
    gate.define("thread-safe", Arc::new(|user: &User, _| user.is_admin));

    let gate = Arc::new(gate);

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                let user = if i % 2 == 0 {
                    User::new_admin(i)
                } else {
                    User::new_regular(i, vec![])
                };
                gate.allows(&user, "thread-safe")
            })
        })
        .collect();

    let results: Vec<bool> = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    // Every other user should be allowed (admins)
    assert_eq!(results.iter().filter(|&&r| r).count(), 5);
}
