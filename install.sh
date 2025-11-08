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
TEMPLATE_URL="https://github.com/Chregu12/RustForge-Starter.git"

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
    echo -e "${RED}Error: cargo is not installed${NC}"
    echo "Please install Rust first: https://rustup.rs/"
    exit 1
fi

echo -e "${BLUE}📦 Creating new RustForge project: ${GREEN}$PROJECT_NAME${NC}"
echo ""

# Clone the template
echo -e "${YELLOW}→${NC} Cloning template..."
git clone --quiet "$TEMPLATE_URL" "$PROJECT_NAME" 2>/dev/null || {
    echo -e "${RED}Failed to clone template${NC}"
    exit 1
}

cd "$PROJECT_NAME"

# Remove git history
echo -e "${YELLOW}→${NC} Initializing git repository..."
rm -rf .git
git init --quiet
git add .
git commit --quiet -m "Initial commit: RustForge application"

# Setup environment
echo -e "${YELLOW}→${NC} Setting up environment..."
cp .env.example .env

# Update project name in Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/name = \"rustforge-app\"/name = \"$PROJECT_NAME\"/" Cargo.toml
else
    # Linux
    sed -i "s/name = \"rustforge-app\"/name = \"$PROJECT_NAME\"/" Cargo.toml
fi

echo ""
echo -e "${GREEN}✅ Project created successfully!${NC}"
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  ${GREEN}Next Steps:${BLUE}                                     ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${YELLOW}1.${NC} cd $PROJECT_NAME"
echo -e "  ${YELLOW}2.${NC} cargo build"
echo -e "  ${YELLOW}3.${NC} cargo run"
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  ${GREEN}Project Structure:${BLUE}                              ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════╝${NC}"
echo ""
echo "  $PROJECT_NAME/"
echo "  ├── Cargo.toml       # Dependencies"
echo "  ├── .env             # Configuration"
echo "  ├── src/"
echo "  │   └── main.rs      # Entry point"
echo "  └── README.md        # Documentation"
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  ${GREEN}Available Features:${BLUE}                             ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════╝${NC}"
echo ""
echo "  ✅ Queue System      - Background job processing"
echo "  ✅ Cache System      - Memory & Redis caching"
echo "  ✅ Validation        - 27+ validation rules"
echo "  ✅ Service Container - Dependency injection"
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  ${GREEN}Resources:${BLUE}                                      ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════╝${NC}"
echo ""
echo "  📚 Docs:     https://github.com/Chregu12/RustForge"
echo "  🐛 Issues:   https://github.com/Chregu12/RustForge/issues"
echo ""
echo -e "${GREEN}Happy coding! 🚀${NC}"
echo ""
