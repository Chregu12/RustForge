//! End-to-end test: code emitted by every `forge make:*` generator actually
//! COMPILES inside a real project.
//!
//! This is the proof that the scaffolding templates use the framework's real,
//! canonical API (not imagined ones). It copies `rustforge-starter`, runs one
//! `forge make` per artifact type, wires the generated modules into the crate,
//! and `cargo check`s the result.
//!
//! It is `#[ignore]`d by default because it compiles the framework crates the
//! starter depends on, which is slow and disk-hungry. Run it explicitly:
//!
//! ```bash
//! cargo test -p forge-cli --test make_compiles -- --ignored
//! ```
//!
//! Like `e2e_generated_project.rs`, it stays offline by repointing the
//! starter's `../crates/*` path deps at absolute paths in this checkout and
//! detaching the project from the parent workspace, then reusing this repo's
//! prebuilt `target/` so the framework rlibs are not rebuilt from scratch.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

/// `<repo>/crates`.
fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Run `forge make <args...>` in `dir` and assert success.
fn run_make(dir: &Path, args: &[&str]) {
    let mut cmd = Command::cargo_bin("forge").expect("forge binary");
    cmd.arg("make").args(args).current_dir(dir);
    cmd.assert().success();
}

#[test]
#[ignore = "compiles the framework; run with --ignored"]
fn generated_make_artifacts_compile() {
    let crates = crates_dir();
    let repo = crates.parent().unwrap();
    let starter = repo.join("rustforge-starter");

    let tmp = tempfile::tempdir().expect("temp dir");
    let project_dir = tmp.path().join("make_app");

    // 1. Copy the starter into a scratch project.
    copy_dir(&starter, &project_dir).expect("copy starter");

    // 2. Repoint `../crates/*` path deps at absolute checkout paths and detach
    //    from this repo's workspace so the project builds standalone.
    let manifest_path = project_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest = manifest.replace(
        "path = \"../crates/",
        &format!("path = \"{}/", crates.to_string_lossy()),
    );
    let manifest = if manifest.contains("[workspace]") {
        manifest
    } else {
        format!("{manifest}\n[workspace]\n")
    };
    fs::write(&manifest_path, manifest).expect("write manifest");

    // 3. Generate one artifact of every type. The events/listeners are linked
    //    (`--event PostCreated`) so the listener's typed `EventListenerFor`
    //    impl resolves against a real event.
    run_make(&project_dir, &["model", "Post"]);
    run_make(&project_dir, &["controller", "PostController"]);
    run_make(&project_dir, &["controller", "ApiPostController", "--api"]);
    run_make(&project_dir, &["migration", "create_posts_table"]);
    run_make(&project_dir, &["request", "StorePostRequest"]);
    run_make(&project_dir, &["policy", "PostPolicy"]);
    run_make(&project_dir, &["event", "PostCreated"]);
    run_make(
        &project_dir,
        &["listener", "SendWelcome", "--event", "PostCreated"],
    );
    run_make(&project_dir, &["job", "SendEmail"]);
    run_make(&project_dir, &["mail", "WelcomeMail"]);
    run_make(&project_dir, &["notification", "InvoicePaid"]);
    run_make(&project_dir, &["resource", "PostResource", "--collection"]);
    run_make(&project_dir, &["middleware", "Authenticate"]);
    run_make(&project_dir, &["command", "SyncData"]);
    run_make(&project_dir, &["factory", "PostFactory"]);
    run_make(&project_dir, &["seeder", "PostSeeder"]);

    // 4. Wire the generated modules so `cargo check` actually type-checks them.
    wire_modules(&project_dir);

    // 5. Compile offline, reusing this repo's target dir for the prebuilt
    //    framework rlibs.
    let target_dir = repo.join("target");
    Command::new("cargo")
        .args(["check", "--offline", "--bins", "--test", "factory_probe"])
        .current_dir(&project_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("CARGO_INCREMENTAL", "0")
        .assert()
        .success();
}

/// Attach every generated `src/*/mod.rs` (plus the out-of-`src` seeders) to the
/// binary crate, build a `migrations/mod.rs` that maps the timestamp-prefixed
/// files to valid module names, and add a small integration test that pulls in
/// the (dev-dependency-using) factory.
fn wire_modules(project: &Path) {
    let src = project.join("src");

    // migrations: timestamped filenames aren't valid module names, so map them
    // via `#[path]`.
    let mig_dir = src.join("migrations");
    let mut mig_mod = String::new();
    let mut migs: Vec<_> = fs::read_dir(&mig_dir)
        .expect("migrations dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".rs") && n != "mod.rs")
        .collect();
    migs.sort();
    for (i, file) in migs.iter().enumerate() {
        let stem = file.trim_end_matches(".rs");
        mig_mod.push_str(&format!("#[path = \"{stem}.rs\"]\npub mod migration_{i};\n"));
    }
    fs::write(mig_dir.join("mod.rs"), mig_mod).expect("write migrations/mod.rs");

    // `forge make command` does not register a mod.rs; create one.
    fs::write(src.join("commands").join("mod.rs"), "pub mod sync_data;\n")
        .expect("write commands/mod.rs");

    // Attach all generated module trees to main.rs (the starter's main.rs uses
    // `#[path]` for app/ and routes/, so we follow the same convention).
    let mut decls = String::from("\n");
    for (dir, name) in [
        ("models", "models"),
        ("controllers", "controllers"),
        ("migrations", "migrations"),
        ("requests", "requests"),
        ("policies", "policies"),
        ("events", "events"),
        ("listeners", "listeners"),
        ("jobs", "jobs"),
        ("mail", "mail"),
        ("notifications", "notifications"),
        ("resources", "resources"),
        ("middleware", "middleware"),
        ("commands", "commands"),
    ] {
        decls.push_str(&format!("#[path = \"{dir}/mod.rs\"]\npub mod {name};\n"));
    }
    // Seeders live under database/seeders/ (outside src/).
    decls.push_str("#[path = \"../database/seeders/mod.rs\"]\npub mod seeders;\n");

    let main_path = src.join("main.rs");
    let main = fs::read_to_string(&main_path).expect("read main.rs");
    // Insert after the existing `mod routes;` declaration.
    let marker = "mod routes;";
    let idx = main.find(marker).expect("main.rs has `mod routes;`") + marker.len();
    let main = format!("{}{}{}", &main[..idx], decls, &main[idx..]);
    fs::write(&main_path, main).expect("write main.rs");

    // Factories use the `rf-testing` dev-dependency, so they only compile in a
    // test target. Pull the generated factory into a dedicated integration test.
    fs::write(
        project.join("tests").join("factory_probe.rs"),
        "#[path = \"factories/post_factory.rs\"]\nmod post_factory;\n",
    )
    .expect("write tests/factory_probe.rs");
}

/// Recursively copy a directory tree.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst = to.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dst)?;
        } else {
            fs::copy(entry.path(), &dst)?;
        }
    }
    Ok(())
}
