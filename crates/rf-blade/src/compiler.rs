//! Blade template compiler
//!
//! Compiles parsed Blade templates into executable HTML

use crate::parser::{Directive, ParsedTemplate};
use crate::{BladeResult, CompiledTemplate};

/// Blade template compiler
pub struct BladeCompiler;

impl BladeCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a parsed template
    pub fn compile(&self, name: &str, parsed: ParsedTemplate) -> BladeResult<CompiledTemplate> {
        let mut html = parsed.content.clone();

        // Remove @extends directive from content
        if parsed.extends.is_some() {
            html = self.remove_extends(&html);
        }

        // Remove @section directives from content (they're in sections map)
        html = self.remove_sections(&html);

        // Keep @yield directives as markers
        // They will be replaced during rendering

        // Compile directives
        html = self.compile_directives(&html, &parsed.directives)?;

        // Remove comments {{-- --}}
        html = self.remove_comments(&html);

        Ok(CompiledTemplate {
            name: name.to_string(),
            html,
            parent: parsed.extends,
            sections: parsed.sections,
            components: Vec::new(),
        })
    }

    /// Remove @extends directive
    fn remove_extends(&self, html: &str) -> String {
        let re = regex::Regex::new(r#"@extends\(['"](.*?)['"]\)"#).unwrap();
        re.replace_all(html, "").to_string()
    }

    /// Remove @section/@endsection blocks
    fn remove_sections(&self, html: &str) -> String {
        let mut result = html.to_string();

        // Remove inline sections
        let inline_re =
            regex::Regex::new(r#"@section\(['"](.*?)['"]\s*,\s*['"](.*?)['"]\)"#).unwrap();
        result = inline_re.replace_all(&result, "").to_string();

        // Remove block sections
        let block_re =
            regex::Regex::new(r#"(?s)@section\(['"](.*?)['"]\)(.*?)@endsection"#).unwrap();
        result = block_re.replace_all(&result, "").to_string();

        result
    }

    /// Compile directives into placeholder HTML
    fn compile_directives(&self, html: &str, _directives: &[Directive]) -> BladeResult<String> {
        let mut result = html.to_string();

        // Compile @if directives
        let if_re = regex::Regex::new(r"(?s)@if\((.*?)\)(.*?)@endif").unwrap();
        result = if_re
            .replace_all(&result, |caps: &regex::Captures| {
                let condition = &caps[1];
                let content = &caps[2];

                // For now, we'll keep a placeholder that gets evaluated during render
                // In a full implementation, this would compile to actual logic
                format!("<!-- if {} -->{}", condition, content)
            })
            .to_string();

        // Compile @foreach directives
        let foreach_re =
            regex::Regex::new(r"(?s)@foreach\(\$(.*?)\s+as\s+\$(.*?)\)(.*?)@endforeach").unwrap();
        result = foreach_re
            .replace_all(&result, |caps: &regex::Captures| {
                let items = &caps[1];
                let item = &caps[2];
                let content = &caps[3];

                // Generate loop code with proper iteration
                format!(
                    "{{{{ for {} in &{} }}}}\n{}\n{{{{ endfor }}}}",
                    item, items, content
                )
            })
            .to_string();

        // Compile @auth directives
        let auth_re = regex::Regex::new(r"(?s)@auth(.*?)@endauth").unwrap();
        result = auth_re
            .replace_all(&result, |caps: &regex::Captures| {
                let content = &caps[1];
                format!("<!-- auth -->{}", content)
            })
            .to_string();

        // Compile @guest directives
        let guest_re = regex::Regex::new(r"(?s)@guest(.*?)@endguest").unwrap();
        result = guest_re
            .replace_all(&result, |caps: &regex::Captures| {
                let content = &caps[1];
                format!("<!-- guest -->{}", content)
            })
            .to_string();

        Ok(result)
    }

    /// Remove Blade comments {{-- --}}
    fn remove_comments(&self, html: &str) -> String {
        let re = regex::Regex::new(r"\{\{--.*?--\}\}").unwrap();
        re.replace_all(html, "").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BladeParser;

    #[test]
    fn test_compile_simple_template() {
        let parser = BladeParser::new();
        let compiler = BladeCompiler::new();

        let content = "<h1>{{ $title }}</h1>";
        let parsed = parser.parse(content).unwrap();
        let compiled = compiler.compile("test", parsed).unwrap();

        assert!(compiled.html.contains("{{ $title }}"));
    }

    #[test]
    fn test_compile_with_extends() {
        let parser = BladeParser::new();
        let compiler = BladeCompiler::new();

        let content = "@extends('layouts.app')\n<h1>Content</h1>";
        let parsed = parser.parse(content).unwrap();
        let compiled = compiler.compile("test", parsed).unwrap();

        assert_eq!(compiled.parent, Some("layouts.app".to_string()));
        assert!(!compiled.html.contains("@extends"));
    }

    #[test]
    fn test_compile_sections() {
        let parser = BladeParser::new();
        let compiler = BladeCompiler::new();

        let content = "@section('content')\n<p>Hello</p>\n@endsection";
        let parsed = parser.parse(content).unwrap();
        let compiled = compiler.compile("test", parsed).unwrap();

        assert!(compiled.sections.contains_key("content"));
        assert!(!compiled.html.contains("@section"));
        assert!(!compiled.html.contains("@endsection"));
    }

    #[test]
    fn test_remove_comments() {
        let compiler = BladeCompiler::new();

        let html = "<p>Hello</p>{{-- This is a comment --}}<p>World</p>";
        let result = compiler.remove_comments(html);

        assert_eq!(result, "<p>Hello</p><p>World</p>");
    }
}
