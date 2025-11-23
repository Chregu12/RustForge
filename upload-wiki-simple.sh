#!/bin/bash
set -e

echo "📚 Uploading RustForge Wiki Pages..."

cd /tmp
rm -rf RustForge.wiki
git clone https://github.com/Chregu12/RustForge.wiki.git
cd RustForge.wiki

# Copy all wiki files (except README)
cp /Users/christian/Developer/Github_Projekte/Rust_DX-Framework/docs/wiki/*.md .
rm -f README.md

# Commit and push
git add .
git commit -m "Add comprehensive wiki documentation (Installation, Quick Start, Features, API Docs, Examples, Migration Guide)"
git push origin master 2>/dev/null || git push origin main

echo "✅ Wiki uploaded successfully!"
echo "🌐 View at: https://github.com/Chregu12/RustForge/wiki"

# Cleanup
cd /tmp
rm -rf RustForge.wiki
