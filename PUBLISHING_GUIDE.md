# RustForge Framework - Publishing & Distribution Guide

## 📦 Strategien zur Veröffentlichung und Nutzung

Dieses Dokument beschreibt verschiedene Wege, wie das RustForge Framework veröffentlicht und in Projekten verwendet werden kann.

---

## 🎯 Empfohlene Publishing-Strategie (3-stufig)

### 1️⃣ **Crates.io Publishing** (Offizielles Rust Package Registry)
### 2️⃣ **GitHub Template Repository** (Schnellstart für neue Projekte)
### 3️⃣ **CLI Scaffolding Tool** (Laravel Artisan-Style)

---

## 📚 Option 1: Crates.io Publishing (EMPFOHLEN)

### Was ist Crates.io?
- Offizielles Package Registry für Rust
- Über 100.000 Packages verfügbar
- Automatische Dokumentation auf docs.rs
- Einfache Integration via `Cargo.toml`

### Vorbereitung

```bash
# 1. Crates.io Account erstellen (falls noch nicht vorhanden)
# https://crates.io/

# 2. API Token generieren
cargo login

# 3. Metadata in Cargo.toml prüfen (bereits vorhanden)
# - name, version, authors, description, license, repository
```

### Publishing Workflow

```bash
# 1. Alle Crates checken und builden
cargo build --release --workspace

# 2. Tests ausführen
cargo test --workspace

# 3. Crates einzeln veröffentlichen (Reihenfolge wichtig!)

# Basis-Crates zuerst:
cargo publish -p foundry-config
cargo publish -p foundry-console
cargo publish -p foundry-env

# Service Layer:
cargo publish -p foundry-service-container
cargo publish -p foundry-domain
cargo publish -p foundry-infra

# Feature Crates:
cargo publish -p foundry-cache
cargo publish -p foundry-queue
cargo publish -p foundry-forms
cargo publish -p foundry-oauth
cargo publish -p foundry-mail
# ... (weitere Crates)

# Hauptcrate zuletzt:
cargo publish -p foundry-application
```

### Nutzung in Projekten

```toml
# Cargo.toml eines neuen Projekts
[dependencies]
foundry-application = "0.2.0"
foundry-cache = "0.2.0"
foundry-queue = "0.2.0"
foundry-forms = "0.2.0"
foundry-oauth = "0.2.0"

tokio = { version = "1", features = ["full"] }
```

```rust
// main.rs
use foundry_application::FoundryApplication;
use foundry_queue::QueueManager;
use foundry_cache::CacheManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = FoundryApplication::new()?;

    // Queue System
    let queue = QueueManager::from_env()?;

    // Cache System
    let cache = CacheManager::from_env()?;

    // App starten
    app.run().await
}
```

### Vorteile ✅
- ✅ Standard-Weg in Rust Ecosystem
- ✅ Automatische Docs auf docs.rs
- ✅ Versionierung und Dependency Management
- ✅ Einfache Updates: `cargo update`
- ✅ Keine Vendor Lock-in
- ✅ Community Discovery

### Nachteile ❌
- ❌ Crate-Namen müssen unique sein (evtl. `rustforge-*` statt `foundry-*`)
- ❌ Kann nicht zurückgezogen werden (nur yanked)
- ❌ Initiales Setup aufwändig für viele Crates

---

## 🚀 Option 2: GitHub Template Repository

### Was ist ein Template Repository?
- GitHub Feature zum Klonen von Repository-Strukturen
- Perfekt für Projekt-Starter
- Ein-Klick Setup für neue Projekte

### Setup

```bash
# 1. Neues Repository erstellen
cd /Users/christian/Developer/Github_Projekte
mkdir RustForge-Template
cd RustForge-Template

# 2. Basis-Projektstruktur erstellen
cargo new . --name my-app

# 3. Template-Dateien hinzufügen
```

### Template-Struktur

```
RustForge-Template/
├── Cargo.toml           # Mit allen foundry-* dependencies
├── .env.example         # Environment variables template
├── src/
│   ├── main.rs          # Entry point
│   ├── commands/        # Custom commands
│   ├── middleware/      # Custom middleware
│   └── routes.rs        # Routes definition
├── config/
│   ├── app.toml
│   ├── database.toml
│   └── cache.toml
├── database/
│   └── migrations/
├── tests/
│   └── integration_test.rs
└── README.md            # Quick Start Guide
```

### Cargo.toml Template

```toml
[package]
name = "my-rustforge-app"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge Framework (via path or git for now, crates.io später)
foundry-application = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-queue = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-cache = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-forms = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }

# Oder wenn auf crates.io veröffentlicht:
# foundry-application = "0.2.0"
# foundry-queue = "0.2.0"
# foundry-cache = "0.2.0"

tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

### Nutzung

```bash
# 1. Template nutzen (GitHub Web Interface)
# - Gehe zu https://github.com/Chregu12/RustForge-Template
# - Klicke "Use this template"
# - Neues Repository erstellen

# 2. Oder via CLI
git clone https://github.com/Chregu12/RustForge-Template.git my-new-app
cd my-new-app
rm -rf .git
git init

# 3. Dependencies installieren
cargo build

# 4. App starten
cargo run
```

### Vorteile ✅
- ✅ Schnellster Weg für neue Projekte
- ✅ Best Practices vorgegeben
- ✅ Vollständige Projektstruktur
- ✅ Kein Setup nötig
- ✅ Kann jederzeit aktualisiert werden

### Nachteile ❌
- ❌ Manuelles Update des Templates
- ❌ User müssen Git verwenden
- ❌ Weniger flexibel als CLI Tool

---

## 🛠️ Option 3: CLI Scaffolding Tool (wie Laravel Artisan)

### Konzept: `cargo-foundry` CLI Tool

Ein CLI Tool das neue RustForge Projekte erstellt, ähnlich wie:
- `cargo new` (Rust)
- `create-react-app` (React)
- `laravel new` (Laravel)

### Installation

```bash
# Via cargo install (wenn auf crates.io)
cargo install cargo-foundry

# Oder via git
cargo install --git https://github.com/Chregu12/RustForge-CLI.git
```

### Nutzung

```bash
# Neues Projekt erstellen
cargo foundry new my-app

# Mit Features
cargo foundry new my-app --features queue,cache,auth

# Mit Template
cargo foundry new my-app --template api

# Projekt-Struktur:
my-app/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs
│   ├── commands/
│   ├── models/
│   └── routes.rs
├── config/
├── database/
└── tests/

# In Projekt Commands ausführen
cd my-app
cargo foundry make:model User
cargo foundry make:command SendEmails
cargo foundry make:middleware RateLimit
cargo foundry migrate
```

### CLI Features

```bash
# Projekt-Management
cargo foundry new <name>              # Neues Projekt
cargo foundry init                    # Existierendes Projekt initialisieren

# Code-Generierung
cargo foundry make:model <name>       # Model erstellen
cargo foundry make:command <name>     # Command erstellen
cargo foundry make:middleware <name>  # Middleware erstellen
cargo foundry make:controller <name>  # Controller erstellen
cargo foundry make:migration <name>   # Migration erstellen

# Datenbank
cargo foundry migrate                 # Migrations ausführen
cargo foundry migrate:rollback        # Rollback
cargo foundry migrate:fresh           # Drop + migrate

# Server
cargo foundry serve                   # Dev server starten
cargo foundry queue:work              # Queue worker starten

# Info
cargo foundry list                    # Alle commands
cargo foundry --version               # Version
```

### Implementierung

```rust
// cargo-foundry CLI Tool (Beispiel)
// crates/cargo-foundry/src/main.rs

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "cargo-foundry")]
#[command(bin_name = "cargo-foundry")]
#[command(about = "RustForge Framework CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new RustForge project
    New {
        /// Project name
        name: String,

        /// Features to include
        #[arg(long, value_delimiter = ',')]
        features: Option<Vec<String>>,
    },

    /// Make a new component
    Make {
        #[command(subcommand)]
        component: MakeCommands,
    },

    /// Run database migrations
    Migrate {
        #[arg(long)]
        rollback: bool,
    },
}

#[derive(Subcommand)]
enum MakeCommands {
    Model { name: String },
    Command { name: String },
    Middleware { name: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, features } => {
            create_new_project(&name, features)?;
        },
        Commands::Make { component } => {
            generate_component(component)?;
        },
        Commands::Migrate { rollback } => {
            run_migrations(rollback)?;
        },
    }

    Ok(())
}
```

### Vorteile ✅
- ✅ Beste Developer Experience
- ✅ Code-Generierung (Models, Commands, etc.)
- ✅ Konsistente Projekt-Struktur
- ✅ Laravel-ähnliche DX
- ✅ Flexibel und erweiterbar

### Nachteile ❌
- ❌ Zusätzlicher Wartungsaufwand
- ❌ Separates Tool muss gepflegt werden
- ❌ Mehr Komplexität

---

## 🔄 Option 4: Git Submodules / Workspace

### Konzept
Framework als Git Submodule in Projekte einbinden.

```bash
# In neuem Projekt
git init
git submodule add https://github.com/Chregu12/RustForge.git framework

# Cargo.toml
[dependencies]
foundry-application = { path = "framework/crates/foundry-application" }
foundry-queue = { path = "framework/crates/foundry-queue" }
```

### Vorteile ✅
- ✅ Direkte Framework-Source
- ✅ Einfaches Debuggen
- ✅ Lokale Änderungen möglich

### Nachteile ❌
- ❌ Kompliziertes Git Workflow
- ❌ Große Repository-Größe
- ❌ Updates umständlich
- ❌ Nicht empfohlen für Production

---

## 📋 Empfohlener Workflow für RustForge

### Phase 1: MVP (Jetzt) ✅
```bash
# Git Tags für Versionen
git tag v0.2.0
git push origin v0.2.0

# Nutzung via Git:
[dependencies]
foundry-application = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
```

### Phase 2: Template (Kurzfristig) 🎯
```bash
# Template Repository erstellen
# - Auf GitHub als Template markieren
# - Vollständige Projektstruktur
# - README mit Quick Start
```

### Phase 3: Crates.io (Mittelfristig) 🚀
```bash
# Alle Crates veröffentlichen
cargo publish --workspace

# Nutzung:
[dependencies]
foundry-application = "0.2"
```

### Phase 4: CLI Tool (Langfristig) 🛠️
```bash
# CLI Tool entwickeln
cargo install cargo-foundry

# Nutzung:
cargo foundry new my-app
```

---

## 🎬 Quick Start Examples

### Mit Git (Jetzt verfügbar)

```toml
# Cargo.toml
[dependencies]
foundry-application = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-queue = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-cache = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
```

### Mit Crates.io (Zukünftig)

```toml
# Cargo.toml
[dependencies]
foundry = "0.2"  # Meta-crate mit allen Features

# Oder einzeln:
foundry-application = "0.2"
foundry-queue = "0.2"
foundry-cache = "0.2"
```

### Mit Template (Zukünftig)

```bash
# Via GitHub
# 1. Gehe zu https://github.com/Chregu12/RustForge-Template
# 2. Klicke "Use this template"
# 3. cargo build && cargo run

# Via CLI
cargo foundry new my-app
cd my-app
cargo run
```

---

## 📊 Vergleich

| Feature | Git Tags | Template | Crates.io | CLI Tool |
|---------|----------|----------|-----------|----------|
| Setup Zeit | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Updates | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Flexibilität | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Einfachheit | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Standards | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Community | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

---

## 🎯 Nächste Schritte

### Sofort (Heute)
1. ✅ Git Tags erstellen (v0.2.0)
2. ✅ GitHub Release mit CHANGELOG
3. ✅ README aktualisieren mit Usage Examples

### Diese Woche
4. 📝 Template Repository erstellen
5. 📝 Quick Start Guide schreiben
6. 📝 Beispiel-Projekte erstellen

### Diesen Monat
7. 📦 Crates.io Publishing vorbereiten
8. 📦 Crate-Namen reservieren
9. 📦 Erste Crates veröffentlichen

### Später
10. 🛠️ CLI Tool Prototyp
11. 🛠️ Code-Generierung
12. 🛠️ cargo-foundry v1.0

---

## 🤔 Welche Option wählen?

### Für Schnellstart (JETZT):
```bash
# Template Repository + Git Dependencies
```

### Für Production (Bald):
```bash
# Crates.io Publishing
```

### Für beste DX (Später):
```bash
# CLI Tool + Crates.io
```

---

**Empfehlung:** Starte mit **Option 1 (Git) + Option 2 (Template)**, dann **Option 3 (Crates.io)** sobald Framework stabiler ist, und entwickle **Option 4 (CLI Tool)** langfristig für perfekte Laravel-ähnliche DX.
