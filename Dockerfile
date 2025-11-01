# Multi-stage Dockerfile for RustForge Framework
# Optimized for production builds with minimal image size

# Stage 1: Builder
FROM rust:1.75-slim as builder

# Install required build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY domain ./domain
COPY migrations ./migrations
COPY seeds ./seeds
COPY app ./app

# Build the application in release mode
RUN cargo build --release --bin foundry-cli

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 rustforge && \
    mkdir -p /app/storage /app/public && \
    chown -R rustforge:rustforge /app

# Set working directory
WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/foundry-cli /usr/local/bin/rustforge

# Copy application files
COPY --chown=rustforge:rustforge .env.example .env
COPY --chown=rustforge:rustforge migrations ./migrations
COPY --chown=rustforge:rustforge seeds ./seeds
COPY --chown=rustforge:rustforge public ./public

# Switch to non-root user
USER rustforge

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD rustforge health || exit 1

# Default command
CMD ["rustforge", "serve", "--host", "0.0.0.0", "--port", "8000"]
