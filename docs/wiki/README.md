# RustForge Wiki

This directory contains the wiki pages for the RustForge project.
The pages are written in standard Markdown and can be published directly to GitHub Wiki.

---

## Wiki pages

| File | Title | Summary |
|------|-------|---------|
| [Home.md](Home.md) | Home | Framework overview, quick orientation, links to all pages and key repo docs |
| [Installation.md](Installation.md) | Installation | Cargo setup, `forge` CLI install, environment variables |
| [Quick-Start.md](Quick-Start.md) | Quick Start | Step-by-step tutorial: first working app from zero |
| [Laravel-Syntax.md](Laravel-Syntax.md) | Laravel Syntax | DX-layer guide: `Model!`, `validate!`, facades, request globals, `Auth` |
| [Features.md](Features.md) | Features | Full capability list with maturity tags (stable / beta / experimental) |
| [API-Documentation.md](API-Documentation.md) | API Documentation | Per-module API reference tables |
| [Examples.md](Examples.md) | Examples | Annotated walk-throughs of the example apps in `examples/` |
| [Migration-Guide.md](Migration-Guide.md) | Migration Guide | Moving to RustForge from axum, Actix-web, or Rocket |

---

## Key repo docs (not in wiki)

These live under `docs/` and are the authoritative technical references:

| Doc | Purpose |
|-----|---------|
| [docs/STABLE_CORE.md](../STABLE_CORE.md) | Exact v1 API contract — every stable entry point, grep-verified |
| [docs/API_PHILOSOPHY.md](../API_PHILOSOPHY.md) | Two-layer architecture and honest trade-offs |
| [docs/TIERS.md](../TIERS.md) | Complete crate maturity roster (34 stable / 76 beta / 8 experimental) |
| [docs/COOKBOOK.md](../COOKBOOK.md) | Task-oriented recipes with CI-tested snippets |
| [docs/RELEASING.md](../RELEASING.md) | SemVer policy, MSRV, deprecation policy |
| [SECURITY.md](../../SECURITY.md) | Security policy and responsible disclosure |
| [CHANGELOG.md](../../CHANGELOG.md) | Release history |

---

## Publish to GitHub Wiki

### Option 1 — Manual (recommended for first publish)

1. Go to `https://github.com/Chregu12/RustForge/wiki`
2. Click **Create the first page** (or **New Page** for subsequent pages)
3. Use the filename without `.md` as the page title (e.g. `Quick-Start`)
4. Paste the file content and click **Save Page**

### Option 2 — Git clone and push

```bash
# Enable the wiki in repository Settings first, then:
git clone https://github.com/Chregu12/RustForge.wiki.git
cd RustForge.wiki

# Copy wiki pages (excluding this README)
for f in /path/to/RustForge/docs/wiki/*.md; do
    name=$(basename "$f")
    [ "$name" != "README.md" ] && cp "$f" .
done

git add .
git commit -m "Update wiki from docs/wiki/"
git push origin main
```

---

## Contributing

To update a wiki page:

1. Edit the relevant `.md` file in `docs/wiki/`
2. Keep examples grounded in real code — see `docs/STABLE_CORE.md` and the `examples/` directory
3. Update maturity tags if crate tiers change in `docs/TIERS.md`
4. Open a pull request; the wiki is synced after merge

Do not list experimental crates (`rf-nova`, `rf-swagger`, `rf-telescope`, `rf-cms`, `rf-breeze`, `rf-vite`, `rf-livereload`, `rf-nova-macros`) as stable or production-ready. Consult `docs/TIERS.md` for the authoritative tier of every crate.
