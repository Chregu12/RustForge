"""Generate a standalone app and exercise its real CLI and HTTP entry points."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import selectors
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
from urllib.error import HTTPError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
FORGE = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/debug/forge"


def forge(cwd, *args, succeeds=True):
    result = subprocess.run(
        [FORGE, *args], cwd=cwd, capture_output=True, text=True, timeout=900
    )
    assert (result.returncode == 0) == succeeds, result.stdout + result.stderr
    return result.stdout + result.stderr


@contextmanager
def server(cwd, *args):
    with tempfile.TemporaryFile() as log, subprocess.Popen(
        [FORGE, "serve", *args],
        cwd=cwd,
        start_new_session=True,
        stdout=subprocess.PIPE,
        stderr=log,
    ) as process:
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(process.stdout, selectors.EVENT_READ)
                deadline = time.monotonic() + 900
                buffered = b""
                while time.monotonic() < deadline:
                    if not selector.select(timeout=max(0, deadline - time.monotonic())):
                        break
                    chunk = os.read(process.stdout.fileno(), 8192)
                    if not chunk:
                        break
                    buffered += chunk
                    match = re.search(rb"Listening on (http://127\.0\.0\.1:\d+)", buffered)
                    if match:
                        yield match.group(1).decode()
                        return
            log.seek(0)
            raise RuntimeError("Server failed to start:\n" + log.read().decode())
        finally:
            try:
                os.killpg(process.pid, signal.SIGINT)
                process.wait(timeout=10)
            except ProcessLookupError:
                pass
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)


def request(base, method, path, payload=None):
    body = json.dumps(payload).encode() if payload is not None else None
    req = Request(base + path, method=method, data=body, headers={"Content-Type": "application/json"})
    try:
        response = urlopen(req, timeout=5)
    except HTTPError as error:
        response = error
    with response:
        content = response.read()
        if content and response.headers.get_content_type() == "application/json":
            content = json.loads(content)
        return response.status, content


def smoke():
    os.environ.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    # The test must only operate on the disposable project's database.
    for key in ("DATABASE_URL", "APP_ENV", "HOST", "PORT"):
        os.environ.pop(key, None)
    with tempfile.TemporaryDirectory(prefix="rustforge-starter-") as temp:
        parent = Path(temp)
        if os.environ.get("RF_SMOKE_GIT_DEPENDENCY"):
            forge(parent, "new", "blog-smoke")
        else:
            forge(parent, "new", "blog-smoke", "--framework-path", str(ROOT))
        app = parent / "blog-smoke"
        with server(app, "--port", "0") as url:
            assert request(url, "GET", "/up")[0] == 200
            assert request(url, "GET", "/ready")[0] == 503
        assert "Pending" in forge(app, "migrate:status")
        with sqlite3.connect(app / "database/database.sqlite") as db:
            assert db.execute("SELECT name FROM sqlite_schema WHERE name='migrations'").fetchone() is None
        assert "batch 1" in forge(app, "migrate")
        assert "Nothing to migrate" in forge(app, "migrate")
        assert "Ran" in forge(app, "migrate:status")
        assert "posts.sql" in forge(app, "db:seed", "--class", "Posts")
        assert "--force" in forge(app, "migrate:fresh", succeeds=False)
        dotenv = app / ".env"
        dotenv.write_text("APP_ENV=production\n")
        assert "--force" in forge(app, "migrate", succeeds=False)
        dotenv.unlink()

        forge(app, "make:model", "Topic", "--migration")
        forge(app, "make:migration", "create_comments_table")
        pending = forge(app, "migrate", "--step")
        assert "batch 2" in pending and "batch 3" in pending
        assert (app / "app/models/topic.rs").is_file()
        with sqlite3.connect(app / "database/database.sqlite") as db:
            migration_names = dict(db.execute(
                "SELECT batch, migration FROM migrations WHERE batch > 1"
            ))
        assert migration_names[3] in forge(app, "migrate:rollback", "--step", "1")
        assert migration_names[2] in forge(app, "migrate:rollback", "--batch", "2")
        with sqlite3.connect(app / "database/database.sqlite") as db:
            assert db.execute("SELECT count(*) FROM posts").fetchone() == (1,)
            assert db.execute("SELECT name FROM sqlite_schema WHERE name='topics'").fetchone() is None

        routes = forge(app, "route:list", "--method", "patch", "--path", "posts")
        assert "PATCH    /api/posts/{post}" in routes and "GET " not in routes

        with server(app, "--port", "0") as url:
            assert request(url, "GET", "/up")[0] == 200
            assert request(url, "GET", "/ready")[0] == 200
            status, html = request(url, "GET", "/")
            assert status == 200 and b"RustForge" in html
            status, posts = request(url, "GET", "/api/posts")
            assert status == 200 and len(posts) == 1
            assert request(url, "POST", "/api/posts", {"title": ""})[0] == 422
            status, post = request(url, "POST", "/api/posts", {
                "title": "Example", "body": "Persistent", "admin": True
            })
            assert status == 201 and "admin" not in post
            path = f"/api/posts/{post['id']}"
            assert request(url, "GET", path) == (200, post)
            assert request(url, "PATCH", path, {"title": None})[0] == 422
            status, updated = request(url, "PATCH", path, {"title": "Edited"})
            assert status == 200 and updated["title"] == "Edited" and updated["body"] == "Persistent"
            status, updated = request(url, "PUT", path, {"title": "Replaced", "body": "Full"})
            assert status == 200 and updated["body"] == "Full"

        dotenv.write_text("PORT=0\n")
        with server(app) as url:
            assert request(url, "GET", path) == (200, updated)
            assert request(url, "DELETE", path)[0] == 204
            assert request(url, "GET", path)[0] == 404
        dotenv.unlink()

        forge(app, "migrate:reset")
        assert "Ran" not in forge(app, "migrate:status")
        forge(app, "migrate:refresh", "--seed")
        with sqlite3.connect(app / "database/database.sqlite") as db:
            db.execute("CREATE TABLE unmanaged (id INTEGER)")
        forge(app, "migrate:fresh", "--force", "--seed")
        with sqlite3.connect(app / "database/database.sqlite") as db:
            assert db.execute("SELECT count(*) FROM posts").fetchone() == (1,)
            assert db.execute("SELECT name FROM sqlite_schema WHERE name='topics'").fetchone() == ("topics",)
            assert db.execute("SELECT name FROM sqlite_schema WHERE name='unmanaged'").fetchone() is None
    print("Generated application passed CLI, migration, HTTP CRUD, validation and restart checks.")


def postgres_smoke(database_url):
    target = urlparse(database_url)
    assert target.hostname in {"localhost", "127.0.0.1", "::1"}
    assert target.path == "/rustforge_starter", "Use a fresh local rustforge_starter database"
    os.environ.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    os.environ["DATABASE_URL"] = database_url
    os.environ["APP_ENV"] = "production"
    with tempfile.TemporaryDirectory(prefix="rustforge-postgres-starter-") as temp:
        parent = Path(temp)
        if os.environ.get("RF_SMOKE_GIT_DEPENDENCY"):
            forge(parent, "new", "postgres-smoke")
        else:
            forge(parent, "new", "postgres-smoke", "--framework-path", str(ROOT))
        app = parent / "postgres-smoke"
        assert "Pending" in forge(app, "migrate:status")
        assert "batch 1" in forge(app, "migrate", "--force")
        assert "posts.sql" in forge(app, "db:seed", "--force")
        forge(app, "make:model", "Topic", "--migration")
        assert "batch 2" in forge(app, "migrate", "--force")
        assert "topics" in forge(app, "migrate:rollback", "--force")

        with server(app, "--host", "127.0.0.1", "--port", "0") as url:
            assert request(url, "GET", "/ready")[0] == 200
            status, posts = request(url, "GET", "/api/posts")
            assert status == 200 and len(posts) == 1
            status, post = request(url, "POST", "/api/posts", {"title": "Postgres", "body": "Ready"})
            assert status == 201
            path = f"/api/posts/{post['id']}"
            status, updated = request(url, "PATCH", path, {"body": "Persisted"})
            assert status == 200 and updated["body"] == "Persisted"

        with server(app, "--host", "127.0.0.1", "--port", "0") as url:
            assert request(url, "GET", path) == (200, updated)
            assert request(url, "DELETE", path)[0] == 204
            assert request(url, "GET", path)[0] == 404

        assert "posts" in forge(app, "migrate:rollback", "--force")
        with server(app, "--host", "127.0.0.1", "--port", "0") as url:
            assert request(url, "GET", "/up")[0] == 200
            assert request(url, "GET", "/ready")[0] == 503
    print("Generated application passed PostgreSQL migration, HTTP and readiness checks.")


if __name__ == "__main__":
    if postgres_url := os.environ.get("RF_SMOKE_POSTGRES_URL"):
        postgres_smoke(postgres_url)
    else:
        smoke()
