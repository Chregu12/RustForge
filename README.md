# RustForge

**A Rust web framework inspired by Laravel**

RustForge bringt bekannte Laravel-Patterns nach Rust. Das Ziel: eine vertraute API für Laravel-Entwickler, kombiniert mit Rusts Typsicherheit und Performance.

> **Status**: Work in Progress - Die Kernfunktionalität ist implementiert, aber das Projekt befindet sich noch in aktiver Entwicklung.

```rust
use rf::prelude::*;

async fn index() -> Response {
    let users = User::where("active", true).get().await;
    Response::json(users)
}
```

[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)]()
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)]()

---

## Was ist RustForge?

RustForge ist ein Full-Stack Web-Framework für Rust, das sich an Laravels API und Konzepten orientiert:

- **Laravel-ähnliche API** - Routing, ORM, Auth, Validation, Queues, etc.
- **Typsicherheit** - Rusts Compiler fängt viele Fehler zur Compile-Zeit ab
- **Async-first** - Aufgebaut auf Tokio und SeaORM
- **Modularer Aufbau** - Einzelne Crates für verschiedene Funktionsbereiche

---

## Features

### Implementierte Kernfunktionalität

**Framework-Basis**
- Routing (RESTful, Gruppen, Middleware, Parameter-Constraints)
- Service Container mit Dependency Injection
- Middleware-Pipeline
- Konfiguration (Environment-basiert)

**Datenbank & ORM**
- Query Builder
- Eloquent-inspiriertes ORM (Active Record, Relationships, Soft Deletes)
- Model Observers
- Global Query Scopes
- Polymorphe Relationen
- Custom Attribute Casters
- Migrationen & Seeders

**Authentifizierung & Autorisierung**
- Multi-Guard Auth (JWT, Session)
- Gates & Policies
- Passwort-Reset Flow
- Email-Verifizierung
- API Tokens

**Background Processing**
- Queue-System (Redis, Database, SQS)
- Job Batching
- Task Scheduler

**Weitere Features**
- Validierung (50+ Regeln, Form Requests)
- Cache (Redis, Memcached, File, Memory)
- Mail-System (SMTP, SES, Mailgun, Postmark) mit Driver Manager
- Notifications (Email, SMS, Slack)
- Broadcasting (WebSocket, Redis)
- File Storage (Local, S3)
- Collection-Klasse mit Pagination
- Rate Limiting
- Internationalisierung
- Search/ORM-Integration (MeiliSearch, Algolia, Elasticsearch)
- SSR mit Inertia
- Fluent Relationship Syntax

---

## Quick Start

### Voraussetzungen

- Rust 1.75+
- PostgreSQL, MySQL oder SQLite

### Installation

Wie bei Laravel (`laravel new app`) erzeugst du ein **neues Projekt**, ohne das
Framework selbst auszuchecken.

**Option A — `forge`-CLI (empfohlen, wie `laravel new`):**

```bash
# Einmalig: die CLI installieren (Binary heisst `forge`)
cargo install --git https://github.com/Chregu12/RustForge forge-cli

# Neues Projekt erzeugen – funktioniert aus jedem Verzeichnis
forge new my-app
cd my-app
forge serve            # Dev-Server (Artisan-Äquivalent: forge migrate, forge make:model, …)
```

**Option B — Installer-One-Liner (wie `laravel.build`):**

```bash
bash <(curl -s https://raw.githubusercontent.com/Chregu12/RustForge/main/install.sh) my-app
cd my-app
forge serve
```

> Das Starter-Template ist in die `forge`-Binary eingebettet – `forge new`
> braucht weder einen Repo-Checkout noch Netzwerkzugriff.

**Am Framework selbst mitentwickeln?** Nur dann das Repo klonen:

```bash
git clone https://github.com/Chregu12/RustForge.git
cd RustForge
cargo build
```

### Beispiel: Einfache REST API

```rust
use rustforge::*;

rustforge! {
    Model!(Post: title, content, user_id);

    async fn index() -> Response {
        let posts = Post::where("published", true)
            .orderBy("created_at", "desc")
            .get();
        Response::json(posts)
    }

    async fn store(data: Json<Value>) -> Response {
        let post = Post::create(data.0);
        Response::json(post).status(201)
    }
}

Route::get("/posts", index);
Route::post("/posts", store);
```

---

## Syntax-Highlights

### `rustforge!` Makro

Das `rustforge!`-Makro vereinfacht den Code:
- Automatische Imports
- `.await` wird automatisch eingefügt
- `where` funktioniert wie in Laravel (kein `r#where` nötig)

### Blade-ähnliche Templates

```rust
let html = blade! {
    <div class="container">
        @if let Some(user) = user {
            <h1>Welcome, {{ user.name }}!</h1>
        }
        @foreach post in posts {
            <li>{{ post.title }}</li>
        }
    </div>
};
```

### Helper-Makros

| Makro | Beispiel |
|-------|----------|
| `now!` | `now!()`, `now!("%Y-%m-%d")` |
| `bcrypt!` | `bcrypt!(password)` |
| `view!` | `view!("welcome", data)` |
| `redirect!` | `redirect!("/home")` |
| `cache!` | `cache!("key")` |
| `abort!` | `abort!(404)` |

---

## Architektur

```
┌──────────────────────────────────────────┐
│           Application Layer              │
│    (Controllers, Jobs, Events)           │
├──────────────────────────────────────────┤
│            Framework Layer               │
│  (Routing, Auth, ORM, Queue, Cache)      │
├──────────────────────────────────────────┤
│          Infrastructure Layer            │
│    (Database, Redis, S3, SMTP)           │
├──────────────────────────────────────────┤
│            Core Libraries                │
│     (Tokio, SeaORM, Redis, AWS SDK)      │
└──────────────────────────────────────────┘
```

### Tech Stack

- **Runtime**: Tokio
- **ORM**: SeaORM
- **Datenbanken**: PostgreSQL, MySQL, SQLite
- **Cache**: Redis, Memcached
- **Queue**: Redis, SQS, Database
- **Storage**: Local, AWS S3

---

## Dokumentation

Weitere Dokumentation im [Wiki](https://github.com/Chregu12/RustForge/wiki):

- [Installation](https://github.com/Chregu12/RustForge/wiki/Installation)
- [Quick Start](https://github.com/Chregu12/RustForge/wiki/Quick-Start)
- [Features](https://github.com/Chregu12/RustForge/wiki/Features)
- [API-Referenz](https://github.com/Chregu12/RustForge/wiki/API-Documentation)
- [Beispiele](https://github.com/Chregu12/RustForge/wiki/Examples)
- [Migration von Laravel](https://github.com/Chregu12/RustForge/wiki/Migration-Guide)

---

## Contributing

1. Fork erstellen
2. Feature-Branch anlegen: `git checkout -b feature/mein-feature`
3. Tests schreiben und `cargo test` ausführen
4. `cargo fmt` und `cargo clippy` laufen lassen
5. Pull Request öffnen

```bash
# Entwicklungssetup
git clone https://github.com/Chregu12/RustForge.git
cd RustForge
cargo build
cargo test
```

---

## Lizenz

MIT OR Apache-2.0

---

## Danksagungen

- [Laravel](https://laravel.com/) - Inspiration für API-Design und Konzepte
- [Tokio](https://tokio.rs/) - Async Runtime
- [SeaORM](https://www.sea-ql.org/SeaORM/) - ORM
- Die Rust-Community
