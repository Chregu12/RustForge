//! Form builder implementation

use crate::field::Field;
use crate::theme::Theme;
use crate::{FormData, FormErrors};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FormMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Fluent form builder
#[derive(Debug, Clone)]
pub struct Form {
    pub name: String,
    pub action: String,
    pub method: FormMethod,
    pub fields: Vec<Field>,
    pub submit_text: String,
    pub csrf_enabled: bool,
    pub enctype: Option<String>,
}

impl Form {
    pub fn new(name: impl Into<String>) -> FormBuilder {
        FormBuilder::new(name)
    }

    pub fn render(&self, theme: Theme) -> anyhow::Result<String> {
        crate::renderer::FormRenderer::new(theme).render(self)
    }

    pub fn render_with_data(
        &self,
        theme: Theme,
        data: &FormData,
        errors: &FormErrors,
    ) -> anyhow::Result<String> {
        crate::renderer::FormRenderer::new(theme).render_with_data(self, data, errors)
    }

    pub fn validate(&self, data: &FormData) -> Result<(), FormErrors> {
        let mut errors = FormErrors::new();

        for field in &self.fields {
            if let Some(rules) = &field.validation {
                let value = data.get(&field.name).map(|s| s.as_str());

                for rule in rules {
                    if let Err(e) = rule.validate(&field.name, value) {
                        errors.add(&field.name, e.message);
                    }
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

/// Form builder
pub struct FormBuilder {
    name: String,
    action: String,
    method: FormMethod,
    fields: Vec<Field>,
    submit_text: String,
    csrf_enabled: bool,
    enctype: Option<String>,
}

impl FormBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            action: "#".to_string(),
            method: FormMethod::Post,
            fields: Vec::new(),
            submit_text: "Submit".to_string(),
            csrf_enabled: true,
            enctype: None,
        }
    }

    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }

    pub fn method(mut self, method: FormMethod) -> Self {
        self.method = method;
        self
    }

    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    pub fn submit(mut self, text: impl Into<String>) -> Self {
        self.submit_text = text.into();
        self
    }

    pub fn csrf(mut self, enabled: bool) -> Self {
        self.csrf_enabled = enabled;
        self
    }

    pub fn multipart(mut self) -> Self {
        self.enctype = Some("multipart/form-data".to_string());
        self
    }

    pub fn build(self) -> Form {
        Form {
            name: self.name,
            action: self.action,
            method: self.method,
            fields: self.fields,
            submit_text: self.submit_text,
            csrf_enabled: self.csrf_enabled,
            enctype: self.enctype,
        }
    }
}
