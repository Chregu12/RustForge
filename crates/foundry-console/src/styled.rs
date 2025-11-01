use crate::colors::Colorize;

/// Apply bold styling to text
pub fn bold(text: &str) -> String {
    text.bold()
}

/// Apply italic styling to text
pub fn italic(text: &str) -> String {
    text.italic()
}

/// Apply underline styling to text
pub fn underline(text: &str) -> String {
    text.underline()
}

/// Apply dim styling to text
pub fn dim(text: &str) -> String {
    text.dim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_functions() {
        let text = bold("test");
        assert!(text.contains("test"));

        let text = italic("test");
        assert!(text.contains("test"));
    }
}
