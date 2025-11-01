//! Form HTML renderer

use crate::builder::{Form, FormMethod};
use crate::field::{Field, FieldType, InputType};
use crate::theme::Theme;
use crate::{FormData, FormErrors};

/// Form renderer
pub struct FormRenderer {
    theme: Theme,
}

impl FormRenderer {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    pub fn render(&self, form: &Form) -> anyhow::Result<String> {
        self.render_with_data(form, &FormData::new(), &FormErrors::new())
    }

    pub fn render_with_data(
        &self,
        form: &Form,
        data: &FormData,
        errors: &FormErrors,
    ) -> anyhow::Result<String> {
        let mut html = String::new();

        // Form opening tag
        let method = match form.method {
            FormMethod::Get => "GET",
            _ => "POST",
        };

        let enctype = form
            .enctype
            .as_deref()
            .map(|e| format!(r#" enctype="{}""#, e))
            .unwrap_or_default();

        html.push_str(&format!(
            r#"<form name="{}" action="{}" method="{}" class="{}"{}>"#,
            form.name,
            form.action,
            method,
            self.theme.form_class(),
            enctype
        ));

        // CSRF token
        if form.csrf_enabled {
            html.push_str(r#"<input type="hidden" name="_csrf" value="TOKEN_HERE">"#);
        }

        // Method spoofing for PUT/PATCH/DELETE
        if !matches!(form.method, FormMethod::Get | FormMethod::Post) {
            let method_value = match form.method {
                FormMethod::Put => "PUT",
                FormMethod::Patch => "PATCH",
                FormMethod::Delete => "DELETE",
                _ => "",
            };
            html.push_str(&format!(
                r#"<input type="hidden" name="_method" value="{}">"#,
                method_value
            ));
        }

        // Render fields
        for field in &form.fields {
            html.push_str(&self.render_field(field, data, errors)?);
        }

        // Submit button
        html.push_str(&format!(
            r#"<div class="{}"><button type="submit" class="{}">{}</button></div>"#,
            self.theme.field_wrapper_class(),
            self.theme.submit_class(),
            form.submit_text
        ));

        html.push_str("</form>");

        Ok(html)
    }

    fn render_field(
        &self,
        field: &Field,
        data: &FormData,
        errors: &FormErrors,
    ) -> anyhow::Result<String> {
        let mut html = String::new();

        if matches!(field.field_type, FieldType::Hidden) {
            let value = field
                .default_value
                .as_deref()
                .or_else(|| data.get(&field.name).map(|s| s.as_str()))
                .unwrap_or("");
            html.push_str(&format!(
                r#"<input type="hidden" name="{}" value="{}">"#,
                field.name, value
            ));
            return Ok(html);
        }

        html.push_str(&format!(r#"<div class="{}">"#, self.theme.field_wrapper_class()));

        // Label
        if let Some(label) = &field.label {
            html.push_str(&format!(
                r#"<label for="{}" class="{}">{}</label>"#,
                field.name,
                self.theme.label_class(),
                label
            ));
        }

        // Field input
        let has_error = errors.get(&field.name).is_some();
        let value = data
            .get(&field.name)
            .map(|s| s.as_str())
            .or(field.default_value.as_deref())
            .unwrap_or("");

        match &field.field_type {
            FieldType::Input { input_type } => {
                let input_class = if has_error {
                    format!("{} {}", self.theme.input_class(), self.theme.input_error_class())
                } else {
                    self.theme.input_class().to_string()
                };

                let type_str = match input_type {
                    InputType::Text => "text",
                    InputType::Email => "email",
                    InputType::Password => "password",
                    InputType::Number => "number",
                    InputType::Tel => "tel",
                    InputType::Url => "url",
                    InputType::Date => "date",
                    InputType::Time => "time",
                    InputType::DateTime => "datetime-local",
                    InputType::Color => "color",
                    InputType::Range => "range",
                };

                let placeholder = field
                    .placeholder
                    .as_ref()
                    .map(|p| format!(r#" placeholder="{}""#, p))
                    .unwrap_or_default();

                html.push_str(&format!(
                    r#"<input type="{}" id="{}" name="{}" value="{}" class="{}"{}>"#,
                    type_str, field.name, field.name, value, input_class, placeholder
                ));
            }
            FieldType::TextArea { rows } => {
                let textarea_class = if has_error {
                    format!("{} {}", self.theme.textarea_class(), self.theme.input_error_class())
                } else {
                    self.theme.textarea_class().to_string()
                };

                html.push_str(&format!(
                    r#"<textarea id="{}" name="{}" rows="{}" class="{}">{}</textarea>"#,
                    field.name, field.name, rows, textarea_class, value
                ));
            }
            FieldType::Select { options, .. } => {
                html.push_str(&format!(
                    r#"<select id="{}" name="{}" class="{}">"#,
                    field.name,
                    field.name,
                    self.theme.select_class()
                ));
                for option in options {
                    let selected = if option.value == value {
                        " selected"
                    } else {
                        ""
                    };
                    html.push_str(&format!(
                        r#"<option value="{}"{}>{}></option>"#,
                        option.value, selected, option.label
                    ));
                }
                html.push_str("</select>");
            }
            FieldType::Checkbox { .. } => {
                html.push_str(&format!(
                    r#"<input type="checkbox" id="{}" name="{}" class="{}">"#,
                    field.name,
                    field.name,
                    self.theme.checkbox_class()
                ));
            }
            FieldType::File { accept, multiple } => {
                let accept_attr = accept
                    .as_ref()
                    .map(|a| format!(r#" accept="{}""#, a))
                    .unwrap_or_default();
                let multiple_attr = if *multiple { " multiple" } else { "" };
                html.push_str(&format!(
                    r#"<input type="file" id="{}" name="{}" class="{}"{}{}">"#,
                    field.name,
                    field.name,
                    self.theme.input_class(),
                    accept_attr,
                    multiple_attr
                ));
            }
            _ => {}
        }

        // Help text
        if let Some(help) = &field.help_text {
            html.push_str(&format!(
                r#"<div class="{}">{}</div>"#,
                self.theme.help_class(),
                help
            ));
        }

        // Errors
        if let Some(field_errors) = errors.get(&field.name) {
            for error in field_errors {
                html.push_str(&format!(
                    r#"<div class="{}">{}</div>"#,
                    self.theme.error_class(),
                    error
                ));
            }
        }

        html.push_str("</div>");

        Ok(html)
    }
}
