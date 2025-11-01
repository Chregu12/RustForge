# Foundry Export

PDF, Excel, and CSV export library for Foundry Core.

## Features

- **PDF Export**: Generate PDFs with printpdf
- **Excel/XLSX Export**: Create styled Excel files
- **CSV Export**: Simple CSV generation
- **Template Support**: Use templates for reports
- **Data Formatting**: Currency, dates, percentages
- **Styling**: Customize fonts, colors, alignment

## Quick Start

```rust
use foundry_export::{ExportData, ExportFormat, Exporter};
use serde_json::json;

let data = ExportData::from_json(json!([
    {"name": "Alice", "age": 30, "email": "alice@example.com"},
    {"name": "Bob", "age": 25, "email": "bob@example.com"}
]))?;

let exporter = Exporter::new();
let pdf_bytes = exporter.export(data.clone(), ExportFormat::Pdf)?;
let xlsx_bytes = exporter.export(data.clone(), ExportFormat::Xlsx)?;
let csv_bytes = exporter.export(data, ExportFormat::Csv)?;

// Save to file
exporter.export_to_file(data, ExportFormat::Pdf, "output.pdf")?;
```

## CLI Commands

```bash
# Export to PDF
foundry export:pdf users.pdf --title "User Report"

# Export to Excel
foundry export:excel users.xlsx

# Export to CSV
foundry export:csv users.csv

# Generate custom export class
foundry make:export UserExport
```

## Excel Styling

```rust
use foundry_export::{ExcelExporter, ExcelStyle};

let style = ExcelStyle {
    header_bold: true,
    header_bg_color: Some("#4472C4".to_string()),
    freeze_header: true,
    auto_filter: true,
};

let exporter = ExcelExporter::new().with_style(style);
let bytes = exporter.export(&data)?;
```

## Data Formatting

```rust
use foundry_export::{DataFormatter, FormatOptions};

let options = FormatOptions {
    date_format: "%Y-%m-%d".to_string(),
    datetime_format: "%Y-%m-%d %H:%M:%S".to_string(),
    currency_symbol: "$".to_string(),
    decimal_places: 2,
};

let formatter = DataFormatter::new(options);
let currency = formatter.format_currency(1234.56); // "$1234.56"
let percentage = formatter.format_percentage(0.125); // "12.50%"
```

## Report Generation

```rust
use foundry_export::{Report, ReportSection};

let report = Report::builder("Q1 Financial Report")
    .author("Finance Team")
    .text_section("Summary", "This report covers Q1 2024...")
    .data_section("Revenue", revenue_data)
    .data_section("Expenses", expense_data)
    .build();

let pdf = report.export(ExportFormat::Pdf)?;
```
