//! PDF export functionality

use crate::{ExportData, style::CellStyle};
use printpdf::*;
use std::io::BufWriter;

/// PDF exporter options
#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub page_size: (f32, f32), // width, height in mm
    pub margin: f32,            // in mm
    pub font_size: f32,
    pub title_font_size: f32,
    pub header_style: CellStyle,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page_size: (210.0, 297.0), // A4
            margin: 20.0,
            font_size: 10.0,
            title_font_size: 16.0,
            header_style: CellStyle::default().bold(),
        }
    }
}

/// PDF exporter
pub struct PdfExporter {
    options: PdfOptions,
}

impl PdfExporter {
    pub fn new() -> Self {
        Self {
            options: PdfOptions::default(),
        }
    }

    pub fn with_options(mut self, options: PdfOptions) -> Self {
        self.options = options;
        self
    }

    pub fn export(&self, data: &ExportData) -> anyhow::Result<Vec<u8>> {
        let (doc, page1, layer1) = PdfDocument::new(
            data.metadata.title.as_deref().unwrap_or("Export"),
            Mm(self.options.page_size.0),
            Mm(self.options.page_size.1),
            "Layer 1",
        );

        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Start position
        let mut y_pos = self.options.page_size.1 - self.options.margin;

        // Title
        if let Some(title) = &data.metadata.title {
            current_layer.use_text(
                title,
                self.options.title_font_size,
                Mm(self.options.margin),
                Mm(y_pos),
                &IndirectFontRef::new(0),
            );
            y_pos -= self.options.title_font_size * 1.5;
        }

        // Headers
        let col_width = (self.options.page_size.0 - 2.0 * self.options.margin) / data.headers.len() as f32;
        y_pos -= 10.0;

        for (i, header) in data.headers.iter().enumerate() {
            let x = self.options.margin + (i as f32 * col_width);
            current_layer.use_text(
                header,
                self.options.font_size,
                Mm(x),
                Mm(y_pos),
                &IndirectFontRef::new(0),
            );
        }

        y_pos -= self.options.font_size * 1.5;

        // Data rows
        for row in &data.rows {
            for (i, cell) in row.iter().enumerate() {
                let x = self.options.margin + (i as f32 * col_width);
                current_layer.use_text(
                    cell,
                    self.options.font_size,
                    Mm(x),
                    Mm(y_pos),
                    &IndirectFontRef::new(0),
                );
            }
            y_pos -= self.options.font_size * 1.2;

            // Check if we need a new page
            if y_pos < self.options.margin {
                // TODO: Add new page
                break;
            }
        }

        // Save to buffer
        let mut buffer = Vec::new();
        doc.save(&mut BufWriter::new(&mut buffer))?;
        Ok(buffer)
    }
}

impl Default for PdfExporter {
    fn default() -> Self {
        Self::new()
    }
}
