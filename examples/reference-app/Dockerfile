# =============================================================================
# RustForge Reference Application — Docker image
# =============================================================================
#
# Build context: REPO ROOT (the workspace Cargo.toml must be at context root)
# Referenced by: examples/reference-app/docker-compose.yml
#
#   docker compose -f examples/reference-app/docker-compose.yml build
#
# Stage 1 (builder): compiles the release binary using the workspace toolchain.
# Stage 2 (runtime): copies only the binary into a minimal Debian image (~30 MB).
#
# HONEST CAPABILITY MAP — what is wired today vs future work:
#
#   SMTP (MailHog):  FULLY WIRED — set SMTP_HOST + SMTP_PORT in compose
#   Database:        SQLite only — DB facade uses rusqlite; Postgres service
#                    is present in compose for future rf-orm SeaORM integration
#   Cache:           MemoryCache — Redis service present for future wiring
#   Storage:         MemoryStorage — MinIO service present for future wiring

# ── Stage 1: Build ─────────────────────────────────────────────────────────
FROM rust:1.96 AS builder

# Build-time C deps: pkg-config + openssl-dev for reqwest TLS; build-essential
# for rusqlite's embedded SQLite (compiled from C at build time).
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests first so Docker can cache the dependency layer.
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members (cargo workspace resolution requires them all).
COPY crates ./crates
COPY examples ./examples
COPY tests ./tests
COPY benchmarks ./benchmarks

# Build ONLY the reference-app release binary; all other workspace members
# are compiled as transitive library deps (never as binaries).
RUN cargo build --release -p reference-app

# ── Stage 2: Runtime ───────────────────────────────────────────────────────
FROM debian:bookworm-slim

# curl: Docker HEALTHCHECK probe against /health.
# ca-certificates: TLS verification for outbound SMTP.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 appuser \
    && mkdir -p /app/data /tmp/rustforge-mailbox \
    && chown -R appuser:appuser /app /tmp/rustforge-mailbox

WORKDIR /app

COPY --from=builder /build/target/release/reference-app /usr/local/bin/reference-app

USER appuser

# PORT env var controls the listen address (default 3000 set inside the binary).
EXPOSE 3000

# Docker-level health probe — mirrors the Kubernetes readiness probe.
# /health is served by rf-health (MemoryCheck, no external deps) so it is
# always healthy as long as the process is alive and the port is open.
HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=3 \
    CMD curl -sf http://localhost:3000/health >/dev/null || exit 1

CMD ["reference-app"]
