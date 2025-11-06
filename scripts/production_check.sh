#!/usr/bin/env bash

# RustForge Production Verification Script
# This script performs comprehensive checks to ensure the framework is production-ready

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0
WARNINGS=0

# Helper functions
print_header() {
    echo -e "\n${BLUE}===========================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===========================================================${NC}\n"
}

print_check() {
    echo -e "${YELLOW}[CHECK]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    PASSED=$((PASSED + 1))
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    FAILED=$((FAILED + 1))
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Main checks
print_header "RustForge Production Verification"

# 1. Check Rust toolchain
print_check "Verifying Rust toolchain..."
if command_exists rustc; then
    RUST_VERSION=$(rustc --version)
    print_success "Rust installed: $RUST_VERSION"
else
    print_error "Rust is not installed"
    exit 1
fi

if command_exists cargo; then
    CARGO_VERSION=$(cargo --version)
    print_success "Cargo installed: $CARGO_VERSION"
else
    print_error "Cargo is not installed"
    exit 1
fi

# 2. Check project structure
print_check "Verifying project structure..."
if [ -f "Cargo.toml" ]; then
    print_success "Cargo.toml found"
else
    print_error "Cargo.toml not found"
fi

if [ -d "crates" ]; then
    print_success "Crates directory found"
else
    print_error "Crates directory not found"
fi

# 3. Build all crates
print_check "Building all crates..."
if cargo build --workspace --all-features 2>&1 | tee /tmp/build.log; then
    print_success "All crates built successfully"
else
    print_error "Build failed. Check /tmp/build.log for details"
    cat /tmp/build.log
fi

# 4. Build release
print_check "Building release version..."
if cargo build --workspace --all-features --release 2>&1 | tee /tmp/build-release.log; then
    print_success "Release build successful"
else
    print_error "Release build failed. Check /tmp/build-release.log for details"
fi

# 5. Run tests
print_check "Running test suite..."
if cargo test --workspace --all-features 2>&1 | tee /tmp/test.log; then
    print_success "All tests passed"
else
    print_error "Some tests failed. Check /tmp/test.log for details"
fi

# 6. Run clippy
print_check "Running clippy (strict mode)..."
if cargo clippy --workspace --all-features --all-targets -- -D warnings 2>&1 | tee /tmp/clippy.log; then
    print_success "Clippy checks passed (zero warnings)"
else
    print_warning "Clippy found issues. Check /tmp/clippy.log for details"
fi

# 7. Check formatting
print_check "Checking code formatting..."
if cargo fmt --all -- --check 2>&1 | tee /tmp/fmt.log; then
    print_success "Code formatting is correct"
else
    print_warning "Code formatting issues found. Run 'cargo fmt' to fix"
fi

# 8. Build documentation
print_check "Building documentation..."
if cargo doc --workspace --all-features --no-deps 2>&1 | tee /tmp/doc.log; then
    print_success "Documentation built successfully"
else
    print_error "Documentation build failed. Check /tmp/doc.log for details"
fi

# 9. Security audit
print_check "Running security audit..."
if command_exists cargo-audit; then
    if cargo audit 2>&1 | tee /tmp/audit.log; then
        print_success "Security audit passed (no vulnerabilities)"
    else
        print_error "Security vulnerabilities found. Check /tmp/audit.log for details"
    fi
else
    print_warning "cargo-audit not installed. Run: cargo install cargo-audit"
fi

# 10. Check dependencies
print_check "Checking dependencies..."
if cargo tree --workspace --all-features > /tmp/deps.log 2>&1; then
    print_success "Dependency tree generated"
else
    print_warning "Could not generate dependency tree"
fi

# 11. Check for outdated dependencies
if command_exists cargo-outdated; then
    print_check "Checking for outdated dependencies..."
    if cargo outdated --root-deps-only > /tmp/outdated.log 2>&1; then
        OUTDATED_COUNT=$(wc -l < /tmp/outdated.log)
        if [ "$OUTDATED_COUNT" -gt 5 ]; then
            print_warning "$OUTDATED_COUNT outdated dependencies found"
        else
            print_success "Dependencies are up to date"
        fi
    fi
else
    print_warning "cargo-outdated not installed. Run: cargo install cargo-outdated"
fi

# 12. Check binary size
print_check "Checking binary sizes..."
if [ -f "target/release/foundry" ]; then
    BINARY_SIZE=$(du -h target/release/foundry | cut -f1)
    print_success "Binary size: $BINARY_SIZE"
elif [ -f "target/release/foundry.exe" ]; then
    BINARY_SIZE=$(du -h target/release/foundry.exe | cut -f1)
    print_success "Binary size: $BINARY_SIZE"
else
    print_warning "Release binary not found"
fi

# 13. Check for TODO/FIXME comments
print_check "Checking for TODO/FIXME comments..."
TODO_COUNT=$(grep -r "TODO\|FIXME" crates/ --include="*.rs" | wc -l || echo 0)
if [ "$TODO_COUNT" -gt 0 ]; then
    print_warning "Found $TODO_COUNT TODO/FIXME comments in code"
else
    print_success "No TODO/FIXME comments found"
fi

# 14. Check for debug prints
print_check "Checking for debug prints..."
DEBUG_PRINTS=$(grep -r "println!\|dbg!\|eprintln!" crates/ --include="*.rs" | grep -v "test" | wc -l || echo 0)
if [ "$DEBUG_PRINTS" -gt 0 ]; then
    print_warning "Found $DEBUG_PRINTS debug print statements"
else
    print_success "No debug print statements found"
fi

# 15. Check test coverage (if tarpaulin is installed)
if command_exists cargo-tarpaulin; then
    print_check "Generating test coverage report..."
    if cargo tarpaulin --workspace --all-features --out Xml > /tmp/coverage.log 2>&1; then
        print_success "Coverage report generated"
    else
        print_warning "Could not generate coverage report"
    fi
else
    print_warning "cargo-tarpaulin not installed. Install with: cargo install cargo-tarpaulin"
fi

# 16. Check for unsafe code
print_check "Checking for unsafe code..."
UNSAFE_COUNT=$(grep -r "unsafe" crates/ --include="*.rs" | grep -v "test" | wc -l || echo 0)
if [ "$UNSAFE_COUNT" -gt 0 ]; then
    print_warning "Found $UNSAFE_COUNT uses of unsafe code"
else
    print_success "No unsafe code found"
fi

# 17. Verify .env.example exists
print_check "Checking for .env.example..."
if [ -f ".env.example" ]; then
    print_success ".env.example found"
else
    print_warning ".env.example not found"
fi

# 18. Check README
print_check "Checking for README.md..."
if [ -f "README.md" ]; then
    README_SIZE=$(wc -l < README.md)
    if [ "$README_SIZE" -gt 50 ]; then
        print_success "README.md found ($README_SIZE lines)"
    else
        print_warning "README.md is too short ($README_SIZE lines)"
    fi
else
    print_error "README.md not found"
fi

# 19. Check LICENSE
print_check "Checking for LICENSE..."
if [ -f "LICENSE" ] || [ -f "LICENSE-MIT" ] || [ -f "LICENSE-APACHE" ]; then
    print_success "License file(s) found"
else
    print_warning "No license file found"
fi

# 20. Check GitHub Actions workflows
print_check "Checking GitHub Actions workflows..."
if [ -d ".github/workflows" ]; then
    WORKFLOW_COUNT=$(find .github/workflows -name "*.yml" -o -name "*.yaml" | wc -l)
    if [ "$WORKFLOW_COUNT" -gt 0 ]; then
        print_success "Found $WORKFLOW_COUNT GitHub Actions workflows"
    else
        print_warning "No GitHub Actions workflows found"
    fi
else
    print_warning ".github/workflows directory not found"
fi

# 21. Check Docker support
print_check "Checking for Dockerfile..."
if [ -f "Dockerfile" ]; then
    print_success "Dockerfile found"
    if command_exists docker; then
        print_check "Testing Docker build..."
        if docker build -t rustforge-test . > /tmp/docker-build.log 2>&1; then
            print_success "Docker image builds successfully"
            docker rmi rustforge-test > /dev/null 2>&1 || true
        else
            print_warning "Docker build failed. Check /tmp/docker-build.log"
        fi
    else
        print_warning "Docker not installed, skipping build test"
    fi
else
    print_warning "Dockerfile not found"
fi

# 22. Check benchmarks
print_check "Checking for benchmarks..."
if [ -d "benches" ]; then
    BENCH_COUNT=$(find benches -name "*.rs" | wc -l)
    if [ "$BENCH_COUNT" -gt 0 ]; then
        print_success "Found $BENCH_COUNT benchmark files"
    else
        print_warning "No benchmark files found"
    fi
else
    print_warning "benches directory not found"
fi

# Summary
print_header "Verification Summary"
echo -e "${GREEN}Passed:${NC} $PASSED"
echo -e "${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "${RED}Failed:${NC} $FAILED"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ Production verification completed successfully!${NC}"
    echo -e "${GREEN}The framework is ready for production deployment.${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Production verification failed!${NC}"
    echo -e "${RED}Please fix the issues above before deploying to production.${NC}"
    exit 1
fi
