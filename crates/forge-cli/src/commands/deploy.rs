//! `forge deploy generate` — generate deployment artifact files.
//!
//! This command is a thin wrapper around the `rf-deploy` library crate.
//! It writes `docker-compose.yml`, `Dockerfile`, and optionally Kubernetes
//! manifests into the current project directory.
//!
//! ## Health-check paths
//!
//! The generated Kubernetes manifests assume `GET /health/live` (liveness) and
//! `GET /health/ready` (readiness) are served by your application (e.g. via the
//! `rf-health` crate).  Pass `--liveness-path` / `--readiness-path` if your app
//! uses different routes.

use anyhow::Result;
use colored::Colorize;
use rf_deploy::{DockerComposeBuilder, DockerfileBuilder, KubernetesBuilder};
use std::fs;

/// Generate deployment configuration files for the current project.
pub async fn generate(
    app_name: &str,
    port: u16,
    with_postgres: Option<&str>,
    with_redis: bool,
    kubernetes: bool,
    liveness_path: &str,
    readiness_path: &str,
    image: Option<&str>,
) -> Result<()> {
    println!("{}", "Generating deployment artifacts…".bold());

    // --- Dockerfile -------------------------------------------------------
    let dockerfile = DockerfileBuilder::new()
        .rust_version("1.82")
        .port(port)
        .build()?;

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
