/// ANSI color codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    pub fn fg_code(&self) -> &str {
        match self {
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            Color::BrightBlack => "90",
            Color::BrightRed => "91",
            Color::BrightGreen => "92",
            Color::BrightYellow => "93",
            Color::BrightBlue => "94",
            Color::BrightMagenta => "95",
            Color::BrightCyan => "96",
            Color::BrightWhite => "97",
        }
    }

    pub fn bg_code(&self) -> &str {
        match self {
            Color::Black => "40",
            Color::Red => "41",
            Color::Green => "42",
            Color::Yellow => "43",
            Color::Blue => "44",
            Color::Magenta => "45",
            Color::Cyan => "46",
            Color::White => "47",
            Color::BrightBlack => "100",
            Color::BrightRed => "101",
            Color::BrightGreen => "102",
            Color::BrightYellow => "103",
            Color::BrightBlue => "104",
            Color::BrightMagenta => "105",
            Color::BrightCyan => "106",
            Color::BrightWhite => "107",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Bold,
    Dim,
    Italic,
    Underline,
}

impl Style {
    pub fn code(&self) -> &str {
        match self {
            Style::Bold => "1",
            Style::Dim => "2",
            Style::Italic => "3",
            Style::Underline => "4",
        }
    }
}

/// Extension trait for colorizing strings
pub trait Colorize {
    fn color(&self, color: Color) -> String;
    fn bg_color(&self, color: Color) -> String;
    fn style(&self, style: Style) -> String;

    // Foreground colors
    fn black(&self) -> String { self.color(Color::Black) }
    fn red(&self) -> String { self.color(Color::Red) }
    fn green(&self) -> String { self.color(Color::Green) }
    fn yellow(&self) -> String { self.color(Color::Yellow) }
    fn blue(&self) -> String { self.color(Color::Blue) }
    fn magenta(&self) -> String { self.color(Color::Magenta) }
    fn cyan(&self) -> String { self.color(Color::Cyan) }
    fn white(&self) -> String { self.color(Color::White) }

    // Bright foreground colors
    fn bright_black(&self) -> String { self.color(Color::BrightBlack) }
    fn bright_red(&self) -> String { self.color(Color::BrightRed) }
    fn bright_green(&self) -> String { self.color(Color::BrightGreen) }
    fn bright_yellow(&self) -> String { self.color(Color::BrightYellow) }
    fn bright_blue(&self) -> String { self.color(Color::BrightBlue) }
    fn bright_magenta(&self) -> String { self.color(Color::BrightMagenta) }
    fn bright_cyan(&self) -> String { self.color(Color::BrightCyan) }
    fn bright_white(&self) -> String { self.color(Color::BrightWhite) }

    // Background colors
    fn on_black(&self) -> String { self.bg_color(Color::Black) }
    fn on_red(&self) -> String { self.bg_color(Color::Red) }
    fn on_green(&self) -> String { self.bg_color(Color::Green) }
    fn on_yellow(&self) -> String { self.bg_color(Color::Yellow) }
    fn on_blue(&self) -> String { self.bg_color(Color::Blue) }
    fn on_magenta(&self) -> String { self.bg_color(Color::Magenta) }
    fn on_cyan(&self) -> String { self.bg_color(Color::Cyan) }
    fn on_white(&self) -> String { self.bg_color(Color::White) }

    // Styles
    fn bold(&self) -> String { self.style(Style::Bold) }
    fn dim(&self) -> String { self.style(Style::Dim) }
    fn italic(&self) -> String { self.style(Style::Italic) }
    fn underline(&self) -> String { self.style(Style::Underline) }
}

impl<T: AsRef<str>> Colorize for T {
    fn color(&self, color: Color) -> String {
        format!("\x1b[{}m{}\x1b[0m", color.fg_code(), self.as_ref())
    }

    fn bg_color(&self, color: Color) -> String {
        format!("\x1b[{}m{}\x1b[0m", color.bg_code(), self.as_ref())
    }

    fn style(&self, style: Style) -> String {
        format!("\x1b[{}m{}\x1b[0m", style.code(), self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_codes() {
        assert_eq!(Color::Red.fg_code(), "31");
        assert_eq!(Color::Green.bg_code(), "42");
    }

    #[test]
    fn test_colorize() {
        let text = "test".green();
        assert!(text.contains("test"));
        assert!(text.starts_with("\x1b["));
    }
}
