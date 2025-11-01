//! Package Manager - Composer-ähnliches System für Rust
//!
//! Verwaltet Abhängigkeiten, Versionen und Package-Installation.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// Eine Package-Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package-Name
    pub name: String,
    /// Version
    pub version: String,
    /// Beschreibung
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Abhängigkeiten
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Dev-Abhängigkeiten
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub dev_dependencies: HashMap<String, String>,
    /// Features
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// Package Lock-Datei
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLock {
    /// Gesperrte Packages mit exakten Versionen
    pub packages: HashMap<String, LockedPackage>,
    /// Zeitstempel der letzten Aktualisierung
    pub updated_at: i64,
}

/// Ein gesperrtes Package mit exakter Version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Exakte Version
    pub version: String,
    /// Aufgelöste Abhängigkeiten
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Checksum/Hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Package Manager Service
pub struct PackageManager {
    /// Workspace-Pfad
    workspace_path: PathBuf,
}

impl PackageManager {
    /// Erstellt einen neuen PackageManager
    pub fn new(workspace_path: impl Into<PathBuf>) -> Self {
        Self {
            workspace_path: workspace_path.into(),
        }
    }

    /// Installiert ein Package
    ///
    /// # Beispiel
    ///
    /// ```no_run
    /// use foundry_infra::package_manager::PackageManager;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let pm = PackageManager::new(".");
    /// pm.install("serde", Some("1.0")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install(&self, package_name: &str, version: Option<&str>) -> Result<()> {
        info!("Installing package: {}@{}", package_name, version.unwrap_or("latest"));

        let version_spec = version.unwrap_or("*");

        // Füge zum Cargo.toml hinzu
        self.add_to_cargo_toml(package_name, version_spec)?;

        // Führe cargo fetch aus
        self.cargo_fetch().await?;

        info!("Package {} installed successfully", package_name);
        Ok(())
    }

    /// Installiert ein Dev-Package
    pub async fn install_dev(&self, package_name: &str, version: Option<&str>) -> Result<()> {
        info!("Installing dev package: {}@{}", package_name, version.unwrap_or("latest"));

        let version_spec = version.unwrap_or("*");

        // Füge zu [dev-dependencies] hinzu
        self.add_to_cargo_toml_dev(package_name, version_spec)?;

        self.cargo_fetch().await?;

        info!("Dev package {} installed successfully", package_name);
        Ok(())
    }

    /// Entfernt ein Package
    pub async fn remove(&self, package_name: &str) -> Result<()> {
        info!("Removing package: {}", package_name);

        // Entferne aus Cargo.toml
        self.remove_from_cargo_toml(package_name)?;

        info!("Package {} removed successfully", package_name);
        Ok(())
    }

    /// Aktualisiert alle Packages
    pub async fn update(&self) -> Result<()> {
        info!("Updating all packages...");

        let output = Command::new("cargo")
            .arg("update")
            .current_dir(&self.workspace_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to update packages: {}", stderr));
        }

        info!("All packages updated successfully");
        Ok(())
    }

    /// Aktualisiert ein spezifisches Package
    pub async fn update_package(&self, package_name: &str) -> Result<()> {
        info!("Updating package: {}", package_name);

        let output = Command::new("cargo")
            .arg("update")
            .arg("-p")
            .arg(package_name)
            .current_dir(&self.workspace_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to update package {}: {}", package_name, stderr));
        }

        info!("Package {} updated successfully", package_name);
        Ok(())
    }

    /// Listet alle installierten Packages
    pub async fn list(&self) -> Result<Vec<String>> {
        let cargo_toml_path = self.workspace_path.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml_path)?;

        let mut packages = Vec::new();

        // Parse TOML (einfache Variante)
        let toml: toml::Value = toml::from_str(&content)?;

        if let Some(deps) = toml.get("dependencies").and_then(|v| v.as_table()) {
            for (name, _) in deps {
                packages.push(name.clone());
            }
        }

        Ok(packages)
    }

    /// Zeigt Informationen über ein Package
    pub async fn show(&self, package_name: &str) -> Result<PackageInfo> {
        info!("Fetching package info: {}", package_name);

        // Nutze cargo metadata
        let output = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            .current_dir(&self.workspace_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to fetch metadata"));
        }

        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        // Finde das Package in den Metadaten
        if let Some(packages) = metadata.get("packages").and_then(|p| p.as_array()) {
            for pkg in packages {
                if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                    if name == package_name {
                        let version = pkg.get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let description = pkg.get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string());

                        let license = pkg.get("license")
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string());

                        return Ok(PackageInfo {
                            name: name.to_string(),
                            version,
                            description,
                            license,
                        });
                    }
                }
            }
        }

        Err(anyhow!("Package {} not found", package_name))
    }

    /// Sucht nach Packages auf crates.io
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        info!("Searching for: {}", query);

        // Verwende crates.io API
        let url = format!("https://crates.io/api/v1/crates?q={}", query);
        let client = reqwest::Client::new();

        let response = client
            .get(&url)
            .header("User-Agent", "RustForge/0.1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Search request failed: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await?;

        let mut results = Vec::new();

        if let Some(crates) = data.get("crates").and_then(|c| c.as_array()) {
            for krate in crates.iter().take(10) {
                let name = krate.get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();

                let version = krate.get("max_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let description = krate.get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string());

                let downloads = krate.get("downloads")
                    .and_then(|d| d.as_u64())
                    .unwrap_or(0);

                results.push(SearchResult {
                    name,
                    version,
                    description,
                    downloads,
                });
            }
        }

        Ok(results)
    }

    /// Bereinigt nicht verwendete Dependencies
    pub async fn clean(&self) -> Result<()> {
        info!("Cleaning unused dependencies...");

        let output = Command::new("cargo")
            .arg("clean")
            .current_dir(&self.workspace_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to clean: {}", stderr));
        }

        info!("Cleanup completed successfully");
        Ok(())
    }

    /// Prüft veraltete Packages
    pub async fn outdated(&self) -> Result<Vec<OutdatedPackage>> {
        info!("Checking for outdated packages...");

        // Nutze cargo-outdated falls installiert
        let output = Command::new("cargo")
            .arg("outdated")
            .arg("--format=json")
            .current_dir(&self.workspace_path)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let data: serde_json::Value = serde_json::from_slice(&out.stdout)?;
                let mut outdated = Vec::new();

                if let Some(packages) = data.get("dependencies").and_then(|d| d.as_array()) {
                    for pkg in packages {
                        let name = pkg.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();

                        let current = pkg.get("project")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let latest = pkg.get("latest")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if current != latest {
                            outdated.push(OutdatedPackage {
                                name,
                                current_version: current,
                                latest_version: latest,
                            });
                        }
                    }
                }

                Ok(outdated)
            }
            _ => {
                warn!("cargo-outdated not available, install with: cargo install cargo-outdated");
                Ok(Vec::new())
            }
        }
    }

    // Private Hilfsmethoden

    fn add_to_cargo_toml(&self, package_name: &str, version: &str) -> Result<()> {
        let cargo_toml_path = self.workspace_path.join("Cargo.toml");
        let mut content = fs::read_to_string(&cargo_toml_path)?;

        // Einfache Implementierung: Füge am Ende der [dependencies] Sektion hinzu
        let dep_line = format!("{} = \"{}\"", package_name, version);

        if content.contains(&dep_line) {
            info!("Package {} already in Cargo.toml", package_name);
            return Ok(());
        }

        // Finde [dependencies] Sektion
        if let Some(deps_pos) = content.find("[dependencies]") {
            let after_deps = &content[deps_pos..];
            if let Some(next_section) = after_deps.find("\n[") {
                let insert_pos = deps_pos + next_section;
                content.insert_str(insert_pos, &format!("\n{}", dep_line));
            } else {
                // Keine weitere Sektion, füge am Ende hinzu
                content.push_str(&format!("\n{}", dep_line));
            }
        } else {
            // [dependencies] existiert nicht, erstelle es
            content.push_str(&format!("\n[dependencies]\n{}\n", dep_line));
        }

        fs::write(&cargo_toml_path, content)?;
        info!("Added {} to Cargo.toml", package_name);
        Ok(())
    }

    fn add_to_cargo_toml_dev(&self, package_name: &str, version: &str) -> Result<()> {
        let cargo_toml_path = self.workspace_path.join("Cargo.toml");
        let mut content = fs::read_to_string(&cargo_toml_path)?;

        let dep_line = format!("{} = \"{}\"", package_name, version);

        if content.contains(&dep_line) {
            return Ok(());
        }

        if let Some(deps_pos) = content.find("[dev-dependencies]") {
            let after_deps = &content[deps_pos..];
            if let Some(next_section) = after_deps.find("\n[") {
                let insert_pos = deps_pos + next_section;
                content.insert_str(insert_pos, &format!("\n{}", dep_line));
            } else {
                content.push_str(&format!("\n{}", dep_line));
            }
        } else {
            content.push_str(&format!("\n[dev-dependencies]\n{}\n", dep_line));
        }

        fs::write(&cargo_toml_path, content)?;
        Ok(())
    }

    fn remove_from_cargo_toml(&self, package_name: &str) -> Result<()> {
        let cargo_toml_path = self.workspace_path.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml_path)?;

        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();

        for line in lines {
            if !line.trim().starts_with(&format!("{} =", package_name)) {
                new_lines.push(line);
            }
        }

        fs::write(&cargo_toml_path, new_lines.join("\n"))?;
        Ok(())
    }

    async fn cargo_fetch(&self) -> Result<()> {
        let output = Command::new("cargo")
            .arg("fetch")
            .current_dir(&self.workspace_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("cargo fetch failed: {}", stderr));
        }

        Ok(())
    }
}

/// Package-Informationen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub license: Option<String>,
}

/// Such-Ergebnis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub downloads: u64,
}

/// Veraltetes Package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedPackage {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_package_manager_creation() {
        let pm = PackageManager::new(".");
        assert_eq!(pm.workspace_path, PathBuf::from("."));
    }

    #[test]
    fn test_package_serialization() {
        let pkg = Package {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test".to_string()),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            features: vec![],
        };

        let json = serde_json::to_string(&pkg).unwrap();
        assert!(json.contains("test-package"));
    }
}
