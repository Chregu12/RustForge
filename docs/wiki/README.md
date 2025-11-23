# RustForge Wiki

This directory contains all wiki pages for the RustForge project.

## Wiki Pages

- **[Home](Home.md)** - Main wiki page
- **[Installation](Installation.md)** - Installation guide
- **[Quick-Start](Quick-Start.md)** - Quick start tutorial
- **[Features](Features.md)** - Complete feature overview
- **[API-Documentation](API-Documentation.md)** - Detailed API documentation
- **[Examples](Examples.md)** - Code examples
- **[Migration-Guide](Migration-Guide.md)** - Migration guide from other frameworks

## How to Set Up GitHub Wiki

To publish these pages to GitHub Wiki:

### Option 1: Manual Upload (Recommended)

1. Go to https://github.com/Chregu12/RustForge/wiki
2. Click "Create the first page" button
3. For each markdown file in this directory:
   - Click "New Page" in the wiki
   - Copy the filename (without .md extension) as the page title
   - Copy the content from the corresponding .md file
   - Click "Save Page"

### Option 2: Git Clone and Push

1. Enable the wiki in your repository settings
2. Clone the wiki repository:
   ```bash
   git clone https://github.com/Chregu12/RustForge.wiki.git
   cd RustForge.wiki
   ```

3. Copy all .md files from this directory to the wiki directory:
   ```bash
   cp /path/to/Rust_DX-Framework/docs/wiki/*.md .
   ```

4. Commit and push:
   ```bash
   git add .
   git commit -m "Add comprehensive wiki documentation"
   git push origin main
   ```

### Option 3: Script Automation

Use the provided script to automatically upload all wiki pages:

```bash
cd RustForge.wiki
for file in ../Rust_DX-Framework/docs/wiki/*.md; do
  if [ "$file" != "README.md" ]; then
    filename=$(basename "$file")
    cp "$file" .
    git add "$filename"
  fi
done
git commit -m "Add all wiki pages"
git push origin main
```

## Wiki Structure

The wiki provides comprehensive documentation covering:

- **Getting Started**: Installation and quick start guides
- **Features**: Complete feature documentation with examples
- **API Reference**: Detailed API documentation for all modules
- **Examples**: Practical code examples for common use cases
- **Migration**: Guides for migrating from Laravel, Actix-web, Rocket, and Axum

## Contributing

To update wiki pages:

1. Edit the markdown files in this directory
2. Test locally (they're standard markdown)
3. Update the wiki repository following the steps above

## Links

- **Repository**: https://github.com/Chregu12/RustForge
- **Wiki**: https://github.com/Chregu12/RustForge/wiki
- **Issues**: https://github.com/Chregu12/RustForge/issues
