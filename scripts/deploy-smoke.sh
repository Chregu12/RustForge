#!/usr/bin/env bash
# deploy-smoke.sh — Staging deploy probe for the RustForge 'hello' example app.
#
# Builds a RELEASE binary of the 'hello' example, starts it, probes /health
# and the root route, asserts both return HTTP 200, then shuts the server down.
#
# Usage:
#   ./scripts/deploy-smoke.sh
#
# Optional environment variables:
#   SMOKE_PORT   Port the server listens on (default: 3737 — avoids collisions
#                with other dev services on 3000/8080).
#   SMOKE_WAIT   Seconds to wait for the server to be ready (default: 15).
#
# Exit codes:
#   0  All health/route checks passed — binary starts and serves traffic.
#   1  Build failed, server did not start in time, or a check returned non-200.
#
# NOTE: This script does NOT require any cloud secrets.  It is the local /
# CI proof that the compiled release artifact actually runs and responds.
# See docs/live-cloud-ci.md for the separate live-cloud proof (AWS/Redis/SMTP).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SMOKE_PORT="${SMOKE_PORT:-3737}"
SMOKE_WAIT="${SMOKE_WAIT:-15}"
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "[smoke] Stopping server (PID $SERVER_PID) ..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "============================================================"
echo " RustForge deploy-smoke probe"
echo " Repo:   $REPO_ROOT"
echo " Port:   $SMOKE_PORT"
echo "============================================================"

# -------------------------------------------------------------------
# 1. Build release binary
# -------------------------------------------------------------------
echo "[smoke] Building 'hello' example in release mode ..."
cargo build --release -p hello --manifest-path "$REPO_ROOT/Cargo.toml"
BINARY="$REPO_ROOT/target/release/hello"

if [ ! -x "$BINARY" ]; then
    echo "[smoke] ERROR: binary not found at $BINARY after build" >&2
    exit 1
fi
echo "[smoke] Binary: $BINARY"

# -------------------------------------------------------------------
# 2. Boot the server
# -------------------------------------------------------------------
# rf-config reads SERVER_PORT from the environment (see crates/rf-config/src/types.rs).
echo "[smoke] Starting server on port $SMOKE_PORT ..."
SERVER_PORT="$SMOKE_PORT" "$BINARY" &
SERVER_PID=$!
echo "[smoke] Server PID: $SERVER_PID"

# -------------------------------------------------------------------
# 3. Wait for the server to accept connections
# -------------------------------------------------------------------
echo "[smoke] Waiting up to ${SMOKE_WAIT}s for port $SMOKE_PORT ..."
READY=""
for i in $(seq 1 "$SMOKE_WAIT"); do
    if (echo > /dev/tcp/127.0.0.1/"$SMOKE_PORT") 2>/dev/null; then
        READY=1
        echo "[smoke] Server is up (${i}s)"
        break
    fi
    sleep 1
done

if [ -z "$READY" ]; then
    echo "[smoke] ERROR: server did not open port $SMOKE_PORT within ${SMOKE_WAIT}s" >&2
    exit 1
fi

# Give the HTTP stack one extra moment to finish binding.
sleep 1

# -------------------------------------------------------------------
# 4. Health check — GET /health → 200
# -------------------------------------------------------------------
echo "[smoke] Probing GET /health ..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${SMOKE_PORT}/health" || echo "000")
if [ "$HTTP_STATUS" != "200" ]; then
    echo "[smoke] FAIL: /health returned HTTP $HTTP_STATUS (expected 200)" >&2
    exit 1
fi
echo "[smoke] PASS: /health -> $HTTP_STATUS"

# -------------------------------------------------------------------
# 5. Root route — GET / → 200
# -------------------------------------------------------------------
echo "[smoke] Probing GET / ..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${SMOKE_PORT}/" || echo "000")
if [ "$HTTP_STATUS" != "200" ]; then
    echo "[smoke] FAIL: / returned HTTP $HTTP_STATUS (expected 200)" >&2
    exit 1
fi
echo "[smoke] PASS: / -> $HTTP_STATUS"

# -------------------------------------------------------------------
# Done
# -------------------------------------------------------------------
echo "============================================================"
echo " deploy-smoke: ALL CHECKS PASSED"
echo "  Binary:   $BINARY"
echo "  /health:  200 OK"
echo "  /:        200 OK"
echo "============================================================"
