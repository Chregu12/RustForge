//! End-to-end tests for the `forge make:*` scaffolding generators.
//!
//! These drive the compiled `forge` binary against a real (temporary)
//! filesystem and assert on the files it produces. `forge make` refuses to run
//! outside a project (it checks for `Cargo.toml` + `src/`), so each test first
//! lays down a minimal project marker; one test exercises the full
//! `forge new` -> `forge make` chain.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a temp dir that looks like a RustForge project (the generators only
/// require `Cargo.toml` + a `src/` directory).
fn forge_project() -> TempDir {
    let tmp = tempfile::tempdir().expect("temp dir");
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    fs::create_dir(tmp.path().join("src")).expect("create src");
    tmp
}

/// Run `forge make <args...>` inside `dir` and assert it succeeds.
fn run_make(dir: &Path, args: &[&str]) {
    let mut cmd = Command::cargo_bin("forge").expect("forge binary");
    cmd.arg("make").args(args).current_dir(dir);
    cmd.assert().success();
}

fn read(dir: &Path, rel: &str) -> String {
    fs::read_to_string(dir.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn assert_contains(haystack: &str, needle: &str, ctx: &str) {
    assert!(
        haystack.contains(needle),
        "{ctx}: expected to find {needle:?} in:\n{haystack}"
    );
}

#[test]
fn make_model_creates_file_and_registers_module() {
    let p = forge_project();
    run_make(p.path(), &["model", "Post"]);

    let model = read(p.path(), "src/models/post.rs");
    assert_contains(&model, "struct Post", "model file");

    // The model module is registered in src/models/mod.rs.
    let mod_rs = read(p.path(), "src/models/mod.rs");
    assert_contains(&mod_rs, "pub mod post", "models/mod.rs");
}

#[test]
fn make_model_with_migration_also_creates_a_migration() {
    let p = forge_project();
    run_make(p.path(), &["model", "Invoice", "--migration"]);

    assert!(
        p.path().join("src/models/invoice.rs").exists(),
        "model file should exist"
    );
    // A migration file (timestamp-prefixed) should have been produced too.
    let migrations = fs::read_dir(p.path().join("src/migrations"))
        .expect("migrations dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        migrations.iter().any(|f| f.ends_with(".rs")),
        "expected a migration file alongside the model, got {migrations:?}"
    );
}

#[test]
fn make_controller_web_uses_html_responses() {
    let p = forge_project();
    run_make(p.path(), &["controller", "PostController"]);

    // Filename is snake_case.
    let body = read(p.path(), "src/controllers/post_controller.rs");
    assert_contains(&body, "struct PostController", "web controller");
    assert_contains(&body, "Html", "web controller should return HTML");
}

#[test]
fn make_controller_api_uses_json_responses() {
    let p = forge_project();
    run_make(p.path(), &["controller", "ApiPostController", "--api"]);

    let body = read(p.path(), "src/controllers/api_post_controller.rs");
    assert_contains(&body, "Json", "api controller should use JSON");
    // API controllers ship a list-query struct; web ones do not.
    assert_contains(&body, "ListQuery", "api controller list query");
}

#[test]
fn make_migration_creates_timestamped_file() {
    let p = forge_project();
    run_make(p.path(), &["migration", "create_users_table"]);

    let found = fs::read_dir(p.path().join("src/migrations"))
        .expect("migrations dir")
        .filter_map(|e| e.ok())
        .any(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.ends_with("_create_users_table.rs") && n.chars().take(8).all(|c| c.is_ascii_digit())
        });
    assert!(found, "expected a timestamped *_create_users_table.rs file");
}

#[test]
fn make_request_includes_validation() {
    let p = forge_project();
    run_make(p.path(), &["request", "StorePostRequest"]);

    let body = read(p.path(), "src/requests/store_post_request.rs");
    assert_contains(&body, "struct StorePostRequest", "request struct");
    assert_contains(&body, "fn validate", "request validation method");
}

#[test]
fn make_factory_and_seeder_land_in_expected_dirs() {
    let p = forge_project();
    run_make(p.path(), &["factory", "PostFactory"]);
    run_make(p.path(), &["seeder", "PostSeeder"]);

    assert!(
        p.path().join("tests/factories/post_factory.rs").exists(),
        "factory should be under tests/factories/"
    );
    assert!(
        p.path().join("database/seeders/post_seeder.rs").exists(),
        "seeder should be under database/seeders/"
    );
}

#[test]
fn make_policy_event_job_middleware_create_files() {
    let p = forge_project();
    run_make(p.path(), &["policy", "PostPolicy"]);
    run_make(p.path(), &["event", "PostCreated"]);
    run_make(p.path(), &["job", "SendEmail"]);
    run_make(p.path(), &["middleware", "Authenticate"]);

    for rel in [
        "src/policies/post_policy.rs",
        "src/events/post_created.rs",
        "src/jobs/send_email.rs",
        // middleware names get a `_middleware` suffix.
        "src/middleware/authenticate_middleware.rs",
    ] {
        assert!(p.path().join(rel).exists(), "expected {rel} to be generated");
    }
}

#[test]
fn make_refuses_to_overwrite_existing_file() {
    let p = forge_project();
    run_make(p.path(), &["model", "Post"]);

    // A second generation of the same model must fail rather than clobber it.
    Command::cargo_bin("forge")
        .expect("forge binary")
        .args(["make", "model", "Post"])
        .current_dir(p.path())
        .assert()
        .failure();
}

#[test]
fn make_refuses_outside_a_project() {
    // A bare temp dir (no Cargo.toml / src) is not a RustForge project.
    let tmp = tempfile::tempdir().expect("temp dir");
    Command::cargo_bin("forge")
        .expect("forge binary")
        .args(["make", "model", "Post"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn new_then_make_end_to_end() {
    // Full chain: scaffold a project with `forge new`, then generate a model
    // inside it with `forge make`.
    let tmp = tempfile::tempdir().expect("temp dir");
    Command::cargo_bin("forge")
        .expect("forge binary")
        .args(["new", "shop"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let project = tmp.path().join("shop");
    assert!(project.join("Cargo.toml").exists(), "project scaffolded");

    Command::cargo_bin("forge")
        .expect("forge binary")
        .args(["make", "model", "Product"])
        .current_dir(&project)
        .assert()
        .success();

    assert!(
        project.join("src/models/product.rs").exists(),
        "make model should work inside a freshly-created project"
    );
}
