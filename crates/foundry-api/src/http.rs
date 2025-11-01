use crate::error::HttpError;
use crate::invocation::{CommandInvoker, FoundryInvoker};
use crate::upload;
use crate::AppJson;
use crate::InvocationRequest;
use anyhow::Result;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use foundry_application::ApplicationError;
use foundry_domain::CommandDescriptor;
use foundry_plugins::{
    AppError, CommandResult, CommandStatus, ValidationPort, ValidationReport, ValidationRules,
};
use foundry_storage::service::FileService;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::pending;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

const DEFAULT_MAX_BODY_SIZE: usize = 50 * 1024 * 1024;

pub type AppRouter = Router<AppState>;

#[derive(Clone)]
pub struct AppState {
    invoker: FoundryInvoker,
    file_service: Arc<FileService>,
    validation: Arc<dyn ValidationPort>,
}

impl AppState {
    pub fn new(invoker: FoundryInvoker) -> Self {
        let storage_manager = invoker.app().storage_manager();
        let file_service = Arc::new(FileService::new(storage_manager));
        let validation = invoker.app().validation();

        Self {
            invoker,
            file_service,
            validation,
        }
    }

    pub fn invoker(&self) -> &FoundryInvoker {
        &self.invoker
    }

    pub fn file_service(&self) -> Arc<FileService> {
        self.file_service.clone()
    }

    pub fn validation(&self) -> Arc<dyn ValidationPort> {
        self.validation.clone()
    }

    pub async fn validate(
        &self,
        payload: Value,
        rules: ValidationRules,
    ) -> Result<ValidationReport, HttpError> {
        self.validation
            .validate(payload, rules)
            .await
            .map_err(|err| {
                let error = AppError::new("VALIDATION_SERVICE_ERROR", err.to_string())
                    .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16());
                HttpError::from(error)
            })
    }

    pub async fn ensure_valid(
        &self,
        payload: Value,
        rules: ValidationRules,
    ) -> Result<ValidationReport, HttpError> {
        let report = self.validate(payload, rules).await?;
        if report.valid {
            return Ok(report);
        }

        let mut violations = report.errors.iter();
        let mut error = violations
            .next()
            .map(|violation| {
                let message = match &violation.code {
                    Some(code) => format!("{} ({code})", violation.message),
                    None => violation.message.clone(),
                };
                AppError::validation(&violation.field, message)
                    .with_status(StatusCode::UNPROCESSABLE_ENTITY.as_u16())
            })
            .unwrap_or_else(|| {
                AppError::validation("request", "Validation failed")
                    .with_status(StatusCode::UNPROCESSABLE_ENTITY.as_u16())
            });

        for violation in violations {
            let message = match &violation.code {
                Some(code) => format!("{} ({code})", violation.message),
                None => violation.message.clone(),
            };
            error = error.with_context(violation.field.clone(), message);
        }

        Err(HttpError::from(error))
    }
}

#[derive(Clone)]
pub struct HttpServer {
    state: AppState,
    router: AppRouter,
    max_body_size: usize,
}

impl HttpServer {
    pub fn new(invoker: FoundryInvoker) -> Self {
        let state = AppState::new(invoker);
        Self {
            router: Self::core_routes(),
            state,
            max_body_size: DEFAULT_MAX_BODY_SIZE,
        }
    }

    fn core_routes() -> AppRouter {
        Router::new()
            .route("/health", get(health))
            .route("/commands", get(commands))
            .route("/invoke", post(invoke))
            .route("/upload", post(upload::upload_file))
    }

    pub fn into_router(self) -> Router {
        self.router
            .layer(DefaultBodyLimit::max(self.max_body_size))
            .with_state(self.state)
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn cloned_state(&self) -> AppState {
        self.state.clone()
    }

    pub fn with_max_body_size(mut self, limit: usize) -> Self {
        self.max_body_size = limit;
        self
    }

    pub fn map_router<F>(mut self, builder: F) -> Self
    where
        F: FnOnce(AppRouter) -> AppRouter,
    {
        self.router = builder(self.router);
        self
    }

    pub fn merge_router(self, routes: AppRouter) -> Self {
        self.map_router(|router| router.merge(routes))
    }

    pub fn with_middleware<F, Fut, R>(self, middleware_fn: F) -> Self
    where
        F: Clone + Send + Sync + 'static,
        F: Fn(axum::http::Request<Body>, middleware::Next) -> Fut,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: axum::response::IntoResponse + 'static,
    {
        self.map_router(|router| router.layer(middleware::from_fn(middleware_fn)))
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let router = self
            .router
            .clone()
            .layer(DefaultBodyLimit::max(self.max_body_size))
            .with_state(self.state.clone());

        info!(%addr, "HTTP server gestartet");

        let listener = TcpListener::bind(addr).await?;
        let server = axum::serve(listener, router.into_make_service());

        tokio::select! {
            result = server => {
                if let Err(err) = result {
                    error!(error = %err, "HTTP server beendet mit Fehler");
                    return Err(err.into());
                }
            }
            _ = ctrl_c_or_pending() => {
                info!("Shutdown-Signal empfangen – HTTP server stoppt");
            }
        }

        Ok(())
    }
}

pub fn app_router() -> AppRouter {
    Router::new()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpEnvelope {
    pub status: CommandStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

impl From<CommandResult> for HttpEnvelope {
    fn from(result: CommandResult) -> Self {
        Self {
            status: result.status,
            message: result.message,
            data: result.data,
            error: result.error,
        }
    }
}

impl From<&CommandResult> for HttpEnvelope {
    fn from(result: &CommandResult) -> Self {
        Self {
            status: result.status.clone(),
            message: result.message.clone(),
            data: result.data.clone(),
            error: result.error.clone(),
        }
    }
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn commands(State(state): State<AppState>) -> Json<Vec<CommandDescriptor>> {
    Json(state.invoker.descriptors())
}

async fn invoke(
    State(state): State<AppState>,
    payload: AppJson<InvocationRequest>,
) -> Result<(StatusCode, Json<HttpEnvelope>), HttpError> {
    let rules = ValidationRules {
        rules: serde_json::json!({
            "required": ["command"],
            "fields": {
                "command": { "min_length": 1 }
            }
        }),
    };
    payload.validate(&state, rules).await?;
    let request = payload.into_inner();

    match state.invoker.invoke(request).await {
        Ok(result) => {
            let status = if let Some(error) = &result.error {
                error.status_code()
            } else {
                match result.status {
                    CommandStatus::Success | CommandStatus::Skipped => StatusCode::OK,
                    CommandStatus::Failure => StatusCode::UNPROCESSABLE_ENTITY,
                }
            };
            Ok((status, Json(HttpEnvelope::from(result))))
        }
        Err(err) => {
            let error = map_application_error(err);
            Err(HttpError::from(error))
        }
    }
}

fn map_application_error(err: ApplicationError) -> AppError {
    match err {
        ApplicationError::CommandNotFound(command) => AppError::new(
            "COMMAND_NOT_FOUND",
            format!("Command `{command}` wurde nicht gefunden"),
        )
        .with_status(StatusCode::NOT_FOUND.as_u16()),
        ApplicationError::CommandAlreadyRegistered(command) => AppError::new(
            "COMMAND_ALREADY_REGISTERED",
            format!("Command `{command}` ist bereits registriert"),
        )
        .with_status(StatusCode::CONFLICT.as_u16()),
        ApplicationError::CommandExecution(inner) => AppError::new(
            "COMMAND_EXECUTION_ERROR",
            format!("Command Execution Error: {inner}"),
        )
        .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
        ApplicationError::StorageError(message) => {
            AppError::new("STORAGE_ERROR", format!("Storage Error: {message}"))
                .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
        }
    }
}

async fn ctrl_c_or_pending() {
    if tokio::signal::ctrl_c().await.is_err() {
        pending::<()>().await;
    }
}
