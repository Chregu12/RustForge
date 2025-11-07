# 🚀 RustForge Deployment Guide

**Version:** 0.2.0
**Last Updated:** 2025-11-05
**Status:** Production Ready (Core Features)

---

## 📋 Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Setup](#environment-setup)
3. [Database Setup](#database-setup)
4. [Application Configuration](#application-configuration)
5. [Building for Production](#building-for-production)
6. [Deployment Options](#deployment-options)
7. [Docker Deployment](#docker-deployment)
8. [Kubernetes Deployment](#kubernetes-deployment)
9. [Monitoring & Logging](#monitoring--logging)
10. [Security Hardening](#security-hardening)
11. [Performance Optimization](#performance-optimization)
12. [Troubleshooting](#troubleshooting)

---

## 🔧 Prerequisites

### System Requirements

**Minimum:**
- **OS:** Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+), macOS 11+
- **CPU:** 2 cores
- **RAM:** 2 GB
- **Disk:** 10 GB free space

**Recommended (Production):**
- **CPU:** 4+ cores
- **RAM:** 8+ GB
- **Disk:** 50+ GB SSD

### Software Dependencies

```bash
# Rust Toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup update

# Build Tools (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev

# Build Tools (RHEL/CentOS/Fedora)
sudo yum groupinstall 'Development Tools'
sudo yum install openssl-devel

# Database (PostgreSQL - Recommended)
sudo apt-get install -y postgresql postgresql-contrib libpq-dev

# OR SQLite (Development/Small Scale)
sudo apt-get install -y sqlite3 libsqlite3-dev

# OR MySQL/MariaDB
sudo apt-get install -y mysql-server libmysqlclient-dev
```

---

## 🌍 Environment Setup

### 1. Clone and Build

```bash
# Clone repository
git clone https://github.com/your-org/rust-dx-framework.git
cd rust-dx-framework

# Build release version
cargo build --release

# Binary location
ls -lh target/release/foundry
```

### 2. Create Environment Configuration

```bash
# Copy example environment file
cp .env.example .env

# Edit with your settings
nano .env
```

### 3. Essential Environment Variables

```bash
# Application
APP_NAME="RustForge Production"
APP_ENV=production
APP_DEBUG=false
APP_KEY=base64:YOUR_GENERATED_KEY_HERE  # Generate with: foundry key:generate

# Server
HOST=0.0.0.0
PORT=8080
WORKERS=4  # Number of Tokio worker threads

# Database (PostgreSQL)
DB_CONNECTION=postgres
DATABASE_URL=postgresql://username:password@localhost:5432/rustforge_prod

# OR SQLite
# DB_CONNECTION=sqlite
# DATABASE_URL=sqlite:./rustforge.db

# Redis Cache (Optional)
REDIS_URL=redis://127.0.0.1:6379
CACHE_DRIVER=redis  # Options: redis, file, memory

# Mail
MAIL_DRIVER=smtp
MAIL_HOST=smtp.example.com
MAIL_PORT=587
MAIL_USERNAME=your_email@example.com
MAIL_PASSWORD=your_password
MAIL_ENCRYPTION=tls
MAIL_FROM_ADDRESS=noreply@example.com
MAIL_FROM_NAME="RustForge"

# OAuth2 (if using foundry-oauth-server)
OAUTH_JWT_SECRET=YOUR_256_BIT_SECRET_HERE
OAUTH_ACCESS_TOKEN_LIFETIME=3600  # seconds
OAUTH_REFRESH_TOKEN_LIFETIME=2592000  # 30 days

# Logging
RUST_LOG=info,rustforge=debug
LOG_LEVEL=info

# Security
ALLOWED_HOSTS=yourdomain.com,www.yourdomain.com
CORS_ORIGINS=https://yourdomain.com
RATE_LIMIT_MAX_REQUESTS=100
RATE_LIMIT_WINDOW_SECONDS=60

# Monitoring
ENABLE_METRICS=true
METRICS_PORT=9090
```

---

## 🗄️ Database Setup

### PostgreSQL (Recommended for Production)

```bash
# 1. Create database and user
sudo -u postgres psql

postgres=# CREATE DATABASE rustforge_prod;
postgres=# CREATE USER rustforge WITH ENCRYPTED PASSWORD 'your_secure_password';
postgres=# GRANT ALL PRIVILEGES ON DATABASE rustforge_prod TO rustforge;
postgres=# \q

# 2. Test connection
psql postgresql://rustforge:your_secure_password@localhost:5432/rustforge_prod

# 3. Run migrations
./target/release/foundry migrate

# 4. Seed database (if needed)
./target/release/foundry db:seed
```

### SQLite (Development/Small Scale)

```bash
# 1. Create database file
touch rustforge.db

# 2. Update .env
DATABASE_URL=sqlite:./rustforge.db

# 3. Run migrations
./target/release/foundry migrate
```

### MySQL/MariaDB

```bash
# 1. Create database
mysql -u root -p

mysql> CREATE DATABASE rustforge_prod CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
mysql> CREATE USER 'rustforge'@'localhost' IDENTIFIED BY 'your_secure_password';
mysql> GRANT ALL PRIVILEGES ON rustforge_prod.* TO 'rustforge'@'localhost';
mysql> FLUSH PRIVILEGES;
mysql> EXIT;

# 2. Update .env
DATABASE_URL=mysql://rustforge:your_secure_password@localhost:3306/rustforge_prod

# 3. Run migrations
./target/release/foundry migrate
```

---

## ⚙️ Application Configuration

### Generate Application Key

```bash
# Generate new encryption key
./target/release/foundry key:generate

# This will update your .env file with APP_KEY
```

### Configure OAuth2 (if using)

```bash
# Install OAuth2 server
./target/release/foundry passport:install

# Create OAuth2 client
./target/release/foundry passport:client \
  --name "Production Client" \
  --redirect "https://yourdomain.com/callback"

# Save the client credentials securely
```

### Cache Configuration

```bash
# Clear caches before deployment
./target/release/foundry cache:clear
./target/release/foundry config:clear

# Cache configuration for production
./target/release/foundry config:cache
./target/release/foundry route:cache
./target/release/foundry optimize
```

---

## 🏗️ Building for Production

### Optimized Release Build

```bash
# Build with maximum optimization
RUSTFLAGS='-C target-cpu=native' cargo build --release

# Strip debug symbols to reduce binary size
strip target/release/foundry

# Check binary size
ls -lh target/release/foundry

# Expected size: ~50-100 MB (depending on features)
```

### Profile-Guided Optimization (PGO)

```bash
# Step 1: Build with instrumentation
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release

# Step 2: Run typical workload
./target/release/foundry serve &
# Run load tests or typical operations
# Stop server

# Step 3: Merge profile data
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

# Step 4: Build with PGO
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata -Cllvm-args=-pgo-warn-missing-function" cargo build --release
```

---

## 🚀 Deployment Options

### Option 1: systemd Service (Linux)

```bash
# 1. Create service file
sudo nano /etc/systemd/system/rustforge.service
```

```ini
[Unit]
Description=RustForge Application Server
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=rustforge
Group=rustforge
WorkingDirectory=/opt/rustforge
Environment="RUST_LOG=info"
EnvironmentFile=/opt/rustforge/.env
ExecStart=/opt/rustforge/foundry serve --addr 0.0.0.0:8080
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustforge

# Security Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/rustforge/storage /opt/rustforge/logs
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

```bash
# 2. Create user and setup directories
sudo useradd -r -s /bin/false rustforge
sudo mkdir -p /opt/rustforge/{storage,logs}
sudo cp target/release/foundry /opt/rustforge/
sudo cp .env /opt/rustforge/
sudo chown -R rustforge:rustforge /opt/rustforge

# 3. Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable rustforge
sudo systemctl start rustforge

# 4. Check status
sudo systemctl status rustforge
sudo journalctl -u rustforge -f
```

### Option 2: Nginx Reverse Proxy

```bash
# Install Nginx
sudo apt-get install -y nginx certbot python3-certbot-nginx

# Create Nginx configuration
sudo nano /etc/nginx/sites-available/rustforge
```

```nginx
upstream rustforge_backend {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 80;
    server_name yourdomain.com www.yourdomain.com;

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com www.yourdomain.com;

    # SSL Configuration (Let's Encrypt)
    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Security Headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Logging
    access_log /var/log/nginx/rustforge_access.log;
    error_log /var/log/nginx/rustforge_error.log;

    # Client limits
    client_max_body_size 100M;

    location / {
        proxy_pass http://rustforge_backend;
        proxy_http_version 1.1;

        # Headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Connection "";

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # WebSocket support
    location /ws {
        proxy_pass http://rustforge_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 3600s;
    }

    # Static files (if applicable)
    location /static {
        alias /opt/rustforge/public;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

```bash
# Enable site and obtain SSL certificate
sudo ln -s /etc/nginx/sites-available/rustforge /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx

# Get SSL certificate
sudo certbot --nginx -d yourdomain.com -d www.yourdomain.com
```

---

## 🐳 Docker Deployment

### Dockerfile (Multi-Stage Build)

```dockerfile
# Stage 1: Build
FROM rust:1.82-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --bin foundry

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 rustforge

# Copy binary from builder
COPY --from=builder /app/target/release/foundry /usr/local/bin/foundry
COPY .env.example .env

# Set ownership
RUN chown -R rustforge:rustforge /app

# Switch to non-root user
USER rustforge

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/local/bin/foundry", "health:check"] || exit 1

# Run application
CMD ["/usr/local/bin/foundry", "serve", "--addr", "0.0.0.0:8080"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - APP_ENV=production
      - DATABASE_URL=postgresql://rustforge:password@db:5432/rustforge_prod
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      - db
      - redis
    restart: unless-stopped
    volumes:
      - ./storage:/app/storage
      - ./logs:/app/logs
    networks:
      - rustforge-network

  db:
    image: postgres:16-alpine
    environment:
      - POSTGRES_DB=rustforge_prod
      - POSTGRES_USER=rustforge
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres-data:/var/lib/postgresql/data
    networks:
      - rustforge-network
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    networks:
      - rustforge-network
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - app
    networks:
      - rustforge-network
    restart: unless-stopped

volumes:
  postgres-data:
  redis-data:

networks:
  rustforge-network:
    driver: bridge
```

### Build and Run

```bash
# Build image
docker-compose build

# Run containers
docker-compose up -d

# Check logs
docker-compose logs -f app

# Run migrations
docker-compose exec app foundry migrate

# Scale application
docker-compose up -d --scale app=3
```

---

## ☸️ Kubernetes Deployment

### deployment.yaml

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustforge
  labels:
    app: rustforge
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rustforge
  template:
    metadata:
      labels:
        app: rustforge
    spec:
      containers:
      - name: rustforge
        image: your-registry/rustforge:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: APP_ENV
          value: "production"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: rustforge-secrets
              key: database-url
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 2Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: rustforge-service
spec:
  selector:
    app: rustforge
  ports:
  - name: http
    port: 80
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
  type: LoadBalancer
---
apiVersion: v1
kind: Secret
metadata:
  name: rustforge-secrets
type: Opaque
data:
  database-url: <base64-encoded-database-url>
  app-key: <base64-encoded-app-key>
```

### Apply Configuration

```bash
# Create namespace
kubectl create namespace rustforge

# Apply secrets
kubectl create secret generic rustforge-secrets \
  --from-literal=database-url='postgresql://...' \
  --from-literal=app-key='base64:...' \
  --namespace=rustforge

# Deploy
kubectl apply -f deployment.yaml --namespace=rustforge

# Check status
kubectl get pods --namespace=rustforge
kubectl logs -f deployment/rustforge --namespace=rustforge
```

---

## 📊 Monitoring & Logging

### Prometheus Metrics

RustForge exposes Prometheus metrics on `/metrics` endpoint:

```bash
# Enable metrics in .env
ENABLE_METRICS=true
METRICS_PORT=9090
```

**Available Metrics:**
- `rustforge_requests_total` - Total HTTP requests
- `rustforge_request_duration_seconds` - Request latency histogram
- `rustforge_db_connections_active` - Active database connections
- `rustforge_cache_hits_total` - Cache hit count
- `rustforge_cache_misses_total` - Cache miss count

### Logging Configuration

```toml
# tracing-subscriber configuration
RUST_LOG=info,rustforge=debug,tower_http=trace

# Log to file
LOG_FILE=/var/log/rustforge/app.log
LOG_MAX_SIZE=100MB
LOG_MAX_BACKUPS=10
```

### Log Aggregation (ELK Stack)

```yaml
# filebeat.yml
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/rustforge/*.log
  json.keys_under_root: true
  json.add_error_key: true

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "rustforge-%{+yyyy.MM.dd}"

setup.kibana:
  host: "kibana:5601"
```

---

## 🔐 Security Hardening

### 1. Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable

# OR iptables
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -A INPUT -j DROP
```

### 2. Rate Limiting

```toml
# .env
RATE_LIMIT_ENABLED=true
RATE_LIMIT_MAX_REQUESTS=100
RATE_LIMIT_WINDOW_SECONDS=60
RATE_LIMIT_BLOCK_DURATION=3600
```

### 3. SSL/TLS Best Practices

```nginx
# In Nginx config
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384';
ssl_prefer_server_ciphers off;
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;
ssl_stapling on;
ssl_stapling_verify on;
```

### 4. Database Security

```sql
-- PostgreSQL: Restrict connections
-- In postgresql.conf
listen_addresses = 'localhost'

-- In pg_hba.conf
host    rustforge_prod    rustforge    127.0.0.1/32    md5
```

### 5. Environment Variables Protection

```bash
# Restrict .env file permissions
chmod 600 .env
chown rustforge:rustforge .env

# Never commit .env to version control
echo ".env" >> .gitignore
```

---

## ⚡ Performance Optimization

### 1. Connection Pooling

```toml
# Database connection pool
DB_POOL_MIN_CONNECTIONS=5
DB_POOL_MAX_CONNECTIONS=20
DB_POOL_TIMEOUT_SECONDS=30
```

### 2. Caching Strategy

```bash
# Redis configuration
CACHE_DRIVER=redis
CACHE_TTL=3600
CACHE_PREFIX=rustforge:

# Enable query caching
ENABLE_QUERY_CACHE=true
QUERY_CACHE_TTL=300
```

### 3. Asset Optimization

```bash
# Compile assets for production
./foundry assets:compile --minify

# Enable CDN for static assets
CDN_URL=https://cdn.yourdomain.com
SERVE_STATIC=false
```

### 4. HTTP/2 and Compression

```nginx
# In Nginx
http2 on;

gzip on;
gzip_vary on;
gzip_min_length 1024;
gzip_types text/plain text/css text/xml text/javascript
           application/json application/javascript application/xml+rss;
```

---

## 🔧 Troubleshooting

### Common Issues

#### 1. Database Connection Failed

```bash
# Check database status
sudo systemctl status postgresql

# Test connection
psql postgresql://username:password@localhost:5432/dbname

# Check logs
sudo journalctl -u postgresql -f
```

#### 2. Port Already in Use

```bash
# Find process using port 8080
sudo lsof -i :8080

# Kill process
sudo kill -9 <PID>

# Or use different port
./foundry serve --addr 0.0.0.0:3000
```

#### 3. Permission Denied

```bash
# Fix file permissions
sudo chown -R rustforge:rustforge /opt/rustforge
sudo chmod -R 755 /opt/rustforge
sudo chmod 600 /opt/rustforge/.env
```

#### 4. Out of Memory

```bash
# Check memory usage
free -h
docker stats

# Increase swap (if needed)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

#### 5. SSL Certificate Issues

```bash
# Renew Let's Encrypt certificate
sudo certbot renew

# Test SSL configuration
sudo nginx -t
openssl s_client -connect yourdomain.com:443
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=trace
export RUST_BACKTRACE=1

# Run in foreground
./foundry serve --verbose
```

### Health Checks

```bash
# Application health
curl http://localhost:8080/health

# Detailed health check
./foundry health:check --detailed

# Database connectivity
./foundry db:show
```

---

## 📚 Additional Resources

- **Official Documentation:** https://docs.rustforge.local
- **GitHub Repository:** https://github.com/your-org/rust-dx-framework
- **Community Forum:** https://forum.rustforge.local
- **Security Issues:** security@rustforge.local

---

## 📝 Deployment Checklist

- [ ] Environment variables configured
- [ ] Database migrated and seeded
- [ ] Application key generated
- [ ] SSL certificates installed
- [ ] Firewall configured
- [ ] Reverse proxy setup (Nginx)
- [ ] systemd service created and enabled
- [ ] Logs configured
- [ ] Monitoring enabled
- [ ] Backups scheduled
- [ ] Security headers configured
- [ ] Rate limiting enabled
- [ ] Health checks passing
- [ ] Load testing completed
- [ ] Documentation updated

---

**Deployed by:** RustForge Team
**Framework Version:** 0.2.0
**Deployment Guide Version:** 1.0.0

🚀 **Happy Deploying!**
