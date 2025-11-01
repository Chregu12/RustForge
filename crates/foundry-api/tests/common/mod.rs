use std::sync::Arc;

use axum::Router;
use foundry_api::http::HttpServer;
use foundry_api::invocation::FoundryInvoker;
use foundry_application::FoundryApp;
use foundry_infra::{LocalArtifactPort, SeaOrmMigrationService, SeaOrmSeedService};
use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static STORAGE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct HttpTestApp {
    router: Router,
    #[allow(dead_code)]
    storage_root: std::path::PathBuf,
    original_storage_config: Option<String>,
    _temp: TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl HttpTestApp {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_server(|server| server)
    }

    pub fn with_server<F>(builder: F) -> Self
    where
        F: FnOnce(HttpServer) -> HttpServer,
    {
        let guard = STORAGE_LOCK
            .lock()
            .expect("failed to acquire storage configuration lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_root = temp.path().join("storage");
        let public_root = storage_root.join("public");
        let local_root = storage_root.join("local");
        std::fs::create_dir_all(&public_root).expect("create public storage");
        std::fs::create_dir_all(&local_root).expect("create local storage");

        let public_root_str = public_root.to_string_lossy().to_string();
        let local_root_str = local_root.to_string_lossy().to_string();

        let storage_config = serde_json::json!({
            "default": "public",
            "disks": {
                "public": {
                    "driver": "local",
                    "root": public_root_str,
                    "url": "http://localhost/storage",
                    "visibility": "public"
                },
                "local": {
                    "driver": "local",
                    "root": local_root_str,
                    "url": "http://localhost/storage",
                    "visibility": "private"
                }
            }
        });

        let original_storage_config = std::env::var("STORAGE_CONFIG").ok();
        std::env::set_var("STORAGE_CONFIG", storage_config.to_string());

        let app = FoundryApp::bootstrap(
            serde_json::json!({}),
            Arc::new(LocalArtifactPort::default()),
            Arc::new(SeaOrmMigrationService::default()),
            Arc::new(SeaOrmSeedService::default()),
        )
        .expect("bootstrap application");

        let invoker = FoundryInvoker::new(app);
        let server = HttpServer::new(invoker);
        let router = builder(server).into_router();

        Self {
            router,
            storage_root,
            original_storage_config,
            _temp: temp,
            _guard: guard,
        }
    }

    pub fn router(&self) -> Router {
        self.router.clone()
    }

    #[allow(dead_code)]
    pub fn public_storage_path(&self) -> std::path::PathBuf {
        std::env::var("STORAGE_CONFIG")
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| {
                value["disks"]["public"]["root"]
                    .as_str()
                    .map(std::path::PathBuf::from)
            })
            .unwrap_or_else(|| self.storage_root.join("public"))
    }
}

impl Drop for HttpTestApp {
    fn drop(&mut self) {
        if let Some(value) = &self.original_storage_config {
            std::env::set_var("STORAGE_CONFIG", value);
        } else {
            std::env::remove_var("STORAGE_CONFIG");
        }
    }
}
