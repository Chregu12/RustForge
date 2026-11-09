#!/usr/bin/env python3
"""Generate public getting-started snippets from the compiled blog application.

Run with --check in CI. The dependency revision deliberately identifies the
implementation commit, rather than an unpublished or older release tag.
"""
import argparse
import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
FILES = ["README.md", "docs/wiki/Home.md", "docs/wiki/Quick-Start.md", "docs/wiki/Installation.md"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    package = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]
    revision = (ROOT / "docs/snippets/getting-started-revision.txt").read_text().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise SystemExit("getting-started-revision.txt must contain a full implementation commit SHA")
    source = (ROOT / "examples/blog-slice/src/main.rs").read_text()
    example = source.split("// docs:begin\n", 1)[1].split("// docs:end", 1)[0].strip()
    dependency = f'''```toml
[dependencies]
rf = {{ git = "https://github.com/Chregu12/RustForge", rev = "{revision}" }}
axum = "0.8"
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
```'''
    blocks = {
        "example": "```rust\n" + example + "\n```",
        "dependencies": dependency,
        "status": f'Workspace version: **{package["version"]}** (development snapshot). '
                  f'Minimum supported Rust: **{package["rust-version"]}**. '
                  'RustForge is consumed from Git; no crates.io release is required.',
    }
    stale = []
    for filename in FILES:
        path = ROOT / filename
        old = new = path.read_text()
        for name, body in blocks.items():
            start, end = f"<!-- rf-{name}:begin -->", f"<!-- rf-{name}:end -->"
            pattern = re.escape(start) + r".*?" + re.escape(end)
            new = re.sub(pattern, lambda _: f"{start}\n{body}\n{end}", new, flags=re.S)
        if new != old:
            stale.append(filename)
            if not args.check:
                path.write_text(new)
    if args.check and stale:
        print("Generated snippets are stale: " + ", ".join(stale), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
