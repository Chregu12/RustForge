use anyhow::{ensure, Context, Result};
use std::{fs, path::Path, process::Command};

const STARTER: &[(&str, &str)] = &[
    ("src/main.rs", include_str!("../../starter/src/main.rs")),
    ("bootstrap/app.rs", include_str!("../../starter/bootstrap/app.rs")),
    ("config/app.rs", include_str!("../../starter/config/app.rs")),
    ("routes/mod.rs", include_str!("../../starter/routes/mod.rs")),
    ("routes/web.rs", include_str!("../../starter/routes/web.rs")),
    ("routes/api.rs", include_str!("../../starter/routes/api.rs")),
    ("app/mod.rs", include_str!("../../starter/app/mod.rs")),
    ("app/http/mod.rs", include_str!("../../starter/app/http/mod.rs")),
    ("app/http/controllers/mod.rs", include_str!("../../starter/app/http/controllers/mod.rs")),
    ("app/http/controllers/post_controller.rs", include_str!("../../starter/app/http/controllers/post_controller.rs")),
    ("app/http/requests/mod.rs", include_str!("../../starter/app/http/requests/mod.rs")),
    ("app/http/requests/post_request.rs", include_str!("../../starter/app/http/requests/post_request.rs")),
    ("app/models/mod.rs", include_str!("../../starter/app/models/mod.rs")),
    ("app/models/post.rs", include_str!("../../starter/app/models/post.rs")),
    ("database/migrations/20260101000000_create_posts_table/up.sql", include_str!("../../starter/database/migrations/20260101000000_create_posts_table/up.sql")),
    ("database/migrations/20260101000000_create_posts_table/down.sql", include_str!("../../starter/database/migrations/20260101000000_create_posts_table/down.sql")),
    ("database/seeders/posts.sql", include_str!("../../starter/database/seeders/posts.sql")),
    ("resources/views/welcome.html", include_str!("../../starter/resources/views/welcome.html")),
    (".env.example", "APP_ENV=local\nHOST=127.0.0.1\nPORT=8000\nDATABASE_URL=sqlite://database/database.sqlite?mode=rwc\n"),
    (".gitignore", "/target\n.env\n/database/*.sqlite\n/database/*.sqlite-*\n"),
];

pub async fn run(name: &str) -> Result<()> {
    run_with_framework(name, None).await
}

pub async fn run_with_framework(name: &str, framework: Option<&Path>) -> Result<()> {
    ensure!(
        name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_'),
        "Project names must start with a lowercase letter and contain only lowercase letters, digits, hyphens and underscores"
    );

    let dependency = match framework {
        Some(root) => {
            let path = root
                .join("crates/rf")
                .canonicalize()
                .context("Locate crates/rf in --framework-path")?;
            ensure!(
                path.join("Cargo.toml").is_file(),
                "Missing RustForge crate at {}",
                path.display()
            );
            let path = path.to_str().context("RustForge path must be UTF-8")?;
            format!("{{ path = {} }}", toml::Value::String(path.to_owned()))
        }
        None => format!(
            "{{ git = \"https://github.com/Chregu12/RustForge\", rev = \"{}\" }}",
            include_str!("../../starter/framework-revision.txt").trim()
        ),
    };

    let root = Path::new(name);
    fs::create_dir(root).with_context(|| format!("Create project directory {name}"))?;
    fs::write(root.join("Cargo.toml"), format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.96.0\"\n\n[workspace]\nresolver = \"3\"\n\n[dependencies]\nrf = {dependency}\nanyhow = \"1\"\naxum = \"0.8\"\ndotenvy = \"0.15\"\nserde = {{ version = \"1\", features = [\"derive\"] }}\ntokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\", \"net\", \"signal\"] }}\n"
    ))?;
    for (relative, content) in STARTER {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    }
    fs::write(root.join("README.md"), format!(
        "# {name}\n\nRun `forge migrate` and then `forge serve`. Open http://127.0.0.1:8000.\n\nThe posts resource is under `/api/posts`. Use `forge route:list` to inspect routes.\nCopy `.env.example` to `.env` to change the listener or database.\n"
    ))?;

    if !Command::new("git")
        .args(["init", "--quiet", name])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("Git initialization failed. Project files are available in {name}.");
    }

    println!("Created {name}\n  cd {name}\n  forge migrate\n  forge serve");
    Ok(())
}
