//! Excel/XLSX export functionality

use crate::ExportData;
use rust_xlsxwriter::{Format, Workbook};

/// Excel exporter with styling support
pub struct ExcelExporter {
    style: ExcelStyle,
}

#[derive(Debug, Clone)]
pub struct ExcelStyle {
    pub header_bold: bool,
    pub header_bg_color: Option<String>,
    pub freeze_header: bool,
    pub auto_filter: bool,
}

impl Default for ExcelStyle {
    fn default() -> Self {
        Self {
            header_bold: true,
            header_bg_color: Some("#4472C4".to_string()),
            freeze_header: true,
            auto_filter: true,
        }
    }
}

impl ExcelExporter {
    pub fn new() -> Self {
        Self {
            style: ExcelStyle::default(),
        }
    }

    pub fn with_style(mut self, style: ExcelStyle) -> Self {
        self.style = style;
        self
    }

    pub fn export(&self, data: &ExportData) -> anyhow::Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        // Configure header format
        let mut header_format = Format::new();
        if self.style.header_bold {
            header_format = header_format.set_bold();
        }
        if let Some(color) = &self.style.header_bg_color {
            header_format = header_format.set_background_color(color.as_str());
        }

        // Write headers
        for (col, header) in data.headers.iter().enumerate() {
            worksheet.write_string_with_format(0, col as u16, header, &header_format)?;
        }

        // Write data rows
        for (row_idx, row) in data.rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                // Try to parse as number first
                if let Ok(num) = cell.parse::<f64>() {
                    worksheet.write_number((row_idx + 1) as u32, col_idx as u16, num)?;
                } else {
                    worksheet.write_string((row_idx + 1) as u32, col_idx as u16, cell)?;
                }
            }
        }

        // Apply freeze panes
        if self.style.freeze_header && !data.rows.is_empty() {
            worksheet.set_freeze_panes(1, 0)?;
        }

        // Apply auto filter
        if self.style.auto_filter && !data.headers.is_empty() && !data.rows.is_empty() {
            worksheet.autofilter(
                0,
                0,
                data.rows.len() as u32,
                (data.headers.len() - 1) as u16,
            )?;
        }

        // Auto-fit columns
        for col in 0..data.headers.len() {
            worksheet.set_column_width(col as u16, 15)?;
        }

        // Save to buffer
        let buffer = workbook.save_to_buffer()?;
        Ok(buffer)
    }
}

impl Default for ExcelExporter {
    fn default() -> Self {
        Self::new()
    }
}
