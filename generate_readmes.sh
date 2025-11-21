#!/bin/bash
# Generate README.md files for all RustForge crates

CRATE_DIR="crates"

# Function to extract crate description from Cargo.toml
get_description() {
    local crate_path=$1
    if [ -f "$crate_path/Cargo.toml" ]; then
        grep '^description' "$crate_path/Cargo.toml" | sed 's/description = "\(.*\)"/\1/' | sed 's/"//g'
    fi
}

# Function to get crate name
get_crate_name() {
    local crate_path=$1
    basename "$crate_path"
}

# Function to generate README
generate_readme() {
    local crate_path=$1
    local crate_name=$(get_crate_name "$crate_path")
    local description=$(get_description "$crate_path")

    # If no description, create a generic one based on the crate name
    if [ -z "$description" ]; then
        # Convert hyphenated name to title case description
        description=$(echo "$crate_name" | sed 's/-/ /g' | sed 's/\b\(.\)/\u\1/g')
        description="$description for RustForge"
    fi

    cat > "$crate_path/README.md" << EOF
# $crate_name

$description

## Overview

This crate is part of the RustForge framework, providing essential functionality for building modern web applications in Rust.

## Features

- Type-safe and performant
- Async/await support with Tokio
- Seamless integration with other RustForge components
- Production-ready implementations

## Installation

Add this to your \`Cargo.toml\`:

\`\`\`toml
[dependencies]
$crate_name = { path = "../$crate_name" }
\`\`\`

## Usage

\`\`\`rust
use $crate_name::*;

// Example usage here
\`\`\`

## Documentation

For detailed documentation, run:

\`\`\`bash
cargo doc --package $crate_name --open
\`\`\`

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Contributing

Contributions are welcome! Please see the main RustForge repository for guidelines.

## Part of RustForge

This crate is part of [RustForge](https://github.com/RustForge/RustForge), a comprehensive full-stack application framework for Rust.
EOF

    echo "Generated README for $crate_name"
}

# Main execution
cd "$CRATE_DIR" || exit 1

for dir in */; do
    dir=${dir%/}  # Remove trailing slash
    if [ ! -f "$dir/README.md" ]; then
        generate_readme "$dir"
    fi
done

echo "README generation complete!"
