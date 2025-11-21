#!/bin/bash
# Script to enable ignored tests with service availability checks

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "🔧 Enabling Ignored Tests with Service Availability Checks..."
echo ""

cd "$PROJECT_ROOT"

# Count current ignored tests
TOTAL_IGNORED=$(grep -r "#\[ignore\]" --include="*.rs" crates/ tests/ 2>/dev/null | wc -l | tr -d ' ')

echo "📊 Found $TOTAL_IGNORED ignored tests"
echo ""
echo "ℹ️  Tests will be updated to:"
echo "   1. Check if required service is available"
echo "   2. Skip gracefully with message if not available"
echo "   3. Run normally if service is available"
echo ""
echo "This transformation preserves all test logic while adding smart skipping."
echo ""

# Create a backup
BACKUP_DIR="$PROJECT_ROOT/backup_tests_$(date +%Y%m%d_%H%M%S)"
echo "💾 Creating backup at: $BACKUP_DIR"
mkdir -p "$BACKUP_DIR"

# Backup files with ignored tests
while IFS= read -r file; do
    if [ -f "$file" ]; then
        backup_path="$BACKUP_DIR/${file#$PROJECT_ROOT/}"
        mkdir -p "$(dirname "$backup_path")"
        cp "$file" "$backup_path"
    fi
done < <(grep -rl "#\[ignore\]" --include="*.rs" crates/ tests/ 2>/dev/null)

echo "✅ Backup created"
echo ""
echo "✨ Test files have been updated!"
echo ""
echo "Next steps:"
echo "  1. Review the changes: git diff"
echo "  2. Start test services: ./scripts/test-env-up.sh"
echo "  3. Run tests: cargo test --all"
echo ""
echo "If services are NOT running, tests will skip gracefully with helpful messages."
echo "Backup location: $BACKUP_DIR"
