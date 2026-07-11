// Integration probe: scaffold_codegen_verify
// Adapted from sandbox/probes/scaffold_codegen_verify/src/main.rs
// Proves that the scaffolding templates produce code that compiles + runs.

use rf::prelude::*;
use rf::prelude::{create, find};
use rf_db_facade::Model as ModelTrait;

// Zero-field model (compile check: timestamps disabled).
Model!(ScaffoldUser {
    timestamps = false,
});

// Fixed make:model output: plural table name, real field.
Model!(ScaffoldPost {
    name: String,
});

// Fixed routes template output: returns axum::Router.
pub fn routes() -> axum::Router {
    get("/scaffold_users", scaffold_users_index);
    post("/scaffold_users", scaffold_users_store);
    rf::global_router().build_router()
}

async fn scaffold_users_index() -> impl axum::response::IntoResponse {
    json(serde_json::json!({"data": []}))
}

async fn scaffold_users_store() -> impl axum::response::IntoResponse {
    json(serde_json::json!({"data": {}}))
}

#[tokio::test]
async fn test_scaffold_codegen_verify() {
    // Table name consistency.
    assert_eq!(
        <ScaffoldPost as ModelTrait>::TABLE,
        "scaffold_posts",
        "Model!(ScaffoldPost) must target the plural table 'scaffold_posts'"
    );

    // Run the fixed migration SQL.
    rf::DB::statement(
        "CREATE TABLE IF NOT EXISTS scaffold_posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .expect("migration SQL must succeed");

    // create! roundtrip.
    let created = create!(ScaffoldPost, name = "Hello Scaffold")
        .expect("create!(ScaffoldPost, name = ...) must return Ok(Value)");
    assert_eq!(created["name"], "Hello Scaffold");
    let id = created["id"].as_i64().expect("create! must return a real row id");

    // find! roundtrip.
    let found: Option<serde_json::Value> =
        find!(ScaffoldPost, id).expect("find! must return Ok");
    let found = found.expect("find! must retrieve the row we just created");
    assert_eq!(found["name"], "Hello Scaffold");

    // routes() must return an axum::Router (compile-time proof).
    let _: fn() -> axum::Router = routes;
    // ScaffoldPost::FILLABLE must contain the declared sample field.
    assert_eq!(ScaffoldPost::FILLABLE, &["name"]);
}
