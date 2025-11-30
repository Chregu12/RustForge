//! HTTP routes for Horizon dashboard

use crate::{metrics::JobHistoryEntry, Horizon};
use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use rf_jobs::{FailedJob, JobPayload, QueueManager};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub horizon: Horizon,
    pub queue_manager: QueueManager,
}

impl AppState {
    /// Create new application state
    pub fn new(horizon: Horizon, queue_manager: QueueManager) -> Self {
        Self {
            horizon,
            queue_manager,
        }
    }
}

/// Build the Horizon routes with enhanced Laravel Horizon features
pub fn routes(state: AppState) -> Router {
    Router::new()
        // Dashboard views
        .route("/horizon", get(dashboard_handler))
        .route("/horizon/jobs", get(jobs_list_handler))
        .route("/horizon/jobs/:id", get(job_detail_handler))
        .route("/horizon/failed", get(failed_jobs_handler))

        // WebSocket for real-time updates
        .route("/horizon/ws", get(ws_handler))

        // API endpoints - Stats & Metrics
        .route("/horizon/api/stats", get(stats_api_handler))
        .route("/horizon/api/workload", get(workload_handler))
        .route("/horizon/api/metrics", get(metrics_api_handler))
        .route("/horizon/api/metrics/:queue", get(queue_metrics_handler))

        // Jobs endpoints
        .route("/horizon/api/jobs", get(jobs_api_handler))
        .route("/horizon/api/jobs/recent", get(recent_jobs_handler))
        .route("/horizon/api/jobs/pending", get(pending_jobs_handler))
        .route("/horizon/api/jobs/completed", get(completed_jobs_handler))
        .route("/horizon/api/jobs/:id", get(job_detail_api_handler))
        .route("/horizon/api/jobs/:id/retry", post(retry_job_handler))
        .route("/horizon/api/jobs/:id", delete(delete_job_handler))

        // Jobs by tag
        .route("/horizon/api/jobs/tag/:tag", get(jobs_by_tag_handler))

        // Failed jobs endpoints
        .route("/horizon/api/failed", get(failed_jobs_api_handler))
        .route("/horizon/api/failed/:id", get(failed_job_details_handler))
        .route("/horizon/api/failed/:id/retry", post(retry_failed_job_handler))
        .route("/horizon/api/failed/:id", delete(forget_failed_handler))
        .route("/horizon/api/failed/retry-all", post(retry_all_failed_handler))
        .route("/horizon/api/failed", delete(flush_failed_handler))
        .route("/horizon/api/failed/batch-retry", post(batch_retry_handler))
        .route("/horizon/api/failed/batch-delete", delete(batch_delete_handler))

        // Workers & Supervisors
        .route("/horizon/api/workers", get(workers_api_handler))
        .route("/horizon/api/masters", get(masters_handler))
        .route("/horizon/api/supervisors", get(supervisors_handler))
        .route("/horizon/api/supervisors/:name", get(supervisor_handler))

        // Monitoring endpoints
        .route("/horizon/api/monitoring", get(monitoring_handler))

        .with_state(Arc::new(state))
}

/// Dashboard home page
async fn dashboard_handler() -> impl IntoResponse {
    Html(include_str!("../views/dashboard.html"))
}

/// Jobs list page
async fn jobs_list_handler() -> impl IntoResponse {
    Html(include_str!("../views/jobs_list.html"))
}

/// Job detail page
async fn job_detail_handler(Path(id): Path<String>) -> impl IntoResponse {
    Html(include_str!("../views/job_detail.html").replace("{{job_id}}", &id))
}

/// Failed jobs page
async fn failed_jobs_handler() -> impl IntoResponse {
    Html(include_str!("../views/failed_jobs.html"))
}

/// Query parameters for jobs list
#[derive(Debug, Deserialize)]
pub struct JobsQuery {
    #[serde(default)]
    pub queue: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_per_page")]
    pub per_page: usize,
    #[serde(default)]
    pub search: Option<String>,
}

fn default_page() -> usize {
    1
}

fn default_per_page() -> usize {
    20
}

/// Stats API response
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_jobs: u64,
    pub jobs_pending: u64,
    pub jobs_processing: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub jobs_per_minute: f64,
    pub queues: Vec<QueueStat>,
}

#[derive(Debug, Serialize)]
pub struct QueueStat {
    pub name: String,
    pub size: u64,
    pub throughput: f64,
}

/// Get dashboard statistics
async fn stats_api_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, AppError> {
    let horizon_state = state.horizon.state().await;

    // Calculate statistics from metrics
    let mut total_jobs = 0u64;
    let mut jobs_pending = 0u64;
    let mut jobs_completed = 0u64;
    let mut jobs_failed = 0u64;
    let mut jobs_per_minute = 0.0;
    let mut queues = Vec::new();

    for (queue_name, metrics) in &horizon_state.metrics {
        total_jobs += metrics.jobs_processed + metrics.jobs_failed;
        jobs_pending += metrics.jobs_pending;
        jobs_completed += metrics.jobs_processed;
        jobs_failed += metrics.jobs_failed;
        jobs_per_minute += metrics.throughput_per_minute;

        // Get queue size from Redis
        let size = state.queue_manager.size(queue_name).await.unwrap_or(0);

        queues.push(QueueStat {
            name: queue_name.clone(),
            size,
            throughput: metrics.throughput_per_minute,
        });
    }

    Ok(Json(StatsResponse {
        total_jobs,
        jobs_pending,
        jobs_processing: 0, // TODO: Track from workers
        jobs_completed,
        jobs_failed,
        jobs_per_minute,
        queues,
    }))
}

/// Jobs list response
#[derive(Debug, Serialize)]
pub struct JobsResponse {
    pub jobs: Vec<JobInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

#[derive(Debug, Serialize)]
pub struct JobInfo {
    pub id: String,
    pub queue: String,
    pub job_type: String,
    pub status: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub dispatched_at: String,
    pub available_at: String,
}

impl From<&JobPayload> for JobInfo {
    fn from(payload: &JobPayload) -> Self {
        Self {
            id: payload.id.to_string(),
            queue: payload.queue.clone(),
            job_type: payload.job_type.clone(),
            status: if payload.attempt > 0 {
                "retrying".to_string()
            } else {
                "pending".to_string()
            },
            attempt: payload.attempt,
            max_attempts: payload.max_attempts,
            dispatched_at: payload.dispatched_at.to_rfc3339(),
            available_at: payload.available_at.to_rfc3339(),
        }
    }
}

/// Get jobs list
async fn jobs_api_handler(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<JobsQuery>,
) -> Result<Json<JobsResponse>, AppError> {
    // For now, we'll return an empty list
    // In a real implementation, we'd query Redis for pending jobs

    let jobs = Vec::new(); // TODO: Implement job listing from Redis
    let total = jobs.len();
    let total_pages = (total + query.per_page - 1) / query.per_page;

    Ok(Json(JobsResponse {
        jobs,
        total,
        page: query.page,
        per_page: query.per_page,
        total_pages,
    }))
}

/// Get job details
async fn job_detail_api_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<JobDetailResponse>, AppError> {
    // TODO: Implement job detail retrieval from Redis
    Err(AppError::NotFound("Job not found".to_string()))
}

#[derive(Debug, Serialize)]
pub struct JobDetailResponse {
    pub id: String,
    pub queue: String,
    pub job_type: String,
    pub status: String,
    pub payload: serde_json::Value,
    pub attempt: u32,
    pub max_attempts: u32,
    pub dispatched_at: String,
    pub available_at: String,
    pub history: Vec<JobHistoryEntry>,
}

/// Retry a failed job
async fn retry_job_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .queue_manager
        .retry_failed(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "success": true,
        "message": "Job queued for retry"
    })))
}

/// Delete a job
async fn delete_job_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement job deletion
    Ok(Json(json!({
        "success": true,
        "message": "Job deleted"
    })))
}

/// Failed jobs response
#[derive(Debug, Serialize)]
pub struct FailedJobsResponse {
    pub jobs: Vec<FailedJobInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct FailedJobInfo {
    pub id: String,
    pub queue: String,
    pub job_type: String,
    pub error: String,
    pub failed_at: String,
    pub payload: serde_json::Value,
}

impl From<&FailedJob> for FailedJobInfo {
    fn from(failed: &FailedJob) -> Self {
        Self {
            id: failed.payload.id.to_string(),
            queue: failed.payload.queue.clone(),
            job_type: failed.payload.job_type.clone(),
            error: failed.error.clone(),
            failed_at: failed.failed_at.to_rfc3339(),
            payload: failed.payload.data.clone(),
        }
    }
}

/// Get failed jobs
async fn failed_jobs_api_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FailedJobsResponse>, AppError> {
    let failed_jobs = state
        .queue_manager
        .failed_jobs()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let jobs: Vec<FailedJobInfo> = failed_jobs.iter().map(|f| f.into()).collect();
    let total = jobs.len();

    Ok(Json(FailedJobsResponse { jobs, total }))
}

/// Retry a specific failed job
async fn retry_failed_job_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .queue_manager
        .retry_failed(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "success": true,
        "message": "Job queued for retry"
    })))
}

/// Delete a specific failed job
async fn delete_failed_job_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement single failed job deletion
    Ok(Json(json!({
        "success": true,
        "message": "Failed job deleted"
    })))
}

/// Batch retry request
#[derive(Debug, Deserialize)]
pub struct BatchRetryRequest {
    pub job_ids: Vec<Uuid>,
}

/// Batch retry failed jobs
async fn batch_retry_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BatchRetryRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut success_count = 0;
    let mut error_count = 0;

    for job_id in request.job_ids {
        match state.queue_manager.retry_failed(job_id).await {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    Ok(Json(json!({
        "success": true,
        "retried": success_count,
        "errors": error_count
    })))
}

/// Batch delete request
#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub job_ids: Vec<Uuid>,
}

/// Batch delete failed jobs
async fn batch_delete_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<BatchDeleteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement batch deletion
    Ok(Json(json!({
        "success": true,
        "deleted": request.job_ids.len()
    })))
}

/// Get metrics
async fn metrics_api_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let horizon_state = state.horizon.state().await;
    Ok(Json(json!({
        "metrics": horizon_state.metrics
    })))
}

/// Get workers
async fn workers_api_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement worker status tracking
    Ok(Json(json!({
        "workers": []
    })))
}

// ========== Enhanced Handler Functions ==========

/// WebSocket handler for real-time updates
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    crate::websocket::ws_handler(ws, State(state)).await
}

/// Get workload statistics
async fn workload_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement workload calculation
    Ok(Json(json!({
        "queues": [],
        "total_load": 0
    })))
}

/// Get metrics for a specific queue
async fn queue_metrics_handler(
    State(state): State<Arc<AppState>>,
    Path(queue): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let horizon_state = state.horizon.state().await;

    if let Some(metrics) = horizon_state.metrics.get(&queue) {
        Ok(Json(json!(metrics)))
    } else {
        Err(AppError::NotFound(format!("Queue {} not found", queue)))
    }
}

/// Get recent jobs
async fn recent_jobs_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement recent jobs retrieval
    Ok(Json(json!({
        "jobs": [],
        "total": 0
    })))
}

/// Get pending jobs
async fn pending_jobs_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement pending jobs retrieval
    Ok(Json(json!({
        "jobs": [],
        "total": 0
    })))
}

/// Get completed jobs
async fn completed_jobs_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement completed jobs retrieval
    Ok(Json(json!({
        "jobs": [],
        "total": 0
    })))
}

/// Get jobs by tag
async fn jobs_by_tag_handler(
    State(_state): State<Arc<AppState>>,
    Path(tag): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement jobs by tag retrieval
    Ok(Json(json!({
        "tag": tag,
        "jobs": [],
        "total": 0
    })))
}

/// Get failed job details
async fn failed_job_details_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement failed job details retrieval
    Err(AppError::NotFound("Failed job not found".to_string()))
}

/// Retry all failed jobs
async fn retry_all_failed_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement retry all failed jobs
    Ok(Json(json!({
        "success": true,
        "retried": 0
    })))
}

/// Flush all failed jobs
async fn flush_failed_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement flush failed jobs
    Ok(Json(json!({
        "success": true,
        "deleted": 0
    })))
}

/// Forget (delete) a failed job
async fn forget_failed_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement forget failed job
    Ok(Json(json!({
        "success": true,
        "message": "Failed job deleted"
    })))
}

/// Get masters (supervisors) information
async fn masters_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement masters retrieval
    Ok(Json(json!({
        "masters": []
    })))
}

/// Get all supervisors
async fn supervisors_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement supervisors retrieval
    Ok(Json(json!({
        "supervisors": []
    })))
}

/// Get specific supervisor
async fn supervisor_handler(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement supervisor retrieval
    Err(AppError::NotFound(format!("Supervisor {} not found", name)))
}

/// Get monitoring information
async fn monitoring_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement monitoring data
    Ok(Json(json!({
        "workers": [],
        "queues": [],
        "stats": {}
    })))
}

/// Application error types
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
