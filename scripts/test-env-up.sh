#!/bin/bash
# Start test environment services

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "🚀 Starting RustForge Test Environment..."
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Error: Docker is not running"
    echo "Please start Docker and try again"
    exit 1
fi

# Check if docker-compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Error: docker-compose is not installed"
    echo "Please install docker-compose and try again"
    exit 1
fi

# Navigate to project root
cd "$PROJECT_ROOT"

# Start services
echo "📦 Starting Docker services..."
docker-compose -f docker-compose.test.yml up -d

# Wait for services to be healthy
echo ""
echo "⏳ Waiting for services to be healthy..."
sleep 5

# Check Redis
echo -n "  Redis... "
if docker exec rustforge_redis_test redis-cli ping > /dev/null 2>&1; then
    echo "✅"
else
    echo "❌ (not ready)"
fi

# Check PostgreSQL
echo -n "  PostgreSQL... "
if docker exec rustforge_postgres_test pg_isready -U rustforge > /dev/null 2>&1; then
    echo "✅"
else
    echo "❌ (not ready)"
fi

# Check MailHog
echo -n "  MailHog... "
if curl -s http://localhost:8025 > /dev/null 2>&1; then
    echo "✅"
else
    echo "❌ (not ready)"
fi

# Check MinIO
echo -n "  MinIO... "
if curl -s http://localhost:9000/minio/health/live > /dev/null 2>&1; then
    echo "✅"
else
    echo "❌ (not ready)"
fi

echo ""
echo "✨ Test environment is ready!"
echo ""
echo "📊 Service URLs:"
echo "  - Redis: redis://localhost:6379"
echo "  - PostgreSQL: postgresql://rustforge:testpass@localhost:5432/rustforge_test"
echo "  - MailHog UI: http://localhost:8025"
echo "  - MailHog SMTP: localhost:1025"
echo "  - MinIO S3: http://localhost:9000"
echo "  - MinIO Console: http://localhost:9001 (minioadmin/minioadmin123)"
echo ""
echo "🧪 Run tests with: cargo test --all"
echo "🛑 Stop services with: ./scripts/test-env-down.sh"
echo ""
