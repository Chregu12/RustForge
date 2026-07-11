// Integration probe: crud_macros
// Adapted from sandbox/probes/crud_macros/src/main.rs
// Exercises Model!/create!/find!/update!/delete! CRUD macros end-to-end.

use rf::prelude::Model;
use rf::prelude::{create, delete, find, update};
use rf_db_facade::Model as ModelTrait;

// Simple syntax: every field becomes a String; `hidden password` is skipped in
// serialisation. Generates `impl rf_db_facade::Model for User { const TABLE = "users" }`.
Model!(User: name, email, hidden password);

// Full syntax: typed fields.
Model!(Post {
    title: String,
    body: String,
});

#[tokio::test]
async fn test_crud_macros() {
    // Create tables for the in-memory SQLite DB.
    rf::DB::statement("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)").unwrap();
    rf::DB::statement("CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT, body TEXT)").unwrap();

    // Model! expansion sanity (struct shape, trait impl, consts).
    assert_eq!(<User as ModelTrait>::TABLE, "users");
    assert_eq!(<Post as ModelTrait>::TABLE, "posts");
    assert_eq!(User::FILLABLE, &["name", "email"]);
    assert_eq!(User::HIDDEN, &["password"]);
    let u = User::default();
    assert!(u.id.is_none() && u.name.is_empty());

    // create! macro: create!(User, name = .., email = ..)
    let created = create!(User, name = "John", email = "john@example.com")
        .expect("create! should return Ok(Value)");
    assert_eq!(created["name"], "John");
    assert_eq!(created["email"], "john@example.com");
    let id = created["id"].as_i64().expect("create! returns a real row id");

    // find! macro: retrieve the row we just created.
    let found: Option<serde_json::Value> = find!(User, id).expect("find! should return Ok");
    let found = found.expect("find! must retrieve the created row");
    assert_eq!(found["email"], "john@example.com");

    // update! macro: real UPDATE, confirmed by re-reading.
    let affected = update!(User, id, name = "John Doe", email = "jd@example.com")
        .expect("update! should return Ok(u64)");
    assert_eq!(affected, 1);
    let reread = find!(User, id).unwrap().unwrap();
    assert_eq!(reread["name"], "John Doe");

    // delete! macro: real DELETE, confirmed gone.
    let deleted = delete!(User, id).expect("delete! should return Ok(u64)");
    assert_eq!(deleted, 1);
    assert!(find!(User, id).unwrap().is_none());

    // Exercise the typed Post model through create! + find! too.
    let post = create!(Post, title = "Hello", body = "World")
        .expect("create!(Post) should return Ok(Value)");
    assert_eq!(post["title"], "Hello");
    let post_id = post["id"].as_i64().expect("real post id");
    let post_back = find!(Post, post_id).unwrap().expect("post retrievable");
    assert_eq!(post_back["body"], "World");
}
