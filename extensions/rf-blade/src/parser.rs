//! Blade template parser
//!
//! Parses Blade syntax into an AST for compilation

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Syntax error: {0}")]
    SyntaxError(String),

    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
}

pub type ParseResult<T> = Result<T, ParseError>;

/// Parsed template
#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    /// Raw content
    pub content: String,

    /// Parent template (from @extends)
    pub extends: Option<String>,

    /// Sections (from @section/@endsection)
    pub sections: HashMap<String, String>,

    /// Yield positions (from @yield)
    pub yields: Vec<String>,

    /// Directives
    pub directives: Vec<Directive>,
}

/// Directive type
#[derive(Debug, Clone)]
pub enum Directive {
    /// @if($condition)
    If { condition: String, content: String },

    /// @foreach($items as $item)
    ForEach {
        items: String,
        item: String,
        content: String,
    },

    /// @auth
    Auth { content: String },

    /// @guest
    Guest { content: String },

    /// Custom directive
    Custom { name: String, args: String },
}

/// Blade template parser
pub struct BladeParser;

impl Default for BladeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BladeParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a Blade template
    pub fn parse(&self, content: &str) -> ParseResult<ParsedTemplate> {
        let mut parsed = ParsedTemplate {
            content: content.to_string(),
            extends: None,
            sections: HashMap::new(),
            yields: Vec::new(),
            directives: Vec::new(),
        };

        // Extract @extends
        if let Some(parent) = self.extract_extends(content)? {
            parsed.extends = Some(parent);
        }

        // Extract @section/@endsection pairs
        parsed.sections = self.extract_sections(content)?;

        // Extract @yield directives
        parsed.yields = self.extract_yields(content)?;

        // Extract other directives
        parsed.directives = self.extract_directives(content)?;

        Ok(parsed)
    }

    /// Extract @extends directive
    fn extract_extends(&self, content: &str) -> ParseResult<Option<String>> {
        let re = regex::Regex::new(r#"@extends\(['"](.*?)['"]\)"#).unwrap();

        if let Some(caps) = re.captures(content) {
            Ok(Some(caps[1].to_string()))
        } else {
            Ok(None)
        }
    }

    /// Extract @section/@endsection pairs
    fn extract_sections(&self, content: &str) -> ParseResult<HashMap<String, String>> {
        let mut sections = HashMap::new();

        // Match inline sections: @section('name', 'content')
        let inline_re =
            regex::Regex::new(r#"@section\(['"](.*?)['"]\s*,\s*['"](.*?)['"]\)"#).unwrap();

        for caps in inline_re.captures_iter(content) {
            let name = caps[1].to_string();
            let content = caps[2].to_string();
            sections.insert(name, content);
        }

        // Match block sections: @section('name') ... @endsection
        let block_re =
            regex::Regex::new(r#"(?s)@section\(['"](.*?)['"]\)(.*?)@endsection"#).unwrap();

        for caps in block_re.captures_iter(content) {
            let name = caps[1].to_string();
            let content = caps[2].trim().to_string();
            sections.insert(name, content);
        }

        Ok(sections)
    }

    /// Extract @yield directives
    fn extract_yields(&self, content: &str) -> ParseResult<Vec<String>> {
        let re = regex::Regex::new(r#"@yield\(['"](.*?)['"]\)"#).unwrap();

        let yields = re
            .captures_iter(content)
            .map(|caps| caps[1].to_string())
            .collect();

        Ok(yields)
    }

    /// Extract other directives (@if, @foreach, etc.)
    fn extract_directives(&self, content: &str) -> ParseResult<Vec<Directive>> {
        let mut directives = Vec::new();

        // @if ... @endif
        let if_re = regex::Regex::new(r"(?s)@if\((.*?)\)(.*?)@endif").unwrap();

        for caps in if_re.captures_iter(content) {
            directives.push(Directive::If {
                condition: caps[1].trim().to_string(),
                content: caps[2].trim().to_string(),
            });
        }

        // @foreach ... @endforeach
        let foreach_re =
            regex::Regex::new(r"(?s)@foreach\(\$(.*?)\s+as\s+\$(.*?)\)(.*?)@endforeach").unwrap();

        for caps in foreach_re.captures_iter(content) {
            directives.push(Directive::ForEach {
                items: caps[1].trim().to_string(),
                item: caps[2].trim().to_string(),
                content: caps[3].trim().to_string(),
            });
        }

        // @auth ... @endauth
        let auth_re = regex::Regex::new(r"(?s)@auth(.*?)@endauth").unwrap();

        for caps in auth_re.captures_iter(content) {
            directives.push(Directive::Auth {
                content: caps[1].trim().to_string(),
            });
        }

        // @guest ... @endguest
        let guest_re = regex::Regex::new(r"(?s)@guest(.*?)@endguest").unwrap();

        for caps in guest_re.captures_iter(content) {
            directives.push(Directive::Guest {
                content: caps[1].trim().to_string(),
            });
        }

        Ok(directives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extends() {
        let parser = BladeParser::new();

        let content = "@extends('layouts.app')";
        let parsed = parser.parse(content).unwrap();

        assert_eq!(parsed.extends, Some("layouts.app".to_string()));
    }

    #[test]
    fn test_parse_inline_section() {
        let parser = BladeParser::new();

        let content = "@section('title', 'My Page')";
        let parsed = parser.parse(content).unwrap();

        assert_eq!(parsed.sections.get("title"), Some(&"My Page".to_string()));
    }

    #[test]
    fn test_parse_block_section() {
        let parser = BladeParser::new();

        let content = "@section('content')\n<h1>Hello</h1>\n@endsection";
        let parsed = parser.parse(content).unwrap();

        assert!(parsed.sections.contains_key("content"));
        assert!(parsed
            .sections
            .get("content")
            .unwrap()
            .contains("<h1>Hello</h1>"));
    }

    #[test]
    fn test_parse_yield() {
        let parser = BladeParser::new();

        let content = "@yield('content')";
        let parsed = parser.parse(content).unwrap();

        assert_eq!(parsed.yields, vec!["content".to_string()]);
    }

    #[test]
    fn test_parse_if_directive() {
        let parser = BladeParser::new();

        let content = "@if($user)\n<p>Welcome!</p>\n@endif";
        let parsed = parser.parse(content).unwrap();

        assert_eq!(parsed.directives.len(), 1);

        match &parsed.directives[0] {
            Directive::If { condition, content } => {
                assert_eq!(condition, "$user");
                assert!(content.contains("Welcome"));
            }
            _ => panic!("Expected If directive"),
        }
    }

    #[test]
    fn test_parse_foreach_directive() {
        let parser = BladeParser::new();

        let content = "@foreach($posts as $post)\n<p>{{ $post }}</p>\n@endforeach";
        let parsed = parser.parse(content).unwrap();

        assert_eq!(parsed.directives.len(), 1);

        match &parsed.directives[0] {
            Directive::ForEach {
                items,
                item,
                content,
            } => {
                assert_eq!(items, "posts");
                assert_eq!(item, "post");
                assert!(content.contains("{{ $post }}"));
            }
            _ => panic!("Expected ForEach directive"),
        }
    }
}
