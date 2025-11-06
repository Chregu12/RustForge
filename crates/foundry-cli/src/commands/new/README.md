# RustForge `new` Command

Das `rustforge new` Command ist das zentrale Feature der RustForge Developer Experience - es transformiert `cargo new` (20 Minuten Setup) zu `rustforge new` (2-3 Minuten bis zur lauffähigen App).

## Features

### Interactive Wizard
- Template-Auswahl (API REST, Full-Stack React, Full-Stack Leptos, CLI Tool)
- Feature-Auswahl (Authentication, Database, Redis, Email, Tests)
- Database-Konfiguration (wenn ausgewählt)
- Zusammenfassung und Bestätigung

### Automatische Generierung
- **Cargo.toml** mit allen notwendigen Dependencies
- **src/** Struktur mit vollständigem Boilerplate-Code
- **.env** und **.env.example** mit Konfiguration
- **.gitignore** für Rust-Projekte
- **migrations/** für Datenbank-Migrationen (wenn aktiviert)
- **tests/** für Integration Tests (wenn aktiviert)

### Post-Generation
- Git Repository initialisieren
- `cargo check` ausführen
- PostgreSQL Datenbank erstellen (wenn aktiviert)
- Migrationen ausführen (wenn sqlx-cli installiert)

## Usage

### Interaktiver Modus
```bash
rustforge new my-app
```

Der Wizard führt Sie durch alle Optionen:
1. Template-Auswahl
2. Feature-Auswahl
3. Datenbank-Konfiguration
4. Bestätigung

### Quick Mode (mit Defaults)
```bash
rustforge new my-app --skip-wizard
```

Verwendet Standard-Einstellungen:
- Template: API REST
- Features: Database, Tests
- DB: localhost:5432, postgres/postgres

### Hilfe
```bash
rustforge new --help
```

## Architektur

### Dateien

```
crates/foundry-cli/src/commands/new/
├── mod.rs              # Command Entry Point
├── config.rs           # Konfigurationsstrukturen
├── wizard.rs           # Interaktiver Wizard
├── generator.rs        # Projekt-Generator
└── templates/
    ├── mod.rs
    └── api_rest.rs     # API REST Template Generator
```

### Ablauf

1. **Wizard** (`wizard.rs`)
   - Sammelt Benutzereingaben
   - Erstellt `ProjectConfig`

2. **Generator** (`generator.rs`)
   - Ruft Template-Generator auf
   - Führt Post-Generation Tasks aus

3. **Template** (`templates/api_rest.rs`)
   - Generiert Dateistruktur
   - Erstellt alle notwendigen Files

## Templates

### API REST (Implementiert)
Vollständiges REST API Template mit:
- Axum Web Framework
- Health Check Endpoint
- Optional: PostgreSQL + SQLx
- Optional: JWT Authentication
- Optional: Redis Cache
- Optional: Integration Tests

### Geplante Templates
- **Full-Stack React**: React Frontend + Axum Backend
- **Full-Stack Leptos**: Leptos WASM Frontend + Axum Backend
- **CLI Tool**: Command-Line Application mit clap

## Erweiterung

### Neues Template hinzufügen

1. Template in `config.rs` registrieren:
```rust
pub enum TemplateType {
    ApiRest,
    YourNewTemplate,  // Neu
}
```

2. Template-Generator erstellen:
```rust
// templates/your_template.rs
pub struct YourTemplateGenerator {
    config: ProjectConfig,
}

impl YourTemplateGenerator {
    pub fn generate(&self, path: &Path) -> Result<()> {
        // Implementierung
    }
}
```

3. In `generator.rs` integrieren:
```rust
match self.config.template {
    TemplateType::ApiRest => { /* ... */ }
    TemplateType::YourNewTemplate => {
        let template = YourTemplateGenerator::new(self.config.clone());
        template.generate(&self.config.path)?;
    }
}
```

### Neues Feature hinzufügen

1. Feature in `config.rs` definieren:
```rust
pub enum Feature {
    Authentication,
    Database,
    YourNewFeature,  // Neu
}
```

2. In Templates implementieren:
```rust
if self.config.has_feature(Feature::YourNewFeature) {
    // Feature-spezifische Code-Generierung
}
```

## Testing

### Unit Tests
```bash
cargo test -p foundry-cli --test new_command_test
```

### Integration Test
```bash
# Projekt erstellen
rustforge new test-project

# Projekt testen
cd test-project
cargo build
cargo run
```

### Erfolgs-Kriterien
- ✅ Projekt wird erstellt
- ✅ Alle Dateien vorhanden
- ✅ `cargo check` erfolgreich
- ✅ `cargo run` startet Server
- ✅ Health-Check Endpoint antwortet

## Troubleshooting

### "Directory already exists"
```bash
# Lösung: Anderen Namen wählen oder Verzeichnis löschen
rm -rf my-app
rustforge new my-app
```

### "Could not create database"
```bash
# Lösung: PostgreSQL muss laufen
brew services start postgresql@14

# Oder manuell erstellen
psql -U postgres -c 'CREATE DATABASE my_app_dev;'
```

### "sqlx-cli not installed"
```bash
# Lösung: sqlx-cli installieren
cargo install sqlx-cli --features postgres
```

### Cargo check fails
```bash
# Lösung: Dependencies aktualisieren
cd my-app
cargo update
cargo check
```

## Beispiel-Output

```
🔨 RustForge Project Generator
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌ What type of project? ─────────────────────────────────┐
│ › API (REST/GraphQL Backend)                            │
│   Full-Stack (React + Axum)                             │
│   Full-Stack (Leptos WASM)                              │
│   CLI Tool                                               │
└─────────────────────────────────────────────────────────┘

┌ Select features ────────────────────────────────────────┐
│ [x] Authentication (JWT)                                 │
│ [x] Database (PostgreSQL)                                │
│ [ ] Redis Cache                                          │
│ [ ] Email (SMTP)                                         │
│ [x] Tests & Fixtures                                     │
└─────────────────────────────────────────────────────────┘

📊 Database Configuration
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Database name: blog_api_dev
Database host: localhost
Database port: 5432
Database username: postgres
Database password: ********

📋 Project Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Name:     blog-api
  Template: API (REST/GraphQL Backend)
  Features:
    - Authentication (JWT)
    - Database (PostgreSQL)
    - Tests & Fixtures
  Database:
    - Name: blog_api_dev
    - Host: localhost:5432

✨ Creating project...

  ✅ Generated project structure
  🔄 Initializing git repository... ✅
  🔄 Running cargo check... ✅
  🔄 Setting up database... ✅
  ✅ Created database 'blog_api_dev'

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 Project 'blog-api' created successfully!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📋 Next steps:

  1. Navigate to your project:
     cd blog-api

  2. Ensure PostgreSQL is running

  3. Run the application:
     cargo run

  The server will start on http://localhost:3000

📚 API Endpoints:
  - GET  /health              - Health check
  - POST /api/auth/register   - Register new user
  - POST /api/auth/login      - Login user

📖 Documentation:
  - Check the .env file for configuration
  - Read the generated code for implementation details
  - Customize the templates to fit your needs

🚀 Happy coding with RustForge!
```

## Lizenz

MIT OR Apache-2.0
