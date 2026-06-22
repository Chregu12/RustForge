//! Smoke tests for top-level `forge` CLI commands that don't need a project.

use assert_cmd::Command;
use predicates::str::contains;

fn forge() -> Command {
    Command::cargo_bin("forge").expect("forge binary")
}

#[test]
fn version_prints_a_version() {
    forge()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("forge"));
}

#[test]
fn help_lists_core_subcommands() {
    forge()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("make"))
        .stdout(contains("new"))
        .stdout(contains("serve"))
        .stdout(contains("migrate"));
}

#[test]
fn make_help_lists_generators() {
    forge()
        .args(["make", "--help"])
        .assert()
        .success()
        .stdout(contains("model"))
        .stdout(contains("controller"))
        .stdout(contains("migration"));
}

#[test]
fn completion_generates_a_bash_script() {
    forge()
        .args(["completion", "bash"])
        .assert()
        .success()
        .stdout(contains("_forge"));
}

#[test]
fn unknown_command_fails() {
    forge().arg("definitely-not-a-command").assert().failure();
}
