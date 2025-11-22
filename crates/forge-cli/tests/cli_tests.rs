//! Integration tests for the CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("RustForge CLI"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("--version").assert().success();
}

#[test]
fn test_about_command() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("about").assert().success();
}

#[test]
fn test_inspire_command() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("inspire").assert().success();
}

#[test]
fn test_completion_bash() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_completion_zsh() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("completion")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("compdef"));
}

#[test]
fn test_completion_fish() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("completion")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_completion_invalid_shell() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("completion")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown shell"));
}

#[test]
fn test_aliases_command() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("aliases")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available Command Aliases"));
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn test_help_make_model() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("help")
        .arg("make-model")
        .assert()
        .success()
        .stdout(predicate::str::contains("make:model"));
}

#[test]
fn test_make_model_without_project() {
    let dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.current_dir(dir.path())
        .arg("make:model")
        .arg("User")
        .assert()
        .failure();
}

// Tests that require a project directory
fn setup_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create minimal project structure
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
        "#,
    )
    .unwrap();

    dir
}

#[test]
fn test_make_model_with_project() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.current_dir(dir.path())
        .arg("make:model")
        .arg("User")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    // Check that model file was created
    assert!(dir.path().join("src/models/user.rs").exists());
}

#[test]
fn test_make_model_with_migration() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.current_dir(dir.path())
        .arg("make:model")
        .arg("Post")
        .arg("--migration")
        .assert()
        .success();

    // Check that model and migration were created
    assert!(dir.path().join("src/models/post.rs").exists());
}

#[test]
fn test_make_controller_with_project() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.current_dir(dir.path())
        .arg("make:controller")
        .arg("UserController")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    // Check that controller file was created
    assert!(dir
        .path()
        .join("src/controllers/user_controller.rs")
        .exists());
}

#[test]
fn test_make_controller_api() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.current_dir(dir.path())
        .arg("make:controller")
        .arg("ApiController")
        .arg("--api")
        .assert()
        .success();

    // Check controller was created
    assert!(dir
        .path()
        .join("src/controllers/api_controller.rs")
        .exists());
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.arg("invalid-command").assert().failure();
}
