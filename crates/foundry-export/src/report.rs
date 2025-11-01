//! Report generation with sections and templates

use crate::{ExportData, ExportFormat, Exporter};
use serde::{Deserialize, Serialize};

/// Report with multiple sections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub title: String,
    pub author: Option<String>,
    pub sections: Vec<ReportSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: SectionContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SectionContent {
    Text { text: String },
    Data { data: ExportData },
    Chart { chart_data: serde_json::Value },
}

/// Builder for reports
pub struct ReportBuilder {
    title: String,
    author: Option<String>,
    sections: Vec<ReportSection>,
}

impl ReportBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            author: None,
            sections: Vec::new(),
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn section(mut self, section: ReportSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn text_section(mut self, title: impl Into<String>, text: impl Into<String>) -> Self {
        self.sections.push(ReportSection {
            title: title.into(),
            content: SectionContent::Text { text: text.into() },
        });
        self
    }

    pub fn data_section(mut self, title: impl Into<String>, data: ExportData) -> Self {
        self.sections.push(ReportSection {
            title: title.into(),
            content: SectionContent::Data { data },
        });
        self
    }

    pub fn build(self) -> Report {
        Report {
            title: self.title,
            author: self.author,
            sections: self.sections,
        }
    }
}

impl Report {
    pub fn builder(title: impl Into<String>) -> ReportBuilder {
        ReportBuilder::new(title)
    }

    pub fn export(&self, format: ExportFormat) -> anyhow::Result<Vec<u8>> {
        let exporter = Exporter::new();

        // For now, just export the first data section
        // In a real implementation, we'd combine all sections
        for section in &self.sections {
            if let SectionContent::Data { data } = &section.content {
                return exporter.export(data.clone(), format);
            }
        }

        Err(anyhow::anyhow!("No data sections found in report"))
    }
}
