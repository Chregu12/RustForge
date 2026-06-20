//! Integration tests for `forge new`.
//!
//! These drive the compiled `forge` binary from a temp directory that lives
//! *outside* the framework checkout. That forces the standalone code path:
//! `find_starter_template` finds nothing on disk, so the binary-embedded
//! `rustforge-starter` is extracted and its `path` dependencies are rewritten
//! to git dependencies. This is exactly what an end user gets after
//! `cargo install --git ... forge-cli`, so it's the path most worth guarding.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

/// Run `forge new <name>` inside `cwd` and assert it succeeds.
fn run_forge_new(cwd: &Path, name: &str) {
    Command::cargo_bin("forge")
        .expect("forge binary should be built")
        .args(["new", name])
        .current_dir(cwd)
        .assert()
        .success();
}

#[test]
fn forge_new_scaffolds_a_standalone_project() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let project = "my-test-app";
    run_forge_new(tmp.path(), project);

    let base = tmp.path().join(project);
    assert!(base.is_dir(), "project directory should be created");

    // Key files from the starter template must be present.
    for expected in [
        "Cargo.toml",
        "src/main.rs",
        "routes/api.rs",
        "routes/web.rs",
        "config/app.toml",
        ".env.example",
    ] {
        assert!(
            base.join(expected).exists(),
            "expected scaffolded file `{expected}` to exist"
        );
    }
}

#[test]
fn forge_new_customizes_the_package_name() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let project = "acme-api";
    run_forge_new(tmp.path(), project);

    let cargo_toml =
        fs::read_to_string(tmp.path().join(project).join("Cargo.toml")).expect("read Cargo.toml");

    assert!(
        cargo_toml.contains(&format!("name = \"{project}\"")),
        "package name should be set to the project name"
    );
    assert!(
        !cargo_toml.contains("my-rustforge-app"),
        "the template's placeholder package name must be replaced"
    );
}

#[test]
fn forge_new_rewrites_path_deps_to_git_for_standalone_projects() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let project = "standalone-app";
    run_forge_new(tmp.path(), project);

    let cargo_toml =
        fs::read_to_string(tmp.path().join(project).join("Cargo.toml")).expect("read Cargo.toml");

    // A standalone project cannot resolve `../crates/rf-*`, so none must remain.
    assert!(
        !cargo_toml.contains("path = \"../crates"),
        "standalone project must not keep `../crates` path dependencies:\n{cargo_toml}"
    );
    // The framework crates must instead come from the public git repo.
    assert!(
        cargo_toml.contains("git = \"https://github.com/Chregu12/RustForge\""),
        "framework deps should be rewritten to a git dependency:\n{cargo_toml}"
    );
}

#[test]
fn forge_new_refuses_an_existing_directory() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let project = "already-here";
    fs::create_dir(tmp.path().join(project)).expect("pre-create dir");

    Command::cargo_bin("forge")
        .expect("forge binary should be built")
        .args(["new", project])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn forge_new_rejects_invalid_project_names() {
    let tmp = tempfile::tempdir().expect("create temp dir");

    Command::cargo_bin("forge")
        .expect("forge binary should be built")
        .args(["new", "Invalid Name!"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}
