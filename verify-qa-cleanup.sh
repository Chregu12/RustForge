#!/bin/bash
# Quality Assurance Verification Script
# Verifies all cleanup tasks have been completed

set -e

echo "===================="
echo "QA Cleanup Verification"
echo "===================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0

# Function to check a condition
check() {
    local name="$1"
    local command="$2"

    echo -n "Checking $name... "

    if eval "$command" &>/dev/null; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}"
        ((FAILED++))
    fi
}

# 1. Check Foundry references removed
echo "1. Foundry References Cleanup"
echo "------------------------------"

FOUNDRY_COUNT=$(grep -r "Foundry CLI" app/ domain/ 2>/dev/null | wc -l || echo 0)
if [ "$FOUNDRY_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓ PASS${NC} - No Foundry CLI references found"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC} - Found $FOUNDRY_COUNT Foundry CLI references"
    ((FAILED++))
fi

FOUNDRY_IMPORT_COUNT=$(grep -r "foundry_plugins" app/ domain/ 2>/dev/null | wc -l || echo 0)
if [ "$FOUNDRY_IMPORT_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓ PASS${NC} - No foundry_plugins imports found"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC} - Found $FOUNDRY_IMPORT_COUNT foundry_plugins imports"
    ((FAILED++))
fi

echo ""

# 2. Check TODO/FIXME comments resolved
echo "2. TODO/FIXME Comments Resolved"
echo "--------------------------------"

TODO_COUNT=$(grep -r "TODO:" app/ domain/ 2>/dev/null | wc -l || echo 0)
if [ "$TODO_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓ PASS${NC} - No TODO comments found"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC} - Found $TODO_COUNT TODO comments"
    ((FAILED++))
fi

FIXME_COUNT=$(grep -r "FIXME:" app/ domain/ 2>/dev/null | wc -l || echo 0)
if [ "$FIXME_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓ PASS${NC} - No FIXME comments found"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC} - Found $FIXME_COUNT FIXME comments"
    ((FAILED++))
fi

echo ""

# 3. Check test files exist
echo "3. Integration Test Files Created"
echo "----------------------------------"

check "app_models_test.rs exists" "test -f tests/integration/app_models_test.rs"
check "request_validation_test.rs exists" "test -f tests/integration/request_validation_test.rs"
check "events_test.rs exists" "test -f tests/integration/events_test.rs"
check "end_to_end_test.rs exists" "test -f tests/integration/end_to_end_test.rs"

echo ""

# 4. Check test count
echo "4. Test Coverage"
echo "----------------"

INTEGRATION_TESTS=$(grep -c "#\[test\]" tests/integration/app_models_test.rs tests/integration/request_validation_test.rs tests/integration/events_test.rs tests/integration/end_to_end_test.rs 2>/dev/null || echo 0)
echo "Integration tests found: $INTEGRATION_TESTS"

if [ "$INTEGRATION_TESTS" -ge 40 ]; then
    echo -e "${GREEN}✓ PASS${NC} - Found $INTEGRATION_TESTS integration tests (target: 40+)"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC} - Only found $INTEGRATION_TESTS integration tests (target: 40+)"
    ((FAILED++))
fi

UNIT_TESTS=$(grep -c "#\[test\]" app/http/requests/store_product_request.rs app/events/order_placed_event.rs app/listeners/send_order_email_listener.rs domain/models/account.rs domain/models/product.rs 2>/dev/null || echo 0)
echo "Unit tests found: $UNIT_TESTS"

if [ "$UNIT_TESTS" -ge 15 ]; then
    echo -e "${GREEN}✓ PASS${NC} - Found $UNIT_TESTS unit tests (target: 15+)"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ WARNING${NC} - Only found $UNIT_TESTS unit tests (target: 15+)"
fi

echo ""

# 5. Check Docker infrastructure
echo "5. Docker Infrastructure"
echo "------------------------"

check "docker-compose.test.yml exists" "test -f docker-compose.test.yml"
check "docker.rs module exists" "test -f crates/rf-testing/src/docker.rs"
check "Docker Compose has Redis service" "grep -q 'redis:' docker-compose.test.yml"
check "Docker Compose has PostgreSQL service" "grep -q 'postgres:' docker-compose.test.yml"
check "Docker Compose has MailHog service" "grep -q 'mailhog:' docker-compose.test.yml"

echo ""

# 6. Check CI/CD
echo "6. CI/CD Pipeline"
echo "-----------------"

check "integration-tests.yml exists" "test -f .github/workflows/integration-tests.yml"
check "Workflow has Redis service" "grep -q 'redis:' .github/workflows/integration-tests.yml"
check "Workflow has PostgreSQL service" "grep -q 'postgres:' .github/workflows/integration-tests.yml"
check "Workflow has coverage job" "grep -q 'test-coverage:' .github/workflows/integration-tests.yml"

echo ""

# 7. Check documentation
echo "7. Documentation"
echo "----------------"

check "QA_CLEANUP_REPORT.md exists" "test -f QA_CLEANUP_REPORT.md"
check "QA_SUMMARY.md exists" "test -f QA_SUMMARY.md"
check "rf-testing docker module documented" "grep -q '//!' crates/rf-testing/src/docker.rs"

echo ""

# 8. Check module documentation
echo "8. Module Documentation"
echo "-----------------------"

check "app/mod.rs has doc comments" "grep -q '///' app/mod.rs"
check "domain/mod.rs has doc comments" "grep -q '///' domain/mod.rs"
check "Account model documented" "grep -q '///' domain/models/account.rs"
check "Product model documented" "grep -q '///' domain/models/product.rs"

echo ""

# Summary
echo "===================="
echo "VERIFICATION SUMMARY"
echo "===================="
echo ""
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}✓ ALL CHECKS PASSED${NC}"
    echo ""
    echo "The QA cleanup is complete and ready for deployment!"
    exit 0
else
    echo -e "${RED}✗ SOME CHECKS FAILED${NC}"
    echo ""
    echo "Please review the failed checks above."
    exit 1
fi
