#!/usr/bin/env bash
# reference-app-smoke.sh — Hermetic smoke test for the RustForge reference app.
#
# Builds a RELEASE binary of the reference app (examples/reference-app),
# starts it with SQLite in-memory + MemoryCache + MemoryStorage + FileMailer
# (no external services required), waits for readiness, then probes:
#
#   GET /health  → 200  (rf-health MemoryCheck — always healthy)
#   GET /posts   → 200  (public route; returns [] from fresh in-memory DB)
#   GET /metrics → 200  (rf-metrics Prometheus text endpoint)
#
# Exit codes:
#   0  All checks passed — binary starts and serves the three probed endpoints.
#   1  Build failed, server did not start in time, or any check returned non-200.
#
# Optional environment variables:
#   SMOKE_PORT   Port the server listens on (default: 3838).
#   SMOKE_WAIT   Seconds to wait for the server to be ready (default: 60).
#
# What this proves vs what needs real services:
#   PROVED (hermetic, no secrets):
#     - Release binary compiles from the workspace
#     - Server boots with zero external services (sqlite:memory, MemoryCache,
#       MemoryStorage, FileMailer) — CI-safe, no Postgres/Redis/S3/SMTP needed
#     - /health returns 200 (MemoryCheck passes)
#     - /posts returns 200 (schema migrations run at boot; empty list is valid)
#     - /metrics returns 200 (Prometheus endpoint is always up)
#   NEEDS REAL SERVICES (not proved here):
#     - Auth round-trip (register → login → JWT-protected routes) — hermetic
#       but BCrypt cost=12 is slow; omitted from sub-30s smoke test
#     - File upload (StorageFacade PUT) — needs a multipart POST
#     - SMTP delivery — needs SMTP_HOST wired to a real relay or MailHog
#     - Postgres persistence — DB facade is SQLite-only (rusqlite); tracked in
#       VISION_GAP.md; full Postgres needs rf-orm SeaORM DatabaseManager

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SMOKE_PORT="${SMOKE_PORT:-3838}"
SMOKE_WAIT="${SMOKE_WAIT:-60}"
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "[ref-smoke] Stopping server (PID $SERVER_PID) ..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
        echo "[ref-smoke] Server stopped."
    fi
}
trap cleanup EXIT

echo "============================================================"
echo " RustForge reference-app smoke probe"
echo " Repo:   $REPO_ROOT"
echo " Port:   $SMOKE_PORT"
echo " Backends: SQLite in-memory, MemoryCache, MemoryStorage, FileMailer"
echo "============================================================"

# -------------------------------------------------------------------
# 1. Build release binary
# -------------------------------------------------------------------
echo "[ref-smoke] Building reference-app in release mode ..."
cargo build --release -p reference-app --manifest-path "$REPO_ROOT/Cargo.toml"
BINARY="$REPO_ROOT/target/release/reference-app"

if [ ! -x "$BINARY" ]; then
    echo "[ref-smoke] ERROR: binary not found at $BINARY after build" >&2
    exit 1
fi
echo "[ref-smoke] Binary: $BINARY"

# -------------------------------------------------------------------
# 2. Boot the server (hermetic: no DATABASE_URL → in-memory SQLite)
# -------------------------------------------------------------------
echo "[ref-smoke] Starting server on port $SMOKE_PORT (hermetic mode) ..."
PORT="$SMOKE_PORT" "$BINARY" &
SERVER_PID=$!
echo "[ref-smoke] Server PID: $SERVER_PID"

# -------------------------------------------------------------------
# 3. Wait for readiness (TCP connect probe)
# -------------------------------------------------------------------
echo "[ref-smoke] Waiting up to ${SMOKE_WAIT}s for port $SMOKE_PORT ..."
READY=""
for i in $(seq 1 "$SMOKE_WAIT"); do
    if (echo > /dev/tcp/127.0.0.1/"$SMOKE_PORT") 2>/dev/null; then
        READY=1
        echo "[ref-smoke] Server is up (${i}s)"
        break
    fi
    sleep 1
done

if [ -z "$READY" ]; then
    echo "[ref-smoke] ERROR: server did not open port $SMOKE_PORT within ${SMOKE_WAIT}s" >&2
    exit 1
fi

# One extra moment for the HTTP stack to finish binding.
sleep 1

# -------------------------------------------------------------------
# Helper: assert HTTP 200
# -------------------------------------------------------------------
assert_200() {
    local label="$1"
    local url="$2"
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" "$url" || echo "000")
    if [ "$status" != "200" ]; then
        echo "[ref-smoke] FAIL: $label returned HTTP $status (expected 200)" >&2
        exit 1
    fi
    echo "[ref-smoke] PASS: $label -> $status"
}

BASE="http://127.0.0.1:${SMOKE_PORT}"

# -------------------------------------------------------------------
# 4. Health check — GET /health → 200
# -------------------------------------------------------------------
echo "[ref-smoke] Probing GET /health ..."
assert_200 "/health" "${BASE}/health"

# -------------------------------------------------------------------
# 5. Real route — GET /posts → 200
#    (public; returns [] from fresh in-memory SQLite after migrations)
# -------------------------------------------------------------------
echo "[ref-smoke] Probing GET /posts ..."
assert_200 "/posts" "${BASE}/posts"

# -------------------------------------------------------------------
# 6. Metrics — GET /metrics → 200
# -------------------------------------------------------------------
echo "[ref-smoke] Probing GET /metrics ..."
assert_200 "/metrics" "${BASE}/metrics"

# -------------------------------------------------------------------
# Done
# -------------------------------------------------------------------
echo "============================================================"
echo " reference-app-smoke: ALL CHECKS PASSED"
echo "  Binary:   $BINARY"
echo "  /health:  200 OK (MemoryCheck healthy)"
echo "  /posts:   200 OK (returns [] — migrations ran, no posts yet)"
echo "  /metrics: 200 OK (Prometheus text format)"
echo "============================================================"
