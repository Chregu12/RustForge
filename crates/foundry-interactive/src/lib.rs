//! Interactive prompt system for Foundry CLI
//!
//! This crate provides an intuitive interface for interactive command-line prompts,
//! similar to Laravel Artisan's interactive features.

mod prompts;
mod error;

pub use prompts::{
    ask, ask_with_default, choice, confirm, password, multi_select, autocomplete,
    PromptOptions, SelectOption,
};
pub use error::{PromptError, PromptResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_option_creation() {
        let option = SelectOption::new("test", "Test Description");
        assert_eq!(option.label, "test");
        assert_eq!(option.description, Some("Test Description".to_string()));
    }
}
