//! CSV export functionality

use crate::ExportData;
use csv::WriterBuilder;

/// CSV exporter with customizable options
pub struct CsvExporter {
    delimiter: u8,
    quote_all: bool,
}

impl CsvExporter {
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            quote_all: false,
        }
    }

    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn with_quote_all(mut self, quote_all: bool) -> Self {
        self.quote_all = quote_all;
        self
    }

    pub fn export(&self, data: &ExportData) -> anyhow::Result<Vec<u8>> {
        let mut wtr = WriterBuilder::new()
            .delimiter(self.delimiter)
            .quote_style(if self.quote_all {
                csv::QuoteStyle::Always
            } else {
                csv::QuoteStyle::Necessary
            })
            .from_writer(vec![]);

        // Write headers
        wtr.write_record(&data.headers)?;

        // Write data rows
        for row in &data.rows {
            wtr.write_record(row)?;
        }

        wtr.flush()?;
        Ok(wtr.into_inner()?)
    }
}

impl Default for CsvExporter {
    fn default() -> Self {
        Self::new()
    }
}
