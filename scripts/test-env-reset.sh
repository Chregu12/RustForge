#!/bin/bash
# Reset test environment (clean slate)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "🔄 Resetting RustForge Test Environment..."
echo ""

# Navigate to project root
cd "$PROJECT_ROOT"

# Stop and remove everything
echo "🧹 Cleaning up old containers and volumes..."
docker-compose -f docker-compose.test.yml down -v

# Start fresh
echo ""
echo "🚀 Starting fresh test environment..."
./scripts/test-env-up.sh
