//! Form themes (Bootstrap, Tailwind, etc.)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Bootstrap,
    Tailwind,
    Plain,
}

impl Theme {
    pub fn form_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "needs-validation",
            Theme::Tailwind => "space-y-4",
            Theme::Plain => "",
        }
    }

    pub fn field_wrapper_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "mb-3",
            Theme::Tailwind => "mb-4",
            Theme::Plain => "field-group",
        }
    }

    pub fn label_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "form-label",
            Theme::Tailwind => "block text-sm font-medium text-gray-700 mb-1",
            Theme::Plain => "label",
        }
    }

    pub fn input_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "form-control",
            Theme::Tailwind => "mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500",
            Theme::Plain => "input",
        }
    }

    pub fn input_error_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "is-invalid",
            Theme::Tailwind => "border-red-500 focus:border-red-500 focus:ring-red-500",
            Theme::Plain => "input-error",
        }
    }

    pub fn textarea_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "form-control",
            Theme::Tailwind => "mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500",
            Theme::Plain => "textarea",
        }
    }

    pub fn select_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "form-select",
            Theme::Tailwind => "mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500",
            Theme::Plain => "select",
        }
    }

    pub fn checkbox_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "form-check-input",
            Theme::Tailwind => "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
            Theme::Plain => "checkbox",
        }
    }

    pub fn submit_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "btn btn-primary",
            Theme::Tailwind => "px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2",
            Theme::Plain => "submit-button",
        }
    }

    pub fn error_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "invalid-feedback",
            Theme::Tailwind => "mt-1 text-sm text-red-600",
            Theme::Plain => "error-message",
        }
    }

    pub fn help_class(&self) -> &str {
        match self {
            Theme::Bootstrap => "form-text",
            Theme::Tailwind => "mt-1 text-sm text-gray-500",
            Theme::Plain => "help-text",
        }
    }
}
