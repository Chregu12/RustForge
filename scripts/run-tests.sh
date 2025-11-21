#!/bin/bash
# Run all tests including previously ignored ones

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

cd "$PROJECT_ROOT"

echo "🧪 Running RustForge Test Suite..."
echo ""

# Check if services are running
echo "📋 Checking test environment..."
if ! docker ps | grep -q rustforge_redis_test; then
    echo "⚠️  Test services not running. Starting them now..."
    ./scripts/test-env-up.sh
else
    echo "✅ Test services are running"
fi

echo ""
echo "🏃 Running all tests..."
echo ""

# Export test environment variables
export DATABASE_URL="postgresql://rustforge:testpass@localhost:5432/rustforge_test"
export REDIS_URL="redis://localhost:6379"
export MAIL_HOST="localhost"
export MAIL_PORT="1025"
export AWS_ENDPOINT="http://localhost:9000"
export AWS_ACCESS_KEY_ID="minioadmin"
export AWS_SECRET_ACCESS_KEY="minioadmin123"

# Run tests
if [ "$1" == "--coverage" ]; then
    echo "📊 Running tests with coverage..."
    cargo tarpaulin --all-features --workspace --timeout 120 --out Html --output-dir coverage
elif [ "$1" == "--ignored" ]; then
    echo "🔍 Running previously ignored tests only..."
    cargo test --all -- --ignored
else
    cargo test --all --verbose
fi

echo ""
echo "✅ Tests complete!"
echo ""
