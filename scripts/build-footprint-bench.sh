#!/usr/bin/env bash
# build-footprint-bench.sh — Measure build & runtime footprint:
#   compile time, binary size, startup time, idle RSS
#   for a minimal raw-axum app vs the RustForge blog-slice.
#
# Usage:
#   bash scripts/build-footprint-bench.sh
#
# Requirements:
#   - Rust toolchain (rustc/cargo)
#   - curl (for startup probe)
#   - ps (for RSS)
#   - strip (on macOS / Linux)
#   Internet access to download Cargo crates (first run only).
#
# What the axum-baseline is:
#   A standalone Cargo project (NOT a workspace member) created in a temp
#   directory during this script's run.  It implements the same REST surface
#   as blog-slice (GET /health, GET /posts, POST /posts, GET /posts/{id})
#   using plain axum 0.8 + tokio + serde, with an in-memory Vec for storage
#   (no sea-orm / sqlx / sqlite — those are RustForge deps, not axum deps).
#   This is the minimal "just axum" project a user would write when NOT using
#   any framework at all.
#
# Outputs a summary table and individual metric lines (METRIC=value) suitable
# for grepping.
#
# Exit codes:
#   0  All measurements completed successfully.
#   1  A required tool is missing or a build/probe step failed.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d -t rustforge-footprint-XXXXXX)"

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

echo "=================================================================="
echo " RustForge build-footprint benchmark"
echo " Repo:    $REPO_ROOT"
echo " Workdir: $WORK_DIR"
echo " rustc:   $(rustc --version)"
echo " cargo:   $(cargo --version)"
echo " Date:    $(date -u '+%Y-%m-%d %H:%M UTC')"
echo "=================================================================="

# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------
die() { echo "[footprint] ERROR: $*" >&2; exit 1; }
require() { command -v "$1" >/dev/null 2>&1 || die "'$1' not found — install it and retry."; }

require curl
require strip
require ps

# ------------------------------------------------------------------
# 1. Create the minimal raw-axum baseline project
# ------------------------------------------------------------------
AXUM_DIR="$WORK_DIR/axum-baseline"
mkdir -p "$AXUM_DIR/src"

cat >"$AXUM_DIR/Cargo.toml" <<'TOML'
[package]
name = "axum-baseline"
version = "0.1.0"
edition = "2021"
publish = false

# Standalone project — NOT a workspace member.
# Mirrors the axum version used by RustForge (axum 0.8) for a
# like-for-like compile-time / binary-size / runtime comparison.
[dependencies]
axum  = { version = "0.8", features = ["json"] }
tokio = { version = "1",   features = ["macros", "rt-multi-thread"] }
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
TOML

cat >"$AXUM_DIR/src/main.rs" <<'RUST'
//! Minimal raw-axum baseline for build & runtime footprint comparison.
//! Equivalent surface to the RustForge blog-slice:
//!   GET  /health
//!   GET  /posts
//!   POST /posts  {"title":"...", "body":"..."}
//!   GET  /posts/{id}
//!
//! Storage is an Arc<Mutex<Vec<Post>>> — no DB — so the only external deps
//! are axum + tokio + serde.  The RustForge blog-slice carries sea-orm,
//! sqlx, sqlite3-sys, and the full rf-* crate graph on top of these.
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post { id: u64, title: String, body: String }

#[derive(Deserialize)]
struct CreatePost { title: String, body: String }

type Db = Arc<Mutex<Vec<Post>>>;

async fn health() -> &'static str { "ok" }
async fn list_posts(State(db): State<Db>) -> Json<Vec<Post>> { Json(db.lock().unwrap().clone()) }
async fn show_post(State(db): State<Db>, Path(id): Path<u64>) -> Json<Option<Post>> {
    Json(db.lock().unwrap().iter().find(|p| p.id == id).cloned())
}
async fn create_post(State(db): State<Db>, Json(body): Json<CreatePost>) -> Json<Post> {
    let mut g = db.lock().unwrap();
    let id = g.len() as u64 + 1;
    let post = Post { id, title: body.title, body: body.body };
    g.push(post.clone());
    Json(post)
}

#[tokio::main]
async fn main() {
    let db: Db = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/health", get(health))
        .route("/posts", get(list_posts).post(create_post))
        .route("/posts/{id}", get(show_post))
        .with_state(db);
    let listener = TcpListener::bind("127.0.0.1:3099").await.unwrap();
    println!("axum-baseline listening on http://127.0.0.1:3099");
    axum::serve(listener, app).await.unwrap();
}
RUST

echo ""
echo "------------------------------------------------------------------"
echo " Step 1: Cold release build — raw axum baseline"
echo "------------------------------------------------------------------"
T0=$(($(date +%s%N)))
RUSTFLAGS="-Dwarnings" cargo build --release --manifest-path "$AXUM_DIR/Cargo.toml" 2>&1
T1=$(($(date +%s%N)))
AXUM_BUILD_MS=$(( (T1 - T0) / 1000000 ))
echo "AXUM_BUILD_MS=$AXUM_BUILD_MS"

echo ""
echo "------------------------------------------------------------------"
echo " Step 2: Cold release build — RustForge blog-slice"
echo "------------------------------------------------------------------"
RF_TARGET="$WORK_DIR/rf-target"
T0=$(($(date +%s%N)))
cargo build --release -p blog-slice \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    --target-dir "$RF_TARGET" 2>&1
T1=$(($(date +%s%N)))
RF_BUILD_MS=$(( (T1 - T0) / 1000000 ))
echo "RF_BUILD_MS=$RF_BUILD_MS"

echo ""
echo "------------------------------------------------------------------"
echo " Step 3: Binary sizes"
echo "------------------------------------------------------------------"
AXUM_BIN="$AXUM_DIR/target/release/axum-baseline"
RF_BIN="$RF_TARGET/release/blog-slice"

# Unstripped
AXUM_BYTES_RAW=$(wc -c < "$AXUM_BIN")
RF_BYTES_RAW=$(wc -c < "$RF_BIN")

# Strip
AXUM_BIN_STRIPPED="$WORK_DIR/axum-stripped"
RF_BIN_STRIPPED="$WORK_DIR/rf-stripped"
cp "$AXUM_BIN" "$AXUM_BIN_STRIPPED"
cp "$RF_BIN"   "$RF_BIN_STRIPPED"
strip "$AXUM_BIN_STRIPPED"
strip "$RF_BIN_STRIPPED"
AXUM_BYTES=$(wc -c < "$AXUM_BIN_STRIPPED")
RF_BYTES=$(wc -c < "$RF_BIN_STRIPPED")

echo "AXUM_BINARY_BYTES_STRIPPED=$AXUM_BYTES"
echo "RF_BINARY_BYTES_STRIPPED=$RF_BYTES"

# ------------------------------------------------------------------
# Startup probe helper
# ------------------------------------------------------------------
probe_startup_ms() {
    local bin="$1" url="$2" port="$3"
    lsof -ti:"$port" 2>/dev/null | xargs kill -9 2>/dev/null || true
    sleep 0.3
    "$bin" 2>/dev/null &
    local pid=$!
    local t_start t_ready
    t_start=$(($(date +%s%N)))
    local status=""
    for _i in $(seq 1 300); do
        status=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 0.1 "$url" 2>/dev/null || echo "000")
        if [ "$status" = "200" ]; then
            t_ready=$(($(date +%s%N)))
            echo $(( (t_ready - t_start) / 1000000 ))
            kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null || true
            return
        fi
        sleep 0.01
    done
    kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null || true
    die "Server at $url did not respond 200 within 3s"
}

# ------------------------------------------------------------------
# RSS helper
# ------------------------------------------------------------------
probe_rss_kb() {
    local bin="$1" url="$2" port="$3"
    lsof -ti:"$port" 2>/dev/null | xargs kill -9 2>/dev/null || true
    sleep 0.3
    "$bin" 2>/dev/null &
    local pid=$!
    local status=""
    for _i in $(seq 1 100); do
        status=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 0.1 "$url" 2>/dev/null || echo "000")
        [ "$status" = "200" ] && break
        sleep 0.05
    done
    [ "$status" != "200" ] && die "Server did not start for RSS probe"
    sleep 1.0
    local rss
    rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ')
    kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null || true
    echo "$rss"
}

echo ""
echo "------------------------------------------------------------------"
echo " Step 4: Startup time (5 runs each, 10ms polling)"
echo "------------------------------------------------------------------"
AXUM_STARTUPS=()
RF_STARTUPS=()
for _i in 1 2 3 4 5; do
    ms=$(probe_startup_ms "$AXUM_BIN_STRIPPED" http://127.0.0.1:3099/health 3099)
    AXUM_STARTUPS+=("$ms")
    echo "  axum-baseline: ${ms}ms"
    sleep 0.2
done
for _i in 1 2 3 4 5; do
    ms=$(probe_startup_ms "$RF_BIN_STRIPPED" http://127.0.0.1:3000/posts 3000)
    RF_STARTUPS+=("$ms")
    echo "  blog-slice:    ${ms}ms"
    sleep 0.2
done

# Median (sort + pick middle)
median() { printf '%s\n' "$@" | sort -n | awk 'BEGIN{s=0} {a[s++]=$1} END{print a[int(s/2)]}'; }
AXUM_STARTUP_MED=$(median "${AXUM_STARTUPS[@]}")
RF_STARTUP_MED=$(median "${RF_STARTUPS[@]}")

echo "AXUM_STARTUP_MEDIAN_MS=$AXUM_STARTUP_MED"
echo "RF_STARTUP_MEDIAN_MS=$RF_STARTUP_MED"

echo ""
echo "------------------------------------------------------------------"
echo " Step 5: Idle RSS (resident set size after boot)"
echo "------------------------------------------------------------------"
AXUM_RSS=$(probe_rss_kb "$AXUM_BIN_STRIPPED" http://127.0.0.1:3099/health 3099)
RF_RSS=$(probe_rss_kb "$RF_BIN_STRIPPED" http://127.0.0.1:3000/posts 3000)
echo "AXUM_IDLE_RSS_KB=$AXUM_RSS"
echo "RF_IDLE_RSS_KB=$RF_RSS"

echo ""
echo "=================================================================="
echo " RESULTS"
echo "=================================================================="
printf "%-30s %15s %15s %15s\n" "Metric" "raw axum" "RustForge" "Ratio"
printf "%-30s %15s %15s %15s\n" "-----" "--------" "---------" "-----"
printf "%-30s %15s %15s %15s\n" "Cold compile time" "${AXUM_BUILD_MS}ms" "${RF_BUILD_MS}ms" "$(( RF_BUILD_MS / (AXUM_BUILD_MS + 1) ))×"
printf "%-30s %15s %15s %15s\n" "Binary size (stripped)" "${AXUM_BYTES}B" "${RF_BYTES}B" "$(awk "BEGIN{printf \"%.1f×\",${RF_BYTES}/${AXUM_BYTES}}")"
printf "%-30s %15s %15s %15s\n" "Startup time (median)" "${AXUM_STARTUP_MED}ms" "${RF_STARTUP_MED}ms" "~1×"
printf "%-30s %15s %15s %15s\n" "Idle RSS" "${AXUM_RSS}KB" "${RF_RSS}KB" "$(awk "BEGIN{printf \"%.1f×\",${RF_RSS}/${AXUM_RSS}}")"
echo "=================================================================="
