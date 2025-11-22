#!/usr/bin/env bash

# RustForge Installer
# Usage: bash <(curl -s https://raw.githubusercontent.com/Chregu12/RustForge/main/install.sh) my-project

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Banner
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                                                   ║${NC}"
echo -e "${BLUE}║         ${GREEN}RustForge Framework Installer${BLUE}           ║${NC}"
echo -e "${BLUE}║         ${YELLOW}Laravel-like Rust Framework${BLUE}             ║${NC}"
echo -e "${BLUE}║                                                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════╝${NC}"
echo ""

# Check if project name is provided
if [ -z "$1" ]; then
    echo -e "${RED}Error: Project name required${NC}"
    echo ""
    echo "Usage:"
    echo "  bash install.sh my-project"
    echo ""
    echo "Or one-liner:"
    echo "  bash <(curl -s https://raw.githubusercontent.com/Chregu12/RustForge/main/install.sh) my-project"
    echo ""
    exit 1
fi

PROJECT_NAME="$1"
REPO_URL="https://github.com/Chregu12/RustForge.git"
TEMP_DIR=$(mktemp -d)

# Check if directory already exists
if [ -d "$PROJECT_NAME" ]; then
    echo -e "${RED}Error: Directory '$PROJECT_NAME' already exists${NC}"
    exit 1
fi

# Check if git is installed
if ! command -v git &> /dev/null; then
    echo -e "${RED}Error: git is not installed${NC}"
    echo "Please install git first: https://git-scm.com/"
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo is not installed${NC}"
    echo "Please install Rust first: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}→ Cloning RustForge repository...${NC}"
git clone --quiet --depth 1 "$REPO_URL" "$TEMP_DIR" 2>&1 | grep -v "Cloning into" || true

if [ ! -d "$TEMP_DIR/starter-template" ]; then
    echo -e "${RED}Error: Starter template not found in repository${NC}"
    rm -rf "$TEMP_DIR"
    exit 1
fi

echo -e "${GREEN}→ Setting up project '$PROJECT_NAME'...${NC}"
cp -r "$TEMP_DIR/starter-template" "$PROJECT_NAME"

# Cleanup
rm -rf "$TEMP_DIR"

# Initialize new git repository
cd "$PROJECT_NAME"
git init --quiet
git add .
git commit --quiet -m "Initial commit from RustForge installer"

# Setup environment
if [ -f ".env.example" ]; then
    cp .env.example .env
    echo -e "${GREEN}→ Created .env file${NC}"
fi

# Update project name in Cargo.toml
if [ "$(uname)" == "Darwin" ]; then
    # macOS
    sed -i '' "s/name = \"rustforge-app\"/name = \"$PROJECT_NAME\"/" Cargo.toml
else
    # Linux
    sed -i "s/name = \"rustforge-app\"/name = \"$PROJECT_NAME\"/" Cargo.toml
fi

echo ""
echo -e "${GREEN}✨ Success! Project '$PROJECT_NAME' created!${NC}"
echo ""
echo "Next steps:"
echo ""
echo -e "  ${BLUE}cd $PROJECT_NAME${NC}"
echo -e "  ${BLUE}cargo run${NC}"
echo ""
echo "Your API will be available at: ${GREEN}http://localhost:3000${NC}"
echo ""
echo "Test it with:"
echo -e "  ${BLUE}curl http://localhost:3000${NC}"
echo -e "  ${BLUE}curl http://localhost:3000/api/posts${NC}"
echo ""
echo -e "Happy coding! 🚀"
echo ""
