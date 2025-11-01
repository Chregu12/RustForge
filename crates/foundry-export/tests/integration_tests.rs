//! Integration tests for foundry-export

use foundry_export::{ExportData, ExportFormat, Exporter};
use serde_json::json;

#[test]
fn test_csv_export() {
    let data = ExportData::new(
        vec!["ID".to_string(), "Name".to_string(), "Email".to_string()],
        vec![
            vec!["1".to_string(), "Alice".to_string(), "alice@example.com".to_string()],
            vec!["2".to_string(), "Bob".to_string(), "bob@example.com".to_string()],
        ],
    );

    let exporter = Exporter::new();
    let result = exporter.export(data, ExportFormat::Csv);

    assert!(result.is_ok());
    let bytes = result.unwrap();
    let csv_content = String::from_utf8(bytes).unwrap();

    assert!(csv_content.contains("ID,Name,Email"));
    assert!(csv_content.contains("Alice"));
    assert!(csv_content.contains("Bob"));
}

#[test]
fn test_excel_export() {
    let data = ExportData::new(
        vec!["ID".to_string(), "Name".to_string()],
        vec![
            vec!["1".to_string(), "Alice".to_string()],
            vec!["2".to_string(), "Bob".to_string()],
        ],
    );

    let exporter = Exporter::new();
    let result = exporter.export(data, ExportFormat::Xlsx);

    assert!(result.is_ok());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn test_pdf_export() {
    let data = ExportData::new(
        vec!["ID".to_string(), "Name".to_string()],
        vec![
            vec!["1".to_string(), "Alice".to_string()],
        ],
    ).with_title("Test Report");

    let exporter = Exporter::new();
    let result = exporter.export(data, ExportFormat::Pdf);

    assert!(result.is_ok());
}

#[test]
fn test_export_from_json() {
    let json_data = json!([
        {"id": 1, "name": "Alice", "active": true},
        {"id": 2, "name": "Bob", "active": false},
    ]);

    let data = ExportData::from_json(json_data).unwrap();

    assert_eq!(data.headers.len(), 3);
    assert_eq!(data.rows.len(), 2);
}

#[test]
fn test_export_with_metadata() {
    let data = ExportData::new(
        vec!["ID".to_string()],
        vec![vec!["1".to_string()]],
    )
    .with_title("My Report")
    .with_author("Test User");

    assert_eq!(data.metadata.title, Some("My Report".to_string()));
    assert_eq!(data.metadata.author, Some("Test User".to_string()));
}
