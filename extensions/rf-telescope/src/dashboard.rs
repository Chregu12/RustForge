//! Web dashboard for Telescope monitoring

use crate::{EntryType, Telescope};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

/// Dashboard state
#[derive(Clone)]
pub struct Dashboard {
    telescope: Telescope,
}

impl Dashboard {
    /// Create a new dashboard
    pub fn new(telescope: Telescope) -> Self {
        Self { telescope }
    }

    /// Build the router
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/", get(index_handler))
            .route("/api/stats", get(stats_handler))
            .route("/api/entries", get(entries_handler))
            .route("/api/requests", get(requests_handler))
            .route("/api/queries", get(queries_handler))
            .route("/api/exceptions", get(exceptions_handler))
            .route("/api/cache", get(cache_handler))
            .route("/api/jobs", get(jobs_handler))
            .route("/api/mail", get(mail_handler))
            .with_state(state)
    }
}

/// Query parameters for filtering entries
#[derive(Debug, Deserialize)]
struct EntryQuery {
    #[serde(default)]
    page: usize,
    #[serde(default = "default_per_page")]
    per_page: usize,
}

fn default_per_page() -> usize {
    20
}

/// Start the dashboard server
pub async fn serve(telescope: Telescope, addr: &str) -> anyhow::Result<()> {
    let dashboard = Dashboard::new(telescope);
    let app = dashboard.router();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Telescope dashboard running on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Index page handler
async fn index_handler() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

/// Stats API handler
async fn stats_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();

    let total_entries = storage.count(None).await;
    let requests = storage.count(Some(EntryType::Request)).await;
    let queries = storage.count(Some(EntryType::Query)).await;
    let exceptions = storage.count(Some(EntryType::Exception)).await;
    let cache = storage.count(Some(EntryType::Cache)).await;
    let jobs = storage.count(Some(EntryType::Job)).await;
    let mail = storage.count(Some(EntryType::Mail)).await;

    Json(json!({
        "total_entries": total_entries,
        "requests": requests,
        "queries": queries,
        "exceptions": exceptions,
        "cache": cache,
        "jobs": jobs,
        "mail": mail,
    }))
}

/// Entries API handler
async fn entries_handler(
    State(dashboard): State<Arc<Dashboard>>,
    Query(params): Query<EntryQuery>,
) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let (entries, total) = storage.paginate(None, params.page, params.per_page).await;

    Json(json!({
        "entries": entries,
        "total": total,
        "page": params.page,
        "per_page": params.per_page,
    }))
}

/// Requests API handler
async fn requests_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let entries = storage.by_type(EntryType::Request).await;

    Json(json!({
        "requests": entries,
    }))
}

/// Queries API handler
async fn queries_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let entries = storage.by_type(EntryType::Query).await;

    Json(json!({
        "queries": entries,
    }))
}

/// Exceptions API handler
async fn exceptions_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let entries = storage.by_type(EntryType::Exception).await;

    Json(json!({
        "exceptions": entries,
    }))
}

/// Cache API handler
async fn cache_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let entries = storage.by_type(EntryType::Cache).await;

    Json(json!({
        "cache": entries,
    }))
}

/// Jobs API handler
async fn jobs_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let entries = storage.by_type(EntryType::Job).await;

    Json(json!({
        "jobs": entries,
    }))
}

/// Mail API handler
async fn mail_handler(State(dashboard): State<Arc<Dashboard>>) -> impl IntoResponse {
    let storage = dashboard.telescope.storage();
    let entries = storage.by_type(EntryType::Mail).await;

    Json(json!({
        "mail": entries,
    }))
}

/// Dashboard HTML template
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Telescope - Debugging Dashboard</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f7fafc;
            min-height: 100vh;
        }

        .header {
            background: linear-gradient(135deg, #4c51bf 0%, #667eea 100%);
            color: white;
            padding: 2rem;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }

        .header h1 {
            font-size: 2rem;
            margin-bottom: 0.5rem;
        }

        .header p {
            opacity: 0.9;
        }

        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
        }

        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }

        .stat-card {
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .stat-card h3 {
            color: #718096;
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

        .tabs {
            display: flex;
            gap: 0.5rem;
            margin-bottom: 2rem;
            border-bottom: 2px solid #e2e8f0;
        }

        .tab {
            background: none;
            border: none;
            padding: 1rem 1.5rem;
            cursor: pointer;
            color: #718096;
            font-weight: 600;
            border-bottom: 2px solid transparent;
            margin-bottom: -2px;
            transition: all 0.2s;
        }

        .tab:hover {
            color: #4c51bf;
        }

        .tab.active {
            color: #4c51bf;
            border-bottom-color: #4c51bf;
        }

        .panel {
            background: white;
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
            display: none;
        }

        .panel.active {
            display: block;
        }

        .entry-list {
            list-style: none;
        }

        .entry-item {
            padding: 1.5rem;
            border-bottom: 1px solid #e2e8f0;
        }

        .entry-item:last-child {
            border-bottom: none;
        }

        .entry-header {
            display: flex;
            justify-content: space-between;
            align-items: start;
            margin-bottom: 0.5rem;
        }

        .entry-title {
            font-weight: 600;
            color: #2d3748;
            font-size: 1rem;
        }

        .entry-time {
            color: #a0aec0;
            font-size: 0.875rem;
        }

        .entry-content {
            color: #4a5568;
            font-size: 0.875rem;
            margin-top: 0.5rem;
        }

        .code-block {
            background: #2d3748;
            color: #e2e8f0;
            padding: 1rem;
            border-radius: 4px;
            overflow-x: auto;
            font-family: 'Monaco', 'Courier New', monospace;
            font-size: 0.875rem;
            margin-top: 0.5rem;
        }

        .badge {
            display: inline-block;
            padding: 0.25rem 0.75rem;
            border-radius: 12px;
            font-size: 0.75rem;
            font-weight: 600;
        }

        .badge.success {
            background: #c6f6d5;
            color: #22543d;
        }

        .badge.error {
            background: #fed7d7;
            color: #742a2a;
        }

        .badge.warning {
            background: #feebc8;
            color: #7c2d12;
        }

        .badge.info {
            background: #bee3f8;
            color: #2c5282;
        }

        .empty-state {
            text-align: center;
            padding: 4rem 2rem;
            color: #a0aec0;
        }

        .empty-state svg {
            width: 64px;
            height: 64px;
            margin-bottom: 1rem;
            opacity: 0.5;
        }

        .refresh-btn {
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            background: #4c51bf;
            color: white;
            border: none;
            padding: 1rem 1.5rem;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            transition: all 0.2s;
        }

        .refresh-btn:hover {
            background: #434190;
            transform: translateY(-2px);
            box-shadow: 0 6px 8px rgba(0, 0, 0, 0.15);
        }

        .duration {
            color: #ed8936;
            font-weight: 600;
        }

        .slow {
            color: #e53e3e;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>🔭 Telescope</h1>
        <p>Debugging Dashboard</p>
    </div>

    <div class="container">
        <div class="stats-grid">
            <div class="stat-card">
                <h3>Total Entries</h3>
                <div class="value" id="total-entries">0</div>
            </div>
            <div class="stat-card">
                <h3>Requests</h3>
                <div class="value" id="stat-requests">0</div>
            </div>
            <div class="stat-card">
                <h3>Queries</h3>
                <div class="value" id="stat-queries">0</div>
            </div>
            <div class="stat-card">
                <h3>Exceptions</h3>
                <div class="value" id="stat-exceptions">0</div>
            </div>
            <div class="stat-card">
                <h3>Jobs</h3>
                <div class="value" id="stat-jobs">0</div>
            </div>
            <div class="stat-card">
                <h3>Mail</h3>
                <div class="value" id="stat-mail">0</div>
            </div>
        </div>

        <div class="tabs">
            <button class="tab active" data-tab="all">All</button>
            <button class="tab" data-tab="requests">Requests</button>
            <button class="tab" data-tab="queries">Queries</button>
            <button class="tab" data-tab="exceptions">Exceptions</button>
            <button class="tab" data-tab="jobs">Jobs</button>
            <button class="tab" data-tab="mail">Mail</button>
        </div>

        <div id="panel-all" class="panel active">
            <ul class="entry-list" id="entries-all"></ul>
        </div>

        <div id="panel-requests" class="panel">
            <ul class="entry-list" id="entries-requests"></ul>
        </div>

        <div id="panel-queries" class="panel">
            <ul class="entry-list" id="entries-queries"></ul>
        </div>

        <div id="panel-exceptions" class="panel">
            <ul class="entry-list" id="entries-exceptions"></ul>
        </div>

        <div id="panel-jobs" class="panel">
            <ul class="entry-list" id="entries-jobs"></ul>
        </div>

        <div id="panel-mail" class="panel">
            <ul class="entry-list" id="entries-mail"></ul>
        </div>
    </div>

    <button class="refresh-btn" onclick="loadData()">Refresh</button>

    <script>
        // Tab switching
        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));

                tab.classList.add('active');
                document.getElementById(`panel-${tab.dataset.tab}`).classList.add('active');
            });
        });

        async function loadData() {
            try {
                // Load stats
                const statsRes = await fetch('/api/stats');
                const stats = await statsRes.json();

                document.getElementById('total-entries').textContent = stats.total_entries;
                document.getElementById('stat-requests').textContent = stats.requests;
                document.getElementById('stat-queries').textContent = stats.queries;
                document.getElementById('stat-exceptions').textContent = stats.exceptions;
                document.getElementById('stat-jobs').textContent = stats.jobs;
                document.getElementById('stat-mail').textContent = stats.mail;

                // Load entries
                await loadEntries();
                await loadRequests();
                await loadQueries();
                await loadExceptions();
                await loadJobs();
                await loadMail();
            } catch (error) {
                console.error('Error loading data:', error);
            }
        }

        function escapeHtml(str) {
            if (str === null || str === undefined) return '';
            return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#x27;');
        }

        async function loadEntries() {
            const res = await fetch('/api/entries');
            const data = await res.json();
            const list = document.getElementById('entries-all');

            if (data.entries.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No entries yet</div></li>';
                return;
            }

            list.innerHTML = data.entries.map(entry => `
                <li class="entry-item">
                    <div class="entry-header">
                        <div class="entry-title">${escapeHtml(entry.entry_type)}</div>
                        <div class="entry-time">${escapeHtml(new Date(entry.created_at).toLocaleString())}</div>
                    </div>
                    <div class="entry-content">${escapeHtml(JSON.stringify(entry.content, null, 2).substring(0, 200))}...</div>
                </li>
            `).join('');
        }

        async function loadRequests() {
            const res = await fetch('/api/requests');
            const data = await res.json();
            const list = document.getElementById('entries-requests');

            if (data.requests.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No requests</div></li>';
                return;
            }

            list.innerHTML = data.requests.map(entry => {
                const req = entry.content;
                const statusClass = req.status >= 500 ? 'error' : req.status >= 400 ? 'warning' : 'success';
                return `
                    <li class="entry-item">
                        <div class="entry-header">
                            <div>
                                <span class="badge ${statusClass}">${escapeHtml(req.status)}</span>
                                <strong>${escapeHtml(req.method)}</strong> ${escapeHtml(req.path)}
                            </div>
                            <div class="entry-time">
                                <span class="duration">${escapeHtml(req.duration_ms)}ms</span> •
                                ${escapeHtml(new Date(entry.created_at).toLocaleString())}
                            </div>
                        </div>
                        <div class="entry-content">IP: ${escapeHtml(req.ip_address)}</div>
                    </li>
                `;
            }).join('');
        }

        async function loadQueries() {
            const res = await fetch('/api/queries');
            const data = await res.json();
            const list = document.getElementById('entries-queries');

            if (data.queries.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No queries</div></li>';
                return;
            }

            list.innerHTML = data.queries.map(entry => {
                const query = entry.content;
                const slowClass = query.is_slow ? 'slow' : '';
                return `
                    <li class="entry-item">
                        <div class="entry-header">
                            <div class="entry-title">${escapeHtml(query.connection)}</div>
                            <div class="entry-time">
                                <span class="duration ${slowClass}">${escapeHtml(query.duration_ms.toFixed(2))}ms</span> •
                                ${escapeHtml(new Date(entry.created_at).toLocaleString())}
                            </div>
                        </div>
                        <div class="code-block">${escapeHtml(query.sql)}</div>
                        ${query.bindings.length > 0 ? `<div class="entry-content">Bindings: ${escapeHtml(JSON.stringify(query.bindings))}</div>` : ''}
                    </li>
                `;
            }).join('');
        }

        async function loadExceptions() {
            const res = await fetch('/api/exceptions');
            const data = await res.json();
            const list = document.getElementById('entries-exceptions');

            if (data.exceptions.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No exceptions</div></li>';
                return;
            }

            list.innerHTML = data.exceptions.map(entry => {
                const exc = entry.content;
                return `
                    <li class="entry-item">
                        <div class="entry-header">
                            <div>
                                <span class="badge error">${escapeHtml(exc.exception_type)}</span>
                                <strong>${escapeHtml(exc.message)}</strong>
                            </div>
                            <div class="entry-time">${escapeHtml(new Date(entry.created_at).toLocaleString())}</div>
                        </div>
                        ${exc.file ? `<div class="entry-content">${escapeHtml(exc.file)}:${escapeHtml(exc.line)}</div>` : ''}
                        ${exc.stack_trace && exc.stack_trace.length > 0 ? `<div class="code-block">${escapeHtml(exc.stack_trace.join('\n'))}</div>` : ''}
                    </li>
                `;
            }).join('');
        }

        async function loadJobs() {
            const res = await fetch('/api/jobs');
            const data = await res.json();
            const list = document.getElementById('entries-jobs');

            if (data.jobs.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No jobs</div></li>';
                return;
            }

            list.innerHTML = data.jobs.map(entry => {
                const job = entry.content;
                const statusClass = job.status === 'completed' ? 'success' : job.status === 'failed' ? 'error' : 'info';
                return `
                    <li class="entry-item">
                        <div class="entry-header">
                            <div>
                                <span class="badge ${statusClass}">${escapeHtml(job.status)}</span>
                                <strong>${escapeHtml(job.job_name)}</strong>
                            </div>
                            <div class="entry-time">
                                ${job.duration_ms ? `<span class="duration">${escapeHtml(job.duration_ms)}ms</span> •` : ''}
                                ${escapeHtml(new Date(entry.created_at).toLocaleString())}
                            </div>
                        </div>
                        <div class="entry-content">Queue: ${escapeHtml(job.queue)}</div>
                        ${job.error ? `<div class="entry-content" style="color: #e53e3e;">Error: ${escapeHtml(job.error)}</div>` : ''}
                    </li>
                `;
            }).join('');
        }

        async function loadMail() {
            const res = await fetch('/api/mail');
            const data = await res.json();
            const list = document.getElementById('entries-mail');

            if (data.mail.length === 0) {
                list.innerHTML = '<li class="empty-state"><div>No mail</div></li>';
                return;
            }

            list.innerHTML = data.mail.map(entry => {
                const mail = entry.content;
                return `
                    <li class="entry-item">
                        <div class="entry-header">
                            <div>
                                <strong>${escapeHtml(mail.subject)}</strong>
                            </div>
                            <div class="entry-time">${escapeHtml(new Date(entry.created_at).toLocaleString())}</div>
                        </div>
                        <div class="entry-content">
                            From: ${escapeHtml(mail.from)}<br>
                            To: ${escapeHtml(mail.to.join(', '))}<br>
                            ${mail.attachments.length > 0 ? `Attachments: ${mail.attachments.length}` : ''}
                        </div>
                    </li>
                `;
            }).join('');
        }

        // Load data on page load
        loadData();

        // Auto-refresh every 10 seconds
        setInterval(loadData, 10000);
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Telescope;

    #[tokio::test]
    async fn test_dashboard_creation() {
        let telescope = Telescope::new();
        let dashboard = Dashboard::new(telescope);
        let _router = dashboard.router();
    }

    #[tokio::test]
    async fn test_stats_endpoint() {
        let telescope = Telescope::new();
        let dashboard = Arc::new(Dashboard::new(telescope));

        let response = stats_handler(State(dashboard)).await;
        // Response should be JSON
        let _json = response.into_response();
    }

    #[tokio::test]
    async fn test_entries_endpoint() {
        let telescope = Telescope::new();
        let dashboard = Arc::new(Dashboard::new(telescope));

        let params = EntryQuery {
            page: 0,
            per_page: 20,
        };

        let response = entries_handler(State(dashboard), Query(params)).await;
        let _json = response.into_response();
    }
}
