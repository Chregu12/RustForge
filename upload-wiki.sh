#!/bin/bash

# RustForge Wiki Upload Script
# This script uploads all wiki pages to GitHub

echo "=== RustForge Wiki Upload ==="
echo ""
echo "This script will help you upload the wiki pages to GitHub."
echo ""

# Step 1: Create first wiki page manually
echo "STEP 1: Create the first wiki page on GitHub"
echo "----------------------------------------"
echo "1. Open: https://github.com/Chregu12/RustForge/wiki"
echo "2. Click 'Create the first page'"
echo "3. Title: Home"
echo "4. Content: (paste the content from docs/wiki/Home.md)"
echo "5. Click 'Save Page'"
echo ""
read -p "Have you created the first page? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    echo "Please create the first wiki page first, then run this script again."
    exit 1
fi

# Step 2: Clone wiki repository
echo ""
echo "STEP 2: Cloning wiki repository..."
echo "----------------------------------------"

cd /Users/christian/Developer/Github_Projekte
rm -rf RustForge.wiki-temp
git clone https://github.com/Chregu12/RustForge.wiki.git RustForge.wiki-temp

if [ $? -ne 0 ]; then
    echo "Error: Failed to clone wiki repository"
    echo "Make sure you created the first page on GitHub."
    exit 1
fi

# Step 3: Copy wiki files
echo ""
echo "STEP 3: Copying wiki files..."
echo "----------------------------------------"

cd RustForge.wiki-temp
cp /Users/christian/Developer/Github_Projekte/Rust_DX-Framework/docs/wiki/*.md .
rm -f README.md  # Don't upload README to wiki

# Step 4: Commit and push
echo ""
echo "STEP 4: Uploading to GitHub..."
echo "----------------------------------------"

git add .
git commit -m "Add comprehensive RustForge wiki documentation

- Installation guide
- Quick start tutorial
- Complete features documentation
- API documentation
- Code examples
- Migration guides"

git push origin master || git push origin main

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ SUCCESS! Wiki pages uploaded successfully!"
    echo ""
    echo "View your wiki at: https://github.com/Chregu12/RustForge/wiki"
    echo ""
else
    echo ""
    echo "❌ Error: Failed to push to GitHub"
    echo "Please check your authentication and try again."
    exit 1
fi

# Cleanup
cd ..
rm -rf RustForge.wiki-temp

echo "Done!"
