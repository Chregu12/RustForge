//! Form field definitions

use crate::validation::ValidationRule;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub default_value: Option<String>,
    pub help_text: Option<String>,
    pub validation: Option<Vec<ValidationRule>>,
    pub attributes: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldType {
    Input {
        input_type: InputType,
    },
    TextArea {
        rows: usize,
    },
    Select {
        options: Vec<SelectOption>,
        multiple: bool,
    },
    Checkbox {
        checked: bool,
    },
    Radio {
        options: Vec<SelectOption>,
    },
    File {
        accept: Option<String>,
        multiple: bool,
    },
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    Text,
    Email,
    Password,
    Number,
    Tel,
    Url,
    Date,
    Time,
    DateTime,
    Color,
    Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl Field {
    pub fn text(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name, FieldType::Input {
            input_type: InputType::Text,
        })
    }

    pub fn email(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name, FieldType::Input {
            input_type: InputType::Email,
        })
    }

    pub fn password(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name, FieldType::Input {
            input_type: InputType::Password,
        })
    }

    pub fn number(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name, FieldType::Input {
            input_type: InputType::Number,
        })
    }

    pub fn textarea(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name, FieldType::TextArea { rows: 4 })
    }

    pub fn select(name: impl Into<String>, options: Vec<SelectOption>) -> FieldBuilder {
        FieldBuilder::new(
            name,
            FieldType::Select {
                options,
                multiple: false,
            },
        )
    }

    pub fn checkbox(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(name, FieldType::Checkbox { checked: false })
    }

    pub fn file(name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(
            name,
            FieldType::File {
                accept: None,
                multiple: false,
            },
        )
    }

    pub fn hidden(name: impl Into<String>, value: impl Into<String>) -> Field {
        Field {
            name: name.into(),
            field_type: FieldType::Hidden,
            label: None,
            placeholder: None,
            default_value: Some(value.into()),
            help_text: None,
            validation: None,
            attributes: Vec::new(),
        }
    }
}

/// Field builder
pub struct FieldBuilder {
    name: String,
    field_type: FieldType,
    label: Option<String>,
    placeholder: Option<String>,
    default_value: Option<String>,
    help_text: Option<String>,
    validation: Vec<ValidationRule>,
    attributes: Vec<(String, String)>,
}

impl FieldBuilder {
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            label: None,
            placeholder: None,
            default_value: None,
            help_text: None,
            validation: Vec::new(),
            attributes: Vec::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help_text = Some(text.into());
        self
    }

    pub fn required(mut self) -> Self {
        self.validation.push(ValidationRule::Required);
        self
    }

    pub fn min_length(mut self, length: usize) -> Self {
        self.validation.push(ValidationRule::MinLength(length));
        self
    }

    pub fn max_length(mut self, length: usize) -> Self {
        self.validation.push(ValidationRule::MaxLength(length));
        self
    }

    pub fn pattern(mut self, pattern: String) -> Self {
        self.validation.push(ValidationRule::Pattern(pattern));
        self
    }

    pub fn rule(mut self, rule: ValidationRule) -> Self {
        self.validation.push(rule);
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> Field {
        Field {
            name: self.name,
            field_type: self.field_type,
            label: self.label,
            placeholder: self.placeholder,
            default_value: self.default_value,
            help_text: self.help_text,
            validation: if self.validation.is_empty() {
                None
            } else {
                Some(self.validation)
            },
            attributes: self.attributes,
        }
    }
}
