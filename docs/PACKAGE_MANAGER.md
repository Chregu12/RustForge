# Package Manager

RustForge bietet einen Composer-ähnlichen Package Manager für Rust, der die Verwaltung von Dependencies vereinfacht.

## Features

- **Install/Remove**: Einfaches Hinzufügen und Entfernen von Packages
- **Update**: Aktualisierung einzelner oder aller Packages
- **Search**: Suche nach Packages auf crates.io
- **List**: Übersicht aller installierten Packages
- **Outdated**: Prüfung auf veraltete Dependencies
- **Integration**: Direkter Zugriff auf Cargo-Funktionalität

## CLI Commands

### Package installieren

```bash
# Neueste Version
rustforge package:install serde

# Spezifische Version
rustforge package:install --version "1.0" serde

# Dev-Dependency
rustforge package:install --dev tokio-test
```

### Package entfernen

```bash
rustforge package:remove serde
```

### Packages aktualisieren

```bash
# Alle Packages
rustforge package:update

# Spezifisches Package
rustforge package:update serde
```

### Packages auflisten

```bash
rustforge package:list
```

### Nach Packages suchen

```bash
rustforge package:search "async runtime"
```

### Veraltete Packages prüfen

```bash
rustforge package:outdated
```

## Programmatische Verwendung

```rust
use foundry_infra::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pm = PackageManager::new(".");

    // Package installieren
    pm.install("serde", Some("1.0")).await?;

    // Package entfernen
    pm.remove("old-package").await?;

    // Alle Packages aktualisieren
    pm.update().await?;

    // Spezifisches Package aktualisieren
    pm.update_package("tokio").await?;

    // Packages auflisten
    let packages = pm.list().await?;
    for pkg in packages {
        println!("- {}", pkg);
    }

    // Package-Info abrufen
    let info = pm.show("serde").await?;
    println!("Name: {}", info.name);
    println!("Version: {}", info.version);

    // Nach Packages suchen
    let results = pm.search("web framework").await?;
    for result in results {
        println!("{} v{}: {}",
            result.name,
            result.version,
            result.description.unwrap_or_default()
        );
    }

    // Veraltete Packages
    let outdated = pm.outdated().await?;
    for pkg in outdated {
        println!("{}: {} -> {}",
            pkg.name,
            pkg.current_version,
            pkg.latest_version
        );
    }

    Ok(())
}
```

## Cargo.toml Integration

Der Package Manager manipuliert direkt deine `Cargo.toml`:

### Vor Installation

```toml
[dependencies]
tokio = "1.0"
```

### Nach `rustforge package:install serde --version "1.0"`

```toml
[dependencies]
tokio = "1.0"
serde = "1.0"
```

## Best Practices

### 1. Versionspinning

```bash
# Exakte Version
rustforge package:install --version "=1.0.0" serde

# Caret (^) - Default
rustforge package:install --version "^1.0" serde

# Tilde (~)
rustforge package:install --version "~1.0" serde
```

### 2. Dev Dependencies

```bash
# Für Testing und Development
rustforge package:install --dev mockito
rustforge package:install --dev criterion
```

### 3. Features aktivieren

```bash
# Packages mit Features in Cargo.toml manuell bearbeiten
# Der Package Manager fügt standard Features hinzu
```

## Erweiterte Features

### Crates.io API

Der Package Manager nutzt die offizielle crates.io API:

```rust
let results = pm.search("http client").await?;

for result in results {
    println!("📦 {} v{}", result.name, result.version);
    println!("   Downloads: {}", result.downloads);
    if let Some(desc) = result.description {
        println!("   {}", desc);
    }
}
```

### Metadata abrufen

```rust
let info = pm.show("axum").await?;

println!("Package: {}", info.name);
println!("Version: {}", info.version);
println!("License: {}", info.license.unwrap_or_else(|| "N/A".to_string()));
```

## Vergleich mit Cargo

| Feature | Cargo | RustForge PM |
|---------|-------|-------------|
| Install | `cargo add serde` | `rustforge package:install serde` |
| Remove | `cargo rm serde` | `rustforge package:remove serde` |
| Update | `cargo update` | `rustforge package:update` |
| Search | N/A | `rustforge package:search` |
| List | N/A | `rustforge package:list` |
| Outdated | Requires cargo-outdated | `rustforge package:outdated` |

## Troubleshooting

### Problem: Package nicht gefunden

- Prüfe Schreibweise auf crates.io
- Verwende `rustforge package:search` zur Suche

### Problem: Version-Konflikt

```bash
# Prüfe Dependencies
cargo tree

# Update auf kompatible Versionen
rustforge package:update
```

### Problem: Cargo.lock Konflikte

```bash
# Lösche Lock-File und rebuild
rm Cargo.lock
cargo build
```

## Zukünftige Features

- [ ] Automatisches Dependency-Cleanup
- [ ] Security Audit Integration
- [ ] License Compliance Checks
- [ ] Private Registry Support
- [ ] Workspace Management

---

**Version**: 0.1.0
**Letztes Update**: 2025-11-01
