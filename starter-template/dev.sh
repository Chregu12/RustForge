#!/bin/bash
# Development helper script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 RustForge Development Helper${NC}"
echo ""

# Check if .env exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}⚠️  .env file not found. Creating from .env.example...${NC}"
    cp .env.example .env
    echo -e "${GREEN}✅ .env file created${NC}"
    echo ""
fi

# Parse command
case "$1" in
    "build")
        echo -e "${GREEN}🔨 Building project...${NC}"
        cargo build
        ;;

    "run")
        echo -e "${GREEN}▶️  Running application...${NC}"
        cargo run
        ;;

    "dev")
        echo -e "${GREEN}👀 Running with auto-reload (requires cargo-watch)${NC}"
        if ! command -v cargo-watch &> /dev/null; then
            echo -e "${YELLOW}Installing cargo-watch...${NC}"
            cargo install cargo-watch
        fi
        cargo watch -x run
        ;;

    "test")
        echo -e "${GREEN}🧪 Running tests...${NC}"
        cargo test -- --nocapture
        ;;

    "clean")
        echo -e "${GREEN}🧹 Cleaning build artifacts...${NC}"
        cargo clean
        rm -f data.db
        echo -e "${GREEN}✅ Cleaned${NC}"
        ;;

    "format")
        echo -e "${GREEN}✨ Formatting code...${NC}"
        cargo fmt
        echo -e "${GREEN}✅ Code formatted${NC}"
        ;;

    "lint")
        echo -e "${GREEN}🔍 Running linter...${NC}"
        cargo clippy -- -D warnings
        ;;

    "check")
        echo -e "${GREEN}✓ Checking code...${NC}"
        cargo check
        ;;

    "release")
        echo -e "${GREEN}📦 Building release binary...${NC}"
        cargo build --release
        echo -e "${GREEN}✅ Release binary: target/release/rustforge-app${NC}"
        ;;

    "db:reset")
        echo -e "${YELLOW}⚠️  Resetting database...${NC}"
        rm -f data.db
        echo -e "${GREEN}✅ Database reset (will be recreated on next run)${NC}"
        ;;

    *)
        echo "Usage: ./dev.sh [command]"
        echo ""
        echo "Commands:"
        echo "  build       - Build the project"
        echo "  run         - Run the application"
        echo "  dev         - Run with auto-reload (cargo-watch)"
        echo "  test        - Run tests"
        echo "  clean       - Clean build artifacts and database"
        echo "  format      - Format code with rustfmt"
        echo "  lint        - Run clippy linter"
        echo "  check       - Quick compilation check"
        echo "  release     - Build optimized release binary"
        echo "  db:reset    - Reset the database"
        echo ""
        ;;
esac
