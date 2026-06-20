//! End-to-end test: a project produced by `forge new` actually compiles.
//!
//! This is the real "does a generated app build" check. It is `#[ignore]`d by
//! default because it compiles the framework crates the starter depends on,
//! which is slow and disk-hungry. Run it explicitly:
//!
//! ```bash
//! cargo test -p forge-cli --test e2e_generated_project -- --ignored
//! ```
//!
//! To keep it **offline** and fast, it does not fetch the framework from git.
//! Instead it generates a standalone project (as a user would), then repoints
//! the framework git dependencies at the local crates in this checkout and
//! detaches the project from the parent workspace. The generated *source* and
//! manifest are what's under test; the dependency *source* (git vs local path)
//! is irrelevant to whether the code compiles — and the git rewrite itself is
//! covered by the unit tests in `commands::new`.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

/// Absolute path to this repository's `crates/` directory.
fn crates_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/forge-cli
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // <repo>/crates
        .unwrap()
        .to_path_buf()
}

/// Rewrite `rf-x = { git = "...RustForge" }` lines to local path deps, and
/// append an empty `[workspace]` table so the generated project does not get
/// absorbed into this repo's workspace during the build.
fn localize_manifest(manifest: &str, crates: &Path) -> String {
    let git = r#"git = "https://github.com/Chregu12/RustForge""#;
    let mut out = String::with_capacity(manifest.len());
    for line in manifest.lines() {
        if let Some(idx) = line.find(" = {") {
            let key = line[..idx].trim();
            if line.contains(git) && key.starts_with("rf-") {
                let local = crates.join(key);
                out.push_str(&format!(
                    "{key} = {{ path = {:?} }}\n",
                    local.to_string_lossy()
                ));
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("\n[workspace]\n");
    out
}

#[test]
#[ignore = "compiles the framework; run with --ignored"]
fn generated_project_compiles() {
    let crates = crates_dir();
    let tmp = tempfile::tempdir().expect("temp dir");
    let project = "e2e_app";

    // 1. Generate a standalone project exactly like an installed user would.
    Command::cargo_bin("forge")
        .expect("forge binary")
        .args(["new", project])
        .current_dir(tmp.path())
        .assert()
        .success();

    let project_dir = tmp.path().join(project);
    let manifest_path = project_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).expect("read manifest");

    // Sanity: the generated manifest really used git deps (the user-facing form).
    assert!(
        manifest.contains("github.com/Chregu12/RustForge"),
        "standalone project should ship git deps"
    );

    // 2. Repoint deps at the local crates and detach from this workspace.
    fs::write(&manifest_path, localize_manifest(&manifest, &crates)).expect("write manifest");

    // 3. Compile it offline, reusing this repo's target dir so the already-built
    //    framework rlibs are picked up instead of rebuilt from scratch.
    let target_dir = crates.parent().unwrap().join("target");
    Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(&project_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("CARGO_INCREMENTAL", "0")
        .assert()
        .success();
}
