//! Data Export System for RustForge
//!
//! This crate provides data export functionality in various formats.

use async_trait::async_trait;
use bytes::Bytes;
use serde::Serialize;
use std::io::Write;
use thiserror::Error;

/// Export errors
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Format error: {0}")]
    FormatError(String),

    #[error("Template error: {0}")]
    TemplateError(String),
}

pub type ExportResult<T> = Result<T, ExportError>;

/// Export format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Excel,
    Pdf,
    Json,
}

/// Exporter trait
#[async_trait]
pub trait Exporter: Send + Sync {
    /// Export data to bytes
    async fn export(&self) -> ExportResult<Bytes>;

    /// Get content type
    fn content_type(&self) -> &'static str;

    /// Get file extension
    fn file_extension(&self) -> &'static str;
}

/// CSV exporter
pub struct CsvExporter {
    data: Vec<serde_json::Value>,
    columns: Vec<String>,
    headers: Option<Vec<String>>,
    delimiter: u8,
}

impl CsvExporter {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            columns: Vec::new(),
            headers: None,
            delimiter: b',',
        }
    }

    /// Set data from serializable values
    pub fn from_data<T: Serialize>(mut self, data: &[T]) -> ExportResult<Self> {
        self.data = data
            .iter()
            .map(|item| {
                serde_json::to_value(item)
                    .map_err(|e| ExportError::SerializationError(e.to_string()))
            })
            .collect::<ExportResult<Vec<_>>>()?;
        Ok(self)
    }

    /// Set columns to export
    pub fn columns(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set custom headers
    pub fn headers(mut self, headers: &[&str]) -> Self {
        self.headers = Some(headers.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set delimiter (default: comma)
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Export to CSV bytes
    pub async fn export(&self) -> ExportResult<Bytes> {
        let mut writer = csv::WriterBuilder::new()
            .delimiter(self.delimiter)
            .from_writer(vec![]);

        // Write headers
        if let Some(ref custom_headers) = self.headers {
            writer
                .write_record(custom_headers)
                .map_err(|e| ExportError::IoError(e.to_string()))?;
        } else if !self.columns.is_empty() {
            writer
                .write_record(&self.columns)
                .map_err(|e| ExportError::IoError(e.to_string()))?;
        }

        // Write data rows
        for item in &self.data {
            let mut row = Vec::new();

            if self.columns.is_empty() {
                // If no columns specified, try to export all fields
                if let serde_json::Value::Object(map) = item {
                    for value in map.values() {
                        row.push(value_to_string(value));
                    }
                }
            } else {
                // Export specified columns only
                for col in &self.columns {
                    let value = item.get(col).unwrap_or(&serde_json::Value::Null);
                    row.push(value_to_string(value));
                }
            }

            writer
                .write_record(&row)
                .map_err(|e| ExportError::IoError(e.to_string()))?;
        }

        writer
            .flush()
            .map_err(|e| ExportError::IoError(e.to_string()))?;

        let bytes = writer
            .into_inner()
            .map_err(|e| ExportError::IoError(e.to_string()))?;

        Ok(Bytes::from(bytes))
    }
}

impl Default for CsvExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Exporter for CsvExporter {
    async fn export(&self) -> ExportResult<Bytes> {
        self.export().await
    }

    fn content_type(&self) -> &'static str {
        "text/csv"
    }

    fn file_extension(&self) -> &'static str {
        "csv"
    }
}

/// Excel exporter (stub - requires additional dependencies)
pub struct ExcelExporter {
    data: Vec<serde_json::Value>,
    sheet_name: String,
    columns: Vec<String>,
}

impl ExcelExporter {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            sheet_name: "Sheet1".to_string(),
            columns: Vec::new(),
        }
    }

    pub fn from_data<T: Serialize>(mut self, data: &[T]) -> ExportResult<Self> {
        self.data = data
            .iter()
            .map(|item| {
                serde_json::to_value(item)
                    .map_err(|e| ExportError::SerializationError(e.to_string()))
            })
            .collect::<ExportResult<Vec<_>>>()?;
        Ok(self)
    }

    pub fn sheet(mut self, name: impl Into<String>) -> Self {
        self.sheet_name = name.into();
        self
    }

    pub fn columns(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Export to Excel bytes (stub implementation)
    pub async fn export(&self) -> ExportResult<Bytes> {
        // This is a stub. In production, use rust_xlsxwriter or similar
        Err(ExportError::FormatError(
            "Excel export requires additional dependencies. Use CsvExporter as alternative."
                .to_string(),
        ))
    }
}

impl Default for ExcelExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Exporter for ExcelExporter {
    async fn export(&self) -> ExportResult<Bytes> {
        self.export().await
    }

    fn content_type(&self) -> &'static str {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    }

    fn file_extension(&self) -> &'static str {
        "xlsx"
    }
}

/// PDF exporter (stub - requires additional dependencies)
pub struct PdfExporter {
    data: serde_json::Value,
    template: Option<String>,
}

impl PdfExporter {
    pub fn new() -> Self {
        Self {
            data: serde_json::Value::Null,
            template: None,
        }
    }

    pub fn from_data<T: Serialize>(mut self, data: &T) -> ExportResult<Self> {
        self.data = serde_json::to_value(data)
            .map_err(|e| ExportError::SerializationError(e.to_string()))?;
        Ok(self)
    }

    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Export to PDF bytes (stub implementation)
    pub async fn export(&self) -> ExportResult<Bytes> {
        // This is a stub. In production, use printpdf, wkhtmltopdf, or similar
        Err(ExportError::FormatError(
            "PDF export requires additional dependencies. Use CsvExporter as alternative."
                .to_string(),
        ))
    }
}

impl Default for PdfExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Exporter for PdfExporter {
    async fn export(&self) -> ExportResult<Bytes> {
        self.export().await
    }

    fn content_type(&self) -> &'static str {
        "application/pdf"
    }

    fn file_extension(&self) -> &'static str {
        "pdf"
    }
}

/// JSON exporter
pub struct JsonExporter {
    data: serde_json::Value,
    pretty: bool,
}

impl JsonExporter {
    pub fn new() -> Self {
        Self {
            data: serde_json::Value::Null,
            pretty: false,
        }
    }

    pub fn from_data<T: Serialize>(mut self, data: &T) -> ExportResult<Self> {
        self.data = serde_json::to_value(data)
            .map_err(|e| ExportError::SerializationError(e.to_string()))?;
        Ok(self)
    }

    pub fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }

    pub async fn export(&self) -> ExportResult<Bytes> {
        let json = if self.pretty {
            serde_json::to_string_pretty(&self.data)
        } else {
            serde_json::to_string(&self.data)
        }
        .map_err(|e| ExportError::SerializationError(e.to_string()))?;

        Ok(Bytes::from(json))
    }
}

impl Default for JsonExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Exporter for JsonExporter {
    async fn export(&self) -> ExportResult<Bytes> {
        self.export().await
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }

    fn file_extension(&self) -> &'static str {
        "json"
    }
}

// Helper function to convert JSON value to string
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestData {
        id: i64,
        name: String,
        email: String,
        active: bool,
    }

    #[tokio::test]
    async fn test_csv_export_basic() {
        let data = vec![
            TestData {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                active: true,
            },
            TestData {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                active: false,
            },
        ];

        let exporter = CsvExporter::new()
            .from_data(&data)
            .unwrap()
            .columns(&["id", "name", "email"]);

        let bytes = exporter.export().await.unwrap();
        let csv = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(csv.contains("id,name,email"));
        assert!(csv.contains("1,Alice,alice@example.com"));
        assert!(csv.contains("2,Bob,bob@example.com"));
    }

    #[tokio::test]
    async fn test_csv_export_with_custom_headers() {
        let data = vec![TestData {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            active: true,
        }];

        let exporter = CsvExporter::new()
            .from_data(&data)
            .unwrap()
            .columns(&["id", "name"])
            .headers(&["ID", "Full Name"]);

        let bytes = exporter.export().await.unwrap();
        let csv = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(csv.contains("ID,Full Name"));
        assert!(csv.contains("1,Alice"));
    }

    #[tokio::test]
    async fn test_csv_export_with_custom_delimiter() {
        let data = vec![TestData {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            active: true,
        }];

        let exporter = CsvExporter::new()
            .from_data(&data)
            .unwrap()
            .columns(&["id", "name"])
            .delimiter(b';');

        let bytes = exporter.export().await.unwrap();
        let csv = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(csv.contains("id;name"));
        assert!(csv.contains("1;Alice"));
    }

    #[tokio::test]
    async fn test_csv_content_type() {
        let exporter = CsvExporter::new();
        assert_eq!(exporter.content_type(), "text/csv");
        assert_eq!(exporter.file_extension(), "csv");
    }

    #[tokio::test]
    async fn test_json_export() {
        let data = vec![
            TestData {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                active: true,
            },
            TestData {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                active: false,
            },
        ];

        let exporter = JsonExporter::new().from_data(&data).unwrap();
        let bytes = exporter.export().await.unwrap();
        let json = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(json.contains("Alice"));
        assert!(json.contains("alice@example.com"));
    }

    #[tokio::test]
    async fn test_json_export_pretty() {
        let data = TestData {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            active: true,
        };

        let exporter = JsonExporter::new().from_data(&data).unwrap().pretty();
        let bytes = exporter.export().await.unwrap();
        let json = String::from_utf8(bytes.to_vec()).unwrap();

        // Pretty JSON should have newlines
        assert!(json.contains('\n'));
        assert!(json.contains("  "));
    }

    #[tokio::test]
    async fn test_json_content_type() {
        let exporter = JsonExporter::new();
        assert_eq!(exporter.content_type(), "application/json");
        assert_eq!(exporter.file_extension(), "json");
    }

    #[tokio::test]
    async fn test_excel_content_type() {
        let exporter = ExcelExporter::new();
        assert_eq!(
            exporter.content_type(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(exporter.file_extension(), "xlsx");
    }

    #[tokio::test]
    async fn test_pdf_content_type() {
        let exporter = PdfExporter::new();
        assert_eq!(exporter.content_type(), "application/pdf");
        assert_eq!(exporter.file_extension(), "pdf");
    }

    #[tokio::test]
    async fn test_csv_empty_data() {
        let data: Vec<TestData> = vec![];
        let exporter = CsvExporter::new()
            .from_data(&data)
            .unwrap()
            .columns(&["id", "name"]);

        let bytes = exporter.export().await.unwrap();
        let csv = String::from_utf8(bytes.to_vec()).unwrap();

        // Should only have headers
        assert_eq!(csv.trim(), "id,name");
    }

    #[tokio::test]
    async fn test_csv_boolean_values() {
        let data = vec![TestData {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            active: true,
        }];

        let exporter = CsvExporter::new()
            .from_data(&data)
            .unwrap()
            .columns(&["id", "active"]);

        let bytes = exporter.export().await.unwrap();
        let csv = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(csv.contains("true"));
    }

    #[tokio::test]
    async fn test_value_to_string_conversions() {
        assert_eq!(value_to_string(&serde_json::Value::Null), "");
        assert_eq!(value_to_string(&serde_json::json!(true)), "true");
        assert_eq!(value_to_string(&serde_json::json!(42)), "42");
        assert_eq!(value_to_string(&serde_json::json!("hello")), "hello");
    }

    #[tokio::test]
    async fn test_csv_with_special_characters() {
        #[derive(Serialize)]
        struct SpecialData {
            text: String,
        }

        let data = vec![SpecialData {
            text: "Hello, \"World\"".to_string(),
        }];

        let exporter = CsvExporter::new()
            .from_data(&data)
            .unwrap()
            .columns(&["text"]);

        let bytes = exporter.export().await.unwrap();
        let csv = String::from_utf8(bytes.to_vec()).unwrap();

        // CSV should properly escape quotes
        assert!(csv.contains("Hello"));
    }
}
