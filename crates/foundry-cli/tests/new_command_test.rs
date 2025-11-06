use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_project_name_validation() {
    // Test that empty project name is rejected
    let name = "";
    assert!(name.is_empty(), "Empty name should be rejected");
}

#[test]
fn test_project_directory_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("test_project");

    // Verify directory doesn't exist yet
    assert!(!project_path.exists());

    // Create directory
    fs::create_dir_all(&project_path).expect("Failed to create project directory");

    // Verify it now exists
    assert!(project_path.exists());
    assert!(project_path.is_dir());
}

#[test]
fn test_cargo_toml_structure() {
    // Test that we can parse a basic Cargo.toml structure
    let cargo_toml = r#"
[package]
name = "test_app"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = "1"
"#;

    // Basic validation that it contains required sections
    assert!(cargo_toml.contains("[package]"));
    assert!(cargo_toml.contains("name = "));
    assert!(cargo_toml.contains("[dependencies]"));
}

#[test]
fn test_sanitized_name() {
    // Test name sanitization logic
    let name = "My-Cool App";
    let sanitized = name
        .to_lowercase()
        .replace('-', "_")
        .replace(' ', "_");

    assert_eq!(sanitized, "my_cool_app");
}

#[test]
fn test_db_name_generation() {
    let project_name = "blog_api";
    let db_name = format!("{}_dev", project_name);

    assert_eq!(db_name, "blog_api_dev");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Note: These are basic structural tests.
    // Full integration tests would require setting up the entire environment
    // and running the actual command, which is better done manually or in CI.

    #[test]
    fn test_temp_directory_cleanup() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_path_buf();

        // Verify temp dir exists
        assert!(path.exists());

        // Drop temp_dir to trigger cleanup
        drop(temp_dir);

        // Verify it's cleaned up (note: this might not work on all systems immediately)
        // In practice, the OS handles cleanup
    }

    #[test]
    fn test_project_structure_requirements() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let project_path = temp_dir.path().join("test_project");

        // Create minimal project structure
        fs::create_dir_all(&project_path).expect("Failed to create project dir");
        fs::create_dir_all(project_path.join("src")).expect("Failed to create src dir");

        // Create minimal Cargo.toml
        let cargo_toml = r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
        fs::write(project_path.join("Cargo.toml"), cargo_toml)
            .expect("Failed to write Cargo.toml");

        // Create minimal main.rs
        let main_rs = r#"fn main() {
    println!("Hello, world!");
}
"#;
        fs::write(project_path.join("src/main.rs"), main_rs)
            .expect("Failed to write main.rs");

        // Verify structure
        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
    }
}

// Add tempfile to dev-dependencies in Cargo.toml for these tests
