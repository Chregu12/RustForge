//! `forge deploy generate` — generate deployment artifact files.
//!
//! This command is a thin wrapper around the `rf-deploy` library crate.
//! It writes `docker-compose.yml`, `Dockerfile`, and optionally Kubernetes
//! manifests into the current project directory.
//!
//! ## Health-check paths
//!
//! The Forge starter probes `/up` and `/ready`. Other applications default to
//! `/health/live` and `/health/ready`. Both paths are configurable.

use anyhow::{ensure, Context, Result};
use colored::Colorize;
use rf_deploy::{DockerComposeBuilder, DockerfileBuilder, KubernetesBuilder};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

const DOCKER_IGNORE: &str = ".git\ntarget\n.env\n.env.*\ndatabase/*.sqlite\ndatabase/*.sqlite-*\n";

fn standalone_binary() -> Result<Option<String>> {
    if !Path::new("src/main.rs").is_file() {
        return Ok(None);
    }
    let manifest: toml::Value = fs::read_to_string("Cargo.toml")?
        .parse()
        .context("Read application Cargo.toml")?;
    let package = manifest.get("package");
    Ok(package
        .and_then(|package| package.get("default-run").or_else(|| package.get("name")))
        .and_then(toml::Value::as_str)
        .map(str::to_owned))
}

/// Generate deployment configuration files for the current project.
pub async fn generate(
    app_name: &str,
    port: u16,
    with_postgres: Option<&str>,
    with_redis: bool,
    kubernetes: bool,
    liveness_path: Option<&str>,
    readiness_path: Option<&str>,
    image: Option<&str>,
) -> Result<()> {
    ensure!(port != 0, "Deployment port must be between 1 and 65535");
    println!("{}", "Generating deployment artifacts…".bold());

    // --- Dockerfile -------------------------------------------------------
    let mut builder = DockerfileBuilder::new().rust_version("1.96.0").port(port);
    if let Some(binary) = standalone_binary()? {
        builder = builder.binary_name(binary);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(".dockerignore")
        {
            Ok(mut file) => file.write_all(DOCKER_IGNORE.as_bytes())?,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error).context("Create .dockerignore"),
        }
    }
    let dockerfile = builder.build()?;

    fs::write("Dockerfile", &dockerfile)?;
    println!("  {} Dockerfile", "created".green());

    // --- docker-compose.yml -----------------------------------------------
    let mut compose_builder = DockerComposeBuilder::new()
        .app_name(app_name)
        .app_service(app_name, port);

    if let Some(pg_version) = with_postgres {
        compose_builder = compose_builder.postgres_service(pg_version);
    }
    if with_redis {
        compose_builder = compose_builder.redis_service();
    }

    let compose_yaml = compose_builder.build()?;
    fs::write("docker-compose.yml", &compose_yaml)?;
    println!("  {} docker-compose.yml", "created".green());

    // --- Kubernetes manifests (optional) ----------------------------------
    if kubernetes {
        let starter =
            Path::new("bootstrap/app.rs").is_file() && Path::new("config/app.rs").is_file();
        let liveness_path = liveness_path.unwrap_or(if starter { "/up" } else { "/health/live" });
        let readiness_path =
            readiness_path.unwrap_or(if starter { "/ready" } else { "/health/ready" });
        let default_image = format!("{app_name}:latest");
        let img = image.unwrap_or(&default_image);
        let k8s = KubernetesBuilder::new(app_name, img)
            .port(port)
            .liveness_path(liveness_path)
            .readiness_path(readiness_path);

        let deployment_yaml = k8s.build_deployment()?;
        let service_yaml = k8s.build_service()?;

        fs::create_dir_all("k8s")?;
        fs::write("k8s/deployment.yaml", &deployment_yaml)?;
        fs::write("k8s/service.yaml", &service_yaml)?;
        println!("  {} k8s/deployment.yaml", "created".green());
        println!("  {} k8s/service.yaml", "created".green());

        if liveness_path == "/health/live" || readiness_path == "/health/ready" {
            println!(
                "\n  {} The generated K8s manifests assume your app serves:\n\
                 \n    GET {}   (liveness probe)\
                 \n    GET {}   (readiness probe)\
                 \n\n  Wire these routes in your app (e.g. via rf-health) or\
                 \n  re-run with --liveness-path / --readiness-path to customise.",
                "note:".yellow().bold(),
                liveness_path,
                readiness_path,
            );
        }
    }

    println!("\n{}", "Done.".green().bold());
    Ok(())
}
