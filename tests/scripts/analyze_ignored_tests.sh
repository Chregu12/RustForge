#!/bin/bash

# analyze_ignored_tests.sh
# Analyzes all ignored tests in the codebase and generates a report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "============================================"
echo "RustForge Ignored Tests Analysis"
echo "============================================"
echo ""

cd "$PROJECT_ROOT"

# Count total ignored tests
TOTAL_IGNORED=$(grep -r "#\[ignore" --include="*.rs" . | wc -l | tr -d ' ')

echo "Total Ignored Tests: $TOTAL_IGNORED"
echo ""

# Category 1: Database tests
echo "--- Category 1: Database Tests ---"
DB_TESTS=$(grep -r "#\[ignore.*database" --include="*.rs" . | wc -l | tr -d ' ')
echo "Count: $DB_TESTS"
grep -r "#\[ignore.*database" --include="*.rs" . | head -10
echo ""

# Category 2: Redis tests
echo "--- Category 2: Redis Tests ---"
REDIS_TESTS=$(grep -r "#\[ignore.*[Rr]edis" --include="*.rs" . | wc -l | tr -d ' ')
echo "Count: $REDIS_TESTS"
grep -r "#\[ignore.*[Rr]edis" --include="*.rs" .
echo ""

# Category 3: AWS/S3 tests
echo "--- Category 3: AWS/S3 Tests ---"
S3_TESTS=$(grep -r "#\[ignore.*[Aa][Ww][Ss]" --include="*.rs" . | wc -l | tr -d ' ')
echo "Count: $S3_TESTS"
grep -r "#\[ignore.*[Aa][Ww][Ss]" --include="*.rs" .
echo ""

# Category 4: Integration tests
echo "--- Category 4: Integration Tests ---"
INTEGRATION_TESTS=$(grep -r "#\[ignore.*integration" --include="*.rs" . | wc -l | tr -d ' ')
echo "Count: $INTEGRATION_TESTS"
grep -r "#\[ignore.*integration" --include="*.rs" . | head -10
echo ""

# Generate summary
echo "============================================"
echo "Summary"
echo "============================================"
echo "Total Ignored Tests:    $TOTAL_IGNORED"
echo "Database Tests:         $DB_TESTS"
echo "Redis Tests:            $REDIS_TESTS"
echo "S3/AWS Tests:           $S3_TESTS"
echo "Integration Tests:      $INTEGRATION_TESTS"
echo "Other:                  $((TOTAL_IGNORED - DB_TESTS - REDIS_TESTS - S3_TESTS - INTEGRATION_TESTS))"
echo ""

# Breakdown by crate
echo "============================================"
echo "Breakdown by Crate"
echo "============================================"
for crate in $(find crates -name "Cargo.toml" -exec dirname {} \;); do
    crate_name=$(basename "$crate")
    count=$(grep -r "#\[ignore" --include="*.rs" "$crate" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$count" -gt 0 ]; then
        echo "$crate_name: $count"
    fi
done
echo ""

# Recommendations
echo "============================================"
echo "Recommendations"
echo "============================================"
echo "1. Start Docker Compose test infrastructure:"
echo "   docker-compose -f tests/docker-compose.test.yml up -d"
echo ""
echo "2. Enable database tests incrementally:"
echo "   - Remove #[ignore] from 1-2 tests at a time"
echo "   - Fix any failures"
echo "   - Commit working tests"
echo ""
echo "3. Run ignored tests to see current failures:"
echo "   cargo test -- --ignored 2>&1 | tee test_failures.log"
echo ""
echo "4. Track progress:"
echo "   - Target: 0 ignored tests"
echo "   - Current: $TOTAL_IGNORED"
echo "   - Progress: $((100 - (TOTAL_IGNORED * 100 / 89)))% complete"
echo ""

# Save report to file
REPORT_FILE="$PROJECT_ROOT/IGNORED_TESTS_REPORT.md"
cat > "$REPORT_FILE" <<EOF
# Ignored Tests Analysis Report

**Generated:** $(date)
**Total Ignored Tests:** $TOTAL_IGNORED

## Summary

| Category | Count |
|----------|-------|
| Database Tests | $DB_TESTS |
| Redis Tests | $REDIS_TESTS |
| S3/AWS Tests | $S3_TESTS |
| Integration Tests | $INTEGRATION_TESTS |
| Other | $((TOTAL_IGNORED - DB_TESTS - REDIS_TESTS - S3_TESTS - INTEGRATION_TESTS)) |
| **Total** | **$TOTAL_IGNORED** |

## Progress

- **Target:** 0 ignored tests (100% enabled)
- **Current:** $TOTAL_IGNORED ignored tests
- **Progress:** $((100 - (TOTAL_IGNORED * 100 / 89)))% complete

## Action Plan

### Phase 1: Database Tests ($DB_TESTS tests)

1. Start PostgreSQL test database
2. Run migrations
3. Enable tests incrementally
4. Fix failures

### Phase 2: Redis Tests ($REDIS_TESTS tests)

1. Start Redis test server
2. Enable cache tests
3. Enable queue tests

### Phase 3: S3 Tests ($S3_TESTS tests)

1. Start MinIO server
2. Configure test credentials
3. Enable storage tests

### Phase 4: Integration Tests ($INTEGRATION_TESTS tests)

1. Set up complete test infrastructure
2. Enable end-to-end tests
3. Verify all components work together

## Detailed Breakdown

### By Crate

EOF

for crate in $(find crates -name "Cargo.toml" -exec dirname {} \;); do
    crate_name=$(basename "$crate")
    count=$(grep -r "#\[ignore" --include="*.rs" "$crate" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$count" -gt 0 ]; then
        echo "- \`$crate_name\`: $count tests" >> "$REPORT_FILE"
    fi
done

cat >> "$REPORT_FILE" <<EOF

## Commands

\`\`\`bash
# Start test infrastructure
docker-compose -f tests/docker-compose.test.yml up -d

# Run ignored tests
cargo test -- --ignored

# Run specific category
cargo test --test '*relationship*' -- --ignored

# Check infrastructure health
docker-compose -f tests/docker-compose.test.yml ps
\`\`\`

## Next Steps

1. [ ] Set up Docker Compose test infrastructure
2. [ ] Enable database tests (highest priority)
3. [ ] Enable Redis tests
4. [ ] Enable S3 tests
5. [ ] Enable integration tests
6. [ ] Achieve 100% test enablement

---

**Target Date:** 2-4 weeks
**Owner:** QA Team
EOF

echo "Report saved to: $REPORT_FILE"
echo ""
