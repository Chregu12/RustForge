#!/bin/bash
# Stop test environment services

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "🛑 Stopping RustForge Test Environment..."
echo ""

# Navigate to project root
cd "$PROJECT_ROOT"

# Stop services
docker-compose -f docker-compose.test.yml down

echo ""
echo "✅ Test environment stopped"
echo ""
echo "💡 Tip: To remove volumes and data, run:"
echo "   docker-compose -f docker-compose.test.yml down -v"
echo ""
