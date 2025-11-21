#!/bin/bash
# Generate prelude modules for major RustForge crates

# Major crates that should have preludes
MAJOR_CRATES=(
    "rf-validation"
    "rf-mail"
    "rf-jobs"
    "rf-cache"
    "rf-queue"
    "rf-auth"
    "rf-orm"
    "rf-web"
    "rf-broadcasting"
    "rf-storage"
    "rf-events"
   "rf-sanctum"
)

generate_prelude() {
    local crate_name=$1
    local crate_path="crates/$crate_name"
    local lib_file="$crate_path/src/lib.rs"
    local prelude_file="$crate_path/src/prelude.rs"

    # Skip if prelude already exists
    if [ -f "$prelude_file" ]; then
        echo "Skipping $crate_name - prelude already exists"
        return
    fi

    # Skip if lib.rs doesn't exist
    if [ ! -f "$lib_file" ]; then
        echo "Skipping $crate_name - lib.rs not found"
        return
    fi

    # Extract pub use statements from lib.rs
    local exports=$(grep '^pub use' "$lib_file" | head -10)

    # Create prelude file
    cat > "$prelude_file" << EOF
//! # $crate_name Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from $crate_name.
//!
//! ## Usage
//!
//! \`\`\`rust
//! use ${crate_name//-/_}::prelude::*;
//! \`\`\`

// Re-export commonly used items
$(echo "$exports" | sed 's/^pub use/pub use crate::/' | sed 's/;$/;/' || echo "// Add common re-exports here")
EOF

    echo "Generated prelude for $crate_name"
}

# Generate preludes for all major crates
for crate in "${MAJOR_CRATES[@]}"; do
    generate_prelude "$crate"
done

echo "Prelude generation complete!"
