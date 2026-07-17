//! Web dashboard for Horizon monitoring

use crate::Horizon;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

/// Dashboard state
#[derive(Clone)]
pub struct Dashboard {
    horizon: Horizon,
}

impl Dashboard {
    /// Create a new dashboard
    pub fn new(horizon: Horizon) -> Self {
        Self { horizon }
    }

    /// Build the router
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/", get(index_handler))
            .route("/api/status", get(status_handler))
            .route("/api/batches", get(batches_handler))
            .route("/api/failed-jobs", get(failed_jobs_handler))
            .route("/api/metrics", get(metrics_handler))
            .with_state(state)
    }
}

/// Start the dashboard server
pub async fn serve(horizon: Horizon, addr: &str) -> anyhow::Result<()> {
    let dashboard = Dashboard::new(horizon);
    let app = dashboard.router();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Horizon dashboard running on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Index page handler
async fn index_handler() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

/// Status API handler
async fn status_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let state = dashboard.horizon.state().await;

    Json(json!({
        "status": "running",
        "total_batches": state.batches.len(),
        "failed_jobs": state.failed_jobs.len(),
        "monitored_queues": dashboard.horizon.config.monitored_queues,
    }))
}

/// Batches API handler
async fn batches_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let state = dashboard.horizon.state().await;
    Json(json!({
        "batches": state.batches,
    }))
}

/// Failed jobs API handler
async fn failed_jobs_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let state = dashboard.horizon.state().await;
    Json(json!({
        "failed_jobs": state.failed_jobs,
    }))
}

/// Metrics API handler
async fn metrics_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let state = dashboard.horizon.state().await;
    Json(json!({
        "metrics": state.metrics,
    }))
}

/// Dashboard HTML template
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Horizon - Queue Dashboard</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 2rem;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
        }

        .header {
            color: white;
            margin-bottom: 2rem;
        }

        .header h1 {
            font-size: 2.5rem;
            margin-bottom: 0.5rem;
        }

        .header p {
            opacity: 0.9;
            font-size: 1.1rem;
        }

        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }

        .stat-card {
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        .stat-card h3 {
            color: #667eea;
            font-size: 0.875rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin-bottom: 0.5rem;
        }

        .stat-card .value {
            font-size: 2rem;
            font-weight: bold;
            color: #2d3748;
        }

        .panel {
            background: white;
            border-radius: 12px;
            padding: 2rem;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            margin-bottom: 2rem;
        }

        .panel h2 {
            color: #2d3748;
            margin-bottom: 1.5rem;
            font-size: 1.5rem;
        }

        .batch-list, .job-list {
            list-style: none;
        }

        .batch-item, .job-item {
            padding: 1rem;
            border-bottom: 1px solid #e2e8f0;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .batch-item:last-child, .job-item:last-child {
            border-bottom: none;
        }

        .batch-name, .job-name {
            font-weight: 600;
            color: #2d3748;
        }

        .batch-progress {
            flex: 1;
            margin: 0 1rem;
        }

        .progress-bar {
            background: #e2e8f0;
            height: 8px;
            border-radius: 4px;
            overflow: hidden;
            margin-bottom: 0.25rem;
        }

        .progress-fill {
            background: linear-gradient(90deg, #667eea, #764ba2);
            height: 100%;
            transition: width 0.3s ease;
        }

        .progress-text {
            font-size: 0.75rem;
            color: #718096;
        }

        .status {
            padding: 0.25rem 0.75rem;
            border-radius: 12px;
            font-size: 0.75rem;
            font-weight: 600;
            text-transform: uppercase;
        }

        .status.completed {
            background: #c6f6d5;
            color: #22543d;
        }

        .status.processing {
            background: #bee3f8;
            color: #2c5282;
        }

        .status.failed {
            background: #fed7d7;
            color: #742a2a;
        }

        .status.pending {
            background: #feebc8;
            color: #7c2d12;
        }

        .empty-state {
            text-align: center;
            padding: 3rem;
            color: #718096;
        }

        .empty-state svg {
            width: 64px;
            height: 64px;
            margin-bottom: 1rem;
            opacity: 0.5;
        }

        .refresh-btn {
            background: #667eea;
            color: white;
            border: none;
            padding: 0.5rem 1rem;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.875rem;
            font-weight: 600;
            transition: background 0.2s;
        }

        .refresh-btn:hover {
            background: #5a67d8;
        }

        .metric-row {
            display: flex;
            justify-content: space-between;
            padding: 0.75rem 0;
            border-bottom: 1px solid #e2e8f0;
        }

        .metric-row:last-child {
            border-bottom: none;
        }

        .metric-label {
            color: #718096;
        }

        .metric-value {
            font-weight: 600;
            color: #2d3748;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 Horizon</h1>
            <p>Queue Dashboard & Monitoring</p>
        </div>

        <div class="stats-grid">
            <div class="stat-card">
                <h3>Total Batches</h3>
                <div class="value" id="total-batches">0</div>
            </div>
            <div class="stat-card">
                <h3>Failed Jobs</h3>
                <div class="value" id="failed-jobs">0</div>
            </div>
            <div class="stat-card">
                <h3>Monitored Queues</h3>
                <div class="value" id="monitored-queues">0</div>
            </div>
            <div class="stat-card">
                <h3>Status</h3>
                <div class="value" id="status">Running</div>
            </div>
        </div>

        <div class="panel">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem;">
                <h2>Active Batches</h2>
                <button class="refresh-btn" onclick="loadData()">Refresh</button>
            </div>
            <ul class="batch-list" id="batch-list">
                <li class="empty-state">
                    <div>No active batches</div>
                </li>
            </ul>
        </div>

        <div class="panel">
            <h2>Failed Jobs</h2>
            <ul class="job-list" id="job-list">
                <li class="empty-state">
                    <div>No failed jobs</div>
                </li>
            </ul>
        </div>

        <div class="panel">
            <h2>Queue Metrics</h2>
            <div id="metrics-container">
                <div class="empty-state">No metrics available</div>
            </div>
        </div>
    </div>

    <script>
        async function loadData() {
            try {
                // Load status
                const statusRes = await fetch('/api/status');
                const statusData = await statusRes.json();
                document.getElementById('total-batches').textContent = statusData.total_batches;
                document.getElementById('failed-jobs').textContent = statusData.failed_jobs;
                document.getElementById('monitored-queues').textContent = statusData.monitored_queues.length;

                // Load batches
                const batchesRes = await fetch('/api/batches');
                const batchesData = await batchesRes.json();
                renderBatches(batchesData.batches);

                // Load failed jobs
                const failedRes = await fetch('/api/failed-jobs');
                const failedData = await failedRes.json();
                renderFailedJobs(failedData.failed_jobs);

                // Load metrics
                const metricsRes = await fetch('/api/metrics');
                const metricsData = await metricsRes.json();
                renderMetrics(metricsData.metrics);
            } catch (error) {
                console.error('Error loading data:', error);
            }
        }

        function renderBatches(batches) {
            const list = document.getElementById('batch-list');

            if (Object.keys(batches).length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No active batches</div></li>';
                return;
            }

            list.innerHTML = Object.values(batches).map(batch => `
                <li class="batch-item">
                    <div class="batch-name">${batch.name}</div>
                    <div class="batch-progress">
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${(batch.total_jobs - batch.pending_jobs) / batch.total_jobs * 100}%"></div>
                        </div>
                        <div class="progress-text">${batch.total_jobs - batch.pending_jobs} / ${batch.total_jobs} jobs</div>
                    </div>
                    <span class="status ${batch.status.toLowerCase()}">${batch.status}</span>
                </li>
            `).join('');
        }

        function renderFailedJobs(jobs) {
            const list = document.getElementById('job-list');

            if (jobs.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No failed jobs</div></li>';
                return;
            }

            list.innerHTML = jobs.map(job => `
                <li class="job-item">
                    <div>
                        <div class="job-name">${job.job_name}</div>
                        <div style="font-size: 0.875rem; color: #718096; margin-top: 0.25rem;">
                            Queue: ${job.queue} • Failed: ${new Date(job.failed_at).toLocaleString()}
                        </div>
                    </div>
                    <span class="status failed">Failed</span>
                </li>
            `).join('');
        }

        function renderMetrics(metrics) {
            const container = document.getElementById('metrics-container');

            if (Object.keys(metrics).length === 0) {
                container.innerHTML = '<div class="empty-state">No metrics available</div>';
                return;
            }

            container.innerHTML = Object.entries(metrics).map(([queue, metric]) => `
                <div style="margin-bottom: 2rem;">
                    <h3 style="color: #667eea; margin-bottom: 1rem;">Queue: ${queue}</h3>
                    <div class="metric-row">
                        <span class="metric-label">Jobs Processed</span>
                        <span class="metric-value">${metric.jobs_processed}</span>
                    </div>
                    <div class="metric-row">
                        <span class="metric-label">Jobs Failed</span>
                        <span class="metric-value">${metric.jobs_failed}</span>
                    </div>
                    <div class="metric-row">
                        <span class="metric-label">Jobs Pending</span>
                        <span class="metric-value">${metric.jobs_pending}</span>
                    </div>
                    <div class="metric-row">
                        <span class="metric-label">Avg Processing Time</span>
                        <span class="metric-value">${metric.average_processing_time_ms.toFixed(2)}ms</span>
                    </div>
                    <div class="metric-row">
                        <span class="metric-label">Throughput</span>
                        <span class="metric-value">${metric.throughput_per_minute.toFixed(2)} jobs/min</span>
                    </div>
                </div>
            `).join('');
        }

        // Load data on page load
        loadData();

        // Auto-refresh every 5 seconds
        setInterval(loadData, 5000);
    </script>
</body>
</html>
"#;
