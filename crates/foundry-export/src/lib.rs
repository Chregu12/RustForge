//! Foundry Export - PDF, Excel, and CSV Export Library
//!
//! Provides flexible data export capabilities with support for:
//! - PDF generation with templates
//! - Excel/XLSX export with styling
//! - CSV export with custom delimiters
//! - Template-based reports
//! - Data formatting helpers
//!
//! # Example
//!
//! ```no_run
//! use foundry_export::{ExportFormat, Exporter, ExportData};
//!
//! let data = ExportData::from_json(json!([
//!     {"name": "John", "age": 30},
//!     {"name": "Jane", "age": 25}
//! ]));
//!
//! let exporter = Exporter::new();
//! let output = exporter.export(data, ExportFormat::Xlsx)?;
//! ```

pub mod csv;
pub mod excel;
pub mod format;
pub mod pdf;
pub mod report;
pub mod style;
pub mod template;

pub use csv::CsvExporter;
pub use excel::{ExcelExporter, ExcelStyle};
pub use format::{DataFormatter, FormatOptions};
pub use pdf::{PdfExporter, PdfOptions};
pub use report::{Report, ReportBuilder, ReportSection};
pub use style::{CellStyle, Color, Font};
pub use template::{TemplateEngine, TemplateRenderer};

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Export format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Pdf,
    Xlsx,
    Csv,
}

/// Main exporter interface
pub struct Exporter {
    pdf: PdfExporter,
    excel: ExcelExporter,
    csv: CsvExporter,
}

impl Exporter {
    pub fn new() -> Self {
        Self {
            pdf: PdfExporter::new(),
            excel: ExcelExporter::new(),
            csv: CsvExporter::new(),
        }
    }

    pub fn export(
        &self,
        data: ExportData,
        format: ExportFormat,
    ) -> anyhow::Result<Vec<u8>> {
        match format {
            ExportFormat::Pdf => self.pdf.export(&data),
            ExportFormat::Xlsx => self.excel.export(&data),
            ExportFormat::Csv => self.csv.export(&data),
        }
    }

    pub fn export_to_file(
        &self,
        data: ExportData,
        format: ExportFormat,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let bytes = self.export(data, format)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Data structure for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub metadata: ExportMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub created_at: String,
}

impl ExportData {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self {
            headers,
            rows,
            metadata: ExportMetadata {
                title: None,
                author: None,
                subject: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.metadata.title = Some(title.into());
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = Some(author.into());
        self
    }

    pub fn from_json(value: serde_json::Value) -> anyhow::Result<Self> {
        if let Some(arr) = value.as_array() {
            if arr.is_empty() {
                return Ok(Self::new(vec![], vec![]));
            }

            // Extract headers from first object
            let headers = if let Some(first) = arr.first().and_then(|v| v.as_object()) {
                first.keys().cloned().collect()
            } else {
                vec![]
            };

            // Extract rows
            let rows: Vec<Vec<String>> = arr
                .iter()
                .filter_map(|v| v.as_object())
                .map(|obj| {
                    headers
                        .iter()
                        .map(|h| {
                            obj.get(h)
                                .map(|v| v.to_string().trim_matches('"').to_string())
                                .unwrap_or_default()
                        })
                        .collect()
                })
                .collect();

            Ok(Self::new(headers, rows))
        } else {
            Err(anyhow::anyhow!("Expected array of objects"))
        }
    }
}
