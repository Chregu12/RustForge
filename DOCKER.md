# Docker Integration - RustForge Framework

## Quick Start

### Development Environment

Start the entire stack with PostgreSQL and Redis:

```bash
docker-compose up -d
```

This will start:
- RustForge application (port 8000)
- PostgreSQL database (port 5432)
- Redis cache (port 6379)

### Using MySQL Instead of PostgreSQL

```bash
docker-compose --profile mysql up -d
```

### Production Deployment

Build and run with Nginx reverse proxy:

```bash
docker-compose --profile production up -d
```

## Docker Commands

### Build

Build the Docker image:

```bash
docker build -t rustforge:latest .
```

Build with custom tag:

```bash
docker build -t rustforge:v1.0.0 .
```

### Run

Run the container:

```bash
docker run -p 8000:8000 \
  -e DATABASE_URL=postgres://user:pass@host/db \
  rustforge:latest
```

Run with volume mounts:

```bash
docker run -p 8000:8000 \
  -v $(pwd)/storage:/app/storage \
  -v $(pwd)/.env:/app/.env \
  rustforge:latest
```

### Database Migrations

Run migrations in the container:

```bash
docker-compose exec app rustforge migrate
```

Seed the database:

```bash
docker-compose exec app rustforge db:seed
```

### Logs

View application logs:

```bash
docker-compose logs -f app
```

View all logs:

```bash
docker-compose logs -f
```

### Shell Access

Access the application container:

```bash
docker-compose exec app sh
```

Access the database:

```bash
docker-compose exec db psql -U postgres -d rustforge
```

## Environment Variables

Create a `.env` file based on `.env.example`:

```env
# Application
APP_ENV=production
APP_DEBUG=false
APP_PORT=8000
LOG_LEVEL=info

# Database (PostgreSQL)
DATABASE_URL=postgres://postgres:secret@db:5432/rustforge
DB_NAME=rustforge
DB_USER=postgres
DB_PASSWORD=secret
DB_PORT=5432

# Redis
REDIS_URL=redis://redis:6379
REDIS_PORT=6379

# MySQL (alternative)
MYSQL_ROOT_PASSWORD=secret
MYSQL_PORT=3306
```

## Docker Compose Services

### Application Service

The main RustForge application:

```yaml
services:
  app:
    build: .
    ports:
      - "8000:8000"
    depends_on:
      - db
      - redis
```

### Database Service

PostgreSQL database:

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: rustforge
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: secret
```

### Cache Service

Redis cache:

```yaml
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
```

## Production Best Practices

### Multi-Stage Build

The Dockerfile uses multi-stage builds to minimize image size:

- **Builder stage**: Compiles the Rust application
- **Runtime stage**: Contains only the binary and runtime dependencies

### Security

- Runs as non-root user (`rustforge`)
- Minimal base image (Debian Slim)
- No unnecessary tools in production image

### Health Checks

Built-in health check:

```yaml
healthcheck:
  test: ["CMD", "rustforge", "health"]
  interval: 30s
  timeout: 3s
  retries: 3
```

### Resource Limits

Add resource limits in production:

```yaml
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 2G
    reservations:
      cpus: '1'
      memory: 1G
```

## Volumes

### Storage Volume

Persistent file storage:

```yaml
volumes:
  - ./storage:/app/storage
```

### Database Volume

Persistent database data:

```yaml
volumes:
  postgres-data:
    driver: local
```

## Networking

All services are connected via a bridge network:

```yaml
networks:
  rustforge-network:
    driver: bridge
```

## Troubleshooting

### Container won't start

Check logs:

```bash
docker-compose logs app
```

### Database connection issues

Verify database is healthy:

```bash
docker-compose ps
docker-compose exec db pg_isready -U postgres
```

### Port conflicts

Change ports in `.env`:

```env
APP_PORT=8001
DB_PORT=5433
REDIS_PORT=6380
```

### Permission issues

Fix storage permissions:

```bash
sudo chown -R 1000:1000 storage/
```

## Development Workflow

### Hot Reload (Development)

For development with hot reload, use volume mounts:

```yaml
volumes:
  - ./:/app
  - /app/target  # Cache target directory
```

### Running Tests

```bash
docker-compose exec app cargo test
```

### Debugging

Enable debug logging:

```env
LOG_LEVEL=debug
APP_DEBUG=true
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Docker Build

on:
  push:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Build Docker image
        run: docker build -t rustforge:${{ github.sha }} .

      - name: Run tests
        run: |
          docker-compose up -d db redis
          docker-compose run app cargo test
```

## Kubernetes Deployment

Example Kubernetes deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustforge
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
        image: rustforge:latest
        ports:
        - containerPort: 8000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: rustforge-secrets
              key: database-url
```

## Performance Optimization

### Build Cache

Speed up builds with cache mounts:

```dockerfile
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release
```

### Layer Caching

Optimize layer caching by copying Cargo files first:

```dockerfile
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch
COPY . .
RUN cargo build --release
```

## Monitoring

### Health Endpoint

The application exposes a health endpoint:

```bash
curl http://localhost:8000/health
```

### Metrics

View container metrics:

```bash
docker stats rustforge-app
```

## Cleanup

### Stop and remove containers

```bash
docker-compose down
```

### Remove volumes (WARNING: deletes data)

```bash
docker-compose down -v
```

### Clean up images

```bash
docker image prune -a
```

---

For more information, see the main [README.md](README.md) or visit the documentation.
