#!/bin/bash

#
# Performance Profiling Script for RustForge
#
# This script provides various profiling options for analyzing performance:
# - CPU profiling with flamegraphs
# - Memory profiling with heaptrack
# - Benchmark execution
# - Performance regression detection
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
PROFILE_TYPE="cpu"
OUTPUT_DIR="./target/profiling"
DURATION="30"
BENCHMARK=""

# Print colored message
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

# Print usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Performance profiling script for RustForge framework

OPTIONS:
    -t, --type TYPE          Profiling type: cpu, memory, bench, all (default: cpu)
    -o, --output DIR         Output directory (default: ./target/profiling)
    -d, --duration SECONDS   Profiling duration in seconds (default: 30)
    -b, --benchmark NAME     Run specific benchmark
    -h, --help              Show this help message

EXAMPLES:
    # CPU profiling with flamegraph
    $0 --type cpu --duration 60

    # Memory profiling
    $0 --type memory

    # Run all benchmarks
    $0 --type bench

    # Run specific benchmark
    $0 --type bench --benchmark orm_benchmarks

    # Run all profiling types
    $0 --type all

EOF
    exit 0
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--type)
            PROFILE_TYPE="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -d|--duration)
            DURATION="$2"
            shift 2
            ;;
        -b|--benchmark)
            BENCHMARK="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if cargo-flamegraph is installed
check_flamegraph() {
    if ! command -v cargo-flamegraph &> /dev/null; then
        print_msg "$YELLOW" "cargo-flamegraph not found. Installing..."
        cargo install flamegraph
    fi
}

# Check if heaptrack is installed
check_heaptrack() {
    if ! command -v heaptrack &> /dev/null; then
        print_msg "$YELLOW" "heaptrack not found. Please install it:"
        print_msg "$YELLOW" "  macOS: brew install heaptrack"
        print_msg "$YELLOW" "  Linux: sudo apt-get install heaptrack"
        exit 1
    fi
}

# CPU profiling with flamegraph
profile_cpu() {
    print_msg "$GREEN" "Starting CPU profiling (${DURATION}s)..."
    check_flamegraph

    local output_file="$OUTPUT_DIR/flamegraph-$(date +%Y%m%d-%H%M%S).svg"

    # Build in release mode first
    print_msg "$YELLOW" "Building in release mode..."
    cargo build --release

    # Generate flamegraph
    print_msg "$YELLOW" "Generating flamegraph..."
    CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph \
        --output="$output_file" \
        -- \
        sleep "$DURATION"

    print_msg "$GREEN" "Flamegraph generated: $output_file"
    print_msg "$YELLOW" "Open with: open $output_file (macOS) or xdg-open $output_file (Linux)"
}

# Memory profiling
profile_memory() {
    print_msg "$GREEN" "Starting memory profiling..."
    check_heaptrack

    local output_file="$OUTPUT_DIR/heaptrack-$(date +%Y%m%d-%H%M%S)"

    # Build in release mode
    print_msg "$YELLOW" "Building in release mode..."
    cargo build --release

    # Run heaptrack
    print_msg "$YELLOW" "Running heaptrack..."
    heaptrack --output "$output_file" \
        ./target/release/framework-test || true

    print_msg "$GREEN" "Memory profile generated: ${output_file}.gz"
    print_msg "$YELLOW" "Analyze with: heaptrack_gui ${output_file}.gz"
}

# Run benchmarks
profile_benchmarks() {
    print_msg "$GREEN" "Running benchmarks..."

    if [ -n "$BENCHMARK" ]; then
        print_msg "$YELLOW" "Running benchmark: $BENCHMARK"
        cargo bench --bench "$BENCHMARK" -- --save-baseline baseline
    else
        print_msg "$YELLOW" "Running all benchmarks..."
        cargo bench -- --save-baseline baseline
    fi

    print_msg "$GREEN" "Benchmarks complete!"
    print_msg "$YELLOW" "Results saved in: ./target/criterion"
}

# Performance regression check
check_regression() {
    print_msg "$GREEN" "Checking for performance regressions..."

    if [ ! -d "./target/criterion" ]; then
        print_msg "$RED" "No baseline found. Run benchmarks first with:"
        print_msg "$YELLOW" "  $0 --type bench"
        exit 1
    fi

    cargo bench -- --baseline baseline --save-baseline new

    print_msg "$GREEN" "Regression check complete!"
    print_msg "$YELLOW" "Compare with: cargo criterion --baseline baseline --compare new"
}

# System information
print_system_info() {
    print_msg "$GREEN" "=== System Information ==="
    echo "OS: $(uname -s)"
    echo "Kernel: $(uname -r)"
    echo "Architecture: $(uname -m)"

    if command -v nproc &> /dev/null; then
        echo "CPU Cores: $(nproc)"
    elif command -v sysctl &> /dev/null; then
        echo "CPU Cores: $(sysctl -n hw.ncpu)"
    fi

    if command -v free &> /dev/null; then
        echo "Memory: $(free -h | awk '/^Mem:/ {print $2}')"
    elif command -v sysctl &> /dev/null; then
        echo "Memory: $(sysctl -n hw.memsize | awk '{print $1/1024/1024/1024 " GB"}')"
    fi

    echo "Rust Version: $(rustc --version)"
    echo "Cargo Version: $(cargo --version)"
    echo ""
}

# Main execution
main() {
    print_system_info

    case $PROFILE_TYPE in
        cpu)
            profile_cpu
            ;;
        memory)
            profile_memory
            ;;
        bench)
            profile_benchmarks
            ;;
        regression)
            check_regression
            ;;
        all)
            profile_cpu
            profile_memory
            profile_benchmarks
            ;;
        *)
            print_msg "$RED" "Unknown profile type: $PROFILE_TYPE"
            usage
            ;;
    esac

    print_msg "$GREEN" "✓ Profiling complete!"
}

main
