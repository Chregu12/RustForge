//! Blade template lexer
//!
//! Tokenizes Blade templates into a stream of tokens for parsing

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LexerError {
    #[error("Unexpected character: {0}")]
    UnexpectedCharacter(char),

    #[error("Unterminated string at position {0}")]
    UnterminatedString(usize),

    #[error("Unterminated directive at position {0}")]
    UnterminatedDirective(usize),

    #[error("Invalid directive syntax: {0}")]
    InvalidDirective(String),
}

pub type LexerResult<T> = Result<T, LexerError>;

/// Token types in Blade templates
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Plain text content
    Text(String),

    /// Variable interpolation: {{ $var }}
    Variable(String),

    /// Raw output: {!! $html !!}
    RawVariable(String),

    /// Comment: {{-- comment --}}
    Comment(String),

    /// Directive start: @if, @foreach, etc.
    DirectiveStart(DirectiveType, String), // (type, arguments)

    /// Directive end: @endif, @endforeach, etc.
    DirectiveEnd(DirectiveType),

    /// Standalone directive: @csrf, @yield('name'), @section('name', 'value')
    Directive(DirectiveType, String), // (type, arguments)

    /// Component opening tag: <x-alert type="danger">
    ComponentStart { name: String, attributes: Vec<(String, String)>, self_closing: bool },

    /// Component closing tag: </x-alert>
    ComponentEnd(String),

    /// End of file
    Eof,
}

/// Directive types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DirectiveType {
    If,
    ElseIf,
    Else,
    EndIf,
    ForEach,
    EndForEach,
    For,
    EndFor,
    While,
    EndWhile,
    Section,
    EndSection,
    Yield,
    Extends,
    Include,
    Auth,
    EndAuth,
    Guest,
    EndGuest,
    Csrf,
    Method,
    Json,
    Dump,
    Error,
    Slot,
    EndSlot,
    Props,
    Custom(String),
}

impl DirectiveType {
    /// Parse directive type from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "if" => Self::If,
            "elseif" => Self::ElseIf,
            "else" => Self::Else,
            "endif" => Self::EndIf,
            "foreach" => Self::ForEach,
            "endforeach" => Self::EndForEach,
            "for" => Self::For,
            "endfor" => Self::EndFor,
            "while" => Self::While,
            "endwhile" => Self::EndWhile,
            "section" => Self::Section,
            "endsection" => Self::EndSection,
            "yield" => Self::Yield,
            "extends" => Self::Extends,
            "include" => Self::Include,
            "auth" => Self::Auth,
            "endauth" => Self::EndAuth,
            "guest" => Self::Guest,
            "endguest" => Self::EndGuest,
            "csrf" => Self::Csrf,
            "method" => Self::Method,
            "json" => Self::Json,
            "dump" => Self::Dump,
            "error" => Self::Error,
            "slot" => Self::Slot,
            "endslot" => Self::EndSlot,
            "props" => Self::Props,
            _ => Self::Custom(s.to_string()),
        }
    }

    /// Check if directive is a closing tag
    pub fn is_closing(&self) -> bool {
        matches!(
            self,
            Self::EndIf
                | Self::EndForEach
                | Self::EndFor
                | Self::EndWhile
                | Self::EndSection
                | Self::EndAuth
                | Self::EndGuest
                | Self::EndSlot
        )
    }

    /// Check if directive needs a closing tag (or is a mid-block directive)
    pub fn needs_closing(&self) -> bool {
        matches!(
            self,
            Self::If | Self::ForEach | Self::For | Self::While | Self::Section | Self::Auth | Self::Guest | Self::ElseIf | Self::Slot
        )
    }

    /// Get matching closing directive
    pub fn closing_directive(&self) -> Option<Self> {
        match self {
            Self::If => Some(Self::EndIf),
            Self::ForEach => Some(Self::EndForEach),
            Self::For => Some(Self::EndFor),
            Self::While => Some(Self::EndWhile),
            Self::Section => Some(Self::EndSection),
            Self::Auth => Some(Self::EndAuth),
            Self::Guest => Some(Self::EndGuest),
            Self::Slot => Some(Self::EndSlot),
            _ => None,
        }
    }
}

/// Blade template lexer
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current: Option<char>,
}

impl Lexer {
    /// Create a new lexer
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current = chars.first().copied();

        Self {
            input: chars,
            position: 0,
            current,
        }
    }

    /// Tokenize the input
    pub fn tokenize(input: &str) -> LexerResult<Vec<Token>> {
        let mut lexer = Self::new(input);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token()?;
            if token == Token::Eof {
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Get the next token
    fn next_token(&mut self) -> LexerResult<Token> {
        // Check for component closing tags: </x-name>
        if self.current == Some('<') && self.peek() == Some('/') && self.peek_ahead(2) == Some('x') && self.peek_ahead(3) == Some('-') {
            return self.read_component_closing_tag();
        }

        // Check for component opening tags: <x-name>
        if self.current == Some('<') && self.peek() == Some('x') && self.peek_ahead(2) == Some('-') {
            return self.read_component_tag();
        }

        // Check for directives
        if self.current == Some('@') {
            return self.read_directive();
        }

        // Check for raw output {!! !!}
        if self.current == Some('{') && self.peek() == Some('!') && self.peek_ahead(2) == Some('!') {
            return self.read_interpolation();
        }

        // Check for variable interpolation {{ }}
        if self.current == Some('{') && self.peek() == Some('{') {
            return self.read_interpolation();
        }

        // Read text until we hit a special character
        if self.current.is_some() {
            return self.read_text();
        }

        Ok(Token::Eof)
    }

    /// Read a directive (@if, @foreach, etc.)
    fn read_directive(&mut self) -> LexerResult<Token> {
        self.advance(); // Skip '@'

        // Read directive name
        let mut name = String::new();
        while let Some(ch) = self.current {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if name.is_empty() {
            return Ok(Token::Text("@".to_string()));
        }

        let directive_type = DirectiveType::from_str(&name);

        // Check if it's a closing directive
        if directive_type.is_closing() {
            return Ok(Token::DirectiveEnd(directive_type));
        }

        // Check for @else (no arguments)
        if matches!(directive_type, DirectiveType::Else) {
            return Ok(Token::DirectiveStart(directive_type, String::new()));
        }

        // Read arguments if present
        let args = if self.current == Some('(') {
            self.read_directive_args()?
        } else {
            String::new()
        };

        // Determine if this is a start directive or standalone
        if directive_type.needs_closing() {
            Ok(Token::DirectiveStart(directive_type, args))
        } else {
            Ok(Token::Directive(directive_type, args))
        }
    }

    /// Read directive arguments: (args)
    fn read_directive_args(&mut self) -> LexerResult<String> {
        self.advance(); // Skip '('

        let mut args = String::new();
        let mut depth = 1;
        let mut in_string = false;
        let mut string_char = ' ';

        while let Some(ch) = self.current {
            if ch == '"' || ch == '\'' {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
                args.push(ch);
                self.advance();
            } else if !in_string && ch == '(' {
                depth += 1;
                args.push(ch);
                self.advance();
            } else if !in_string && ch == ')' {
                depth -= 1;
                if depth == 0 {
                    self.advance(); // Skip closing ')'
                    break;
                }
                args.push(ch);
                self.advance();
            } else {
                args.push(ch);
                self.advance();
            }
        }

        Ok(args.trim().to_string())
    }

    /// Read variable interpolation: {{ $var }} or {!! $var !!}
    fn read_interpolation(&mut self) -> LexerResult<Token> {
        // First check if this is {!! (raw output)
        if self.current == Some('{')
            && self.peek() == Some('!')
            && self.peek_ahead(2) == Some('!')
        {
            self.advance(); // Skip '{'
            self.advance(); // Skip first '!'
            self.advance(); // Skip second '!'
            return self.read_raw_output();
        }

        // Otherwise it's {{ (regular variable)
        self.advance(); // Skip first '{'
        self.advance(); // Skip second '{'

        // Check for comment {{-- --}}
        let is_comment = self.current == Some('-') && self.peek() == Some('-');
        if is_comment {
            self.advance(); // Skip first '-'
            self.advance(); // Skip second '-'

            let mut comment = String::new();
            loop {
                if self.current == Some('-')
                    && self.peek() == Some('-')
                    && self.peek_ahead(2) == Some('}')
                    && self.peek_ahead(3) == Some('}')
                {
                    self.advance(); // Skip first '-'
                    self.advance(); // Skip second '-'
                    self.advance(); // Skip first '}'
                    self.advance(); // Skip second '}'
                    break;
                }

                if let Some(ch) = self.current {
                    comment.push(ch);
                    self.advance();
                } else {
                    return Err(LexerError::UnterminatedString(self.position));
                }
            }

            return Ok(Token::Comment(comment.trim().to_string()));
        }

        // Read variable content
        let mut content = String::new();
        loop {
            if self.current == Some('}') && self.peek() == Some('}') {
                self.advance(); // Skip first '}'
                self.advance(); // Skip second '}'
                break;
            }

            if let Some(ch) = self.current {
                content.push(ch);
                self.advance();
            } else {
                return Err(LexerError::UnterminatedString(self.position));
            }
        }

        Ok(Token::Variable(content.trim().to_string()))
    }

    /// Read raw output: {!! $var !!}
    fn read_raw_output(&mut self) -> LexerResult<Token> {
        let mut content = String::new();

        loop {
            if self.current == Some('!')
                && self.peek() == Some('!')
                && self.peek_ahead(2) == Some('}')
            {
                self.advance(); // Skip first '!'
                self.advance(); // Skip second '!'
                self.advance(); // Skip '}'
                break;
            }

            if let Some(ch) = self.current {
                content.push(ch);
                self.advance();
            } else {
                return Err(LexerError::UnterminatedString(self.position));
            }
        }

        Ok(Token::RawVariable(content.trim().to_string()))
    }

    /// Read plain text
    fn read_text(&mut self) -> LexerResult<Token> {
        let mut text = String::new();

        while let Some(ch) = self.current {
            // Stop at special characters
            if ch == '@' {
                break;
            }

            // Stop at {{ or {!!
            if ch == '{' {
                if self.peek() == Some('{') || (self.peek() == Some('!') && self.peek_ahead(2) == Some('!')) {
                    break;
                }
            }

            // Stop at component tags <x- or </x-
            if ch == '<' {
                if (self.peek() == Some('x') && self.peek_ahead(2) == Some('-'))
                    || (self.peek() == Some('/') && self.peek_ahead(2) == Some('x') && self.peek_ahead(3) == Some('-'))
                {
                    break;
                }
            }

            text.push(ch);
            self.advance();
        }

        Ok(Token::Text(text))
    }

    /// Read component opening tag: <x-alert type="danger">
    fn read_component_tag(&mut self) -> LexerResult<Token> {
        self.advance(); // Skip '<'
        self.advance(); // Skip 'x'
        self.advance(); // Skip '-'

        // Read component name
        let mut name = String::new();
        while let Some(ch) = self.current {
            if ch.is_alphanumeric() || ch == '-' || ch == '.' || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Skip whitespace
        while self.current == Some(' ') || self.current == Some('\t') || self.current == Some('\n') {
            self.advance();
        }

        // Read attributes
        let mut attributes = Vec::new();
        let mut self_closing = false;

        loop {
            // Check for self-closing />
            if self.current == Some('/') && self.peek() == Some('>') {
                self_closing = true;
                self.advance(); // Skip '/'
                self.advance(); // Skip '>'
                break;
            }

            // Check for closing >
            if self.current == Some('>') {
                self.advance();
                break;
            }

            // Read attribute name
            let mut attr_name = String::new();
            while let Some(ch) = self.current {
                if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ':' {
                    attr_name.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            if attr_name.is_empty() {
                break;
            }

            // Skip whitespace and '='
            while self.current == Some(' ') || self.current == Some('\t') {
                self.advance();
            }

            if self.current != Some('=') {
                // Attribute without value (boolean attribute)
                attributes.push((attr_name, String::new()));
                continue;
            }

            self.advance(); // Skip '='

            // Skip whitespace
            while self.current == Some(' ') || self.current == Some('\t') {
                self.advance();
            }

            // Read attribute value
            let quote_char = self.current;
            if quote_char != Some('"') && quote_char != Some('\'') {
                // No quotes - read until space or >
                let mut value = String::new();
                while let Some(ch) = self.current {
                    if ch == ' ' || ch == '\t' || ch == '>' || ch == '/' {
                        break;
                    }
                    value.push(ch);
                    self.advance();
                }
                attributes.push((attr_name, value));
            } else {
                self.advance(); // Skip opening quote

                let mut value = String::new();
                while let Some(ch) = self.current {
                    if Some(ch) == quote_char {
                        self.advance(); // Skip closing quote
                        break;
                    }
                    value.push(ch);
                    self.advance();
                }
                attributes.push((attr_name, value));
            }

            // Skip whitespace
            while self.current == Some(' ') || self.current == Some('\t') || self.current == Some('\n') {
                self.advance();
            }
        }

        Ok(Token::ComponentStart {
            name,
            attributes,
            self_closing,
        })
    }

    /// Read component closing tag: </x-alert>
    fn read_component_closing_tag(&mut self) -> LexerResult<Token> {
        self.advance(); // Skip '<'
        self.advance(); // Skip '/'
        self.advance(); // Skip 'x'
        self.advance(); // Skip '-'

        // Read component name
        let mut name = String::new();
        while let Some(ch) = self.current {
            if ch.is_alphanumeric() || ch == '-' || ch == '.' || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Skip to closing >
        while self.current.is_some() && self.current != Some('>') {
            self.advance();
        }

        if self.current == Some('>') {
            self.advance();
        }

        Ok(Token::ComponentEnd(name))
    }

    /// Advance to next character
    fn advance(&mut self) {
        self.position += 1;
        self.current = self.input.get(self.position).copied();
    }

    /// Peek at next character
    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    /// Peek ahead n characters
    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.input.get(self.position + n).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_text() {
        let tokens = Lexer::tokenize("Hello World").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Text("Hello World".to_string()));
    }

    #[test]
    fn test_tokenize_variable() {
        let tokens = Lexer::tokenize("{{ $name }}").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Variable("$name".to_string()));
    }

    #[test]
    fn test_tokenize_raw_variable() {
        let tokens = Lexer::tokenize("{!! $html !!}").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::RawVariable("$html".to_string()));
    }

    #[test]
    fn test_tokenize_comment() {
        let tokens = Lexer::tokenize("{{-- This is a comment --}}").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Comment("This is a comment".to_string()));
    }

    #[test]
    fn test_tokenize_if_directive() {
        let tokens = Lexer::tokenize("@if($show) Content @endif").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[0],
            Token::DirectiveStart(DirectiveType::If, "$show".to_string())
        );
        assert_eq!(tokens[1], Token::Text(" Content ".to_string()));
        assert_eq!(tokens[2], Token::DirectiveEnd(DirectiveType::EndIf));
    }

    #[test]
    fn test_tokenize_foreach_directive() {
        let tokens = Lexer::tokenize("@foreach($items as $item) {{ $item }} @endforeach").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(
            tokens[0],
            Token::DirectiveStart(DirectiveType::ForEach, "$items as $item".to_string())
        );
        assert_eq!(tokens[1], Token::Text(" ".to_string()));
        assert_eq!(tokens[2], Token::Variable("$item".to_string()));
        assert_eq!(tokens[3], Token::Text(" ".to_string()));
        assert_eq!(tokens[4], Token::DirectiveEnd(DirectiveType::EndForEach));
    }

    #[test]
    fn test_tokenize_section() {
        let tokens = Lexer::tokenize("@section('content') Body @endsection").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[0],
            Token::DirectiveStart(DirectiveType::Section, "'content'".to_string())
        );
        assert_eq!(tokens[1], Token::Text(" Body ".to_string()));
        assert_eq!(tokens[2], Token::DirectiveEnd(DirectiveType::EndSection));
    }

    #[test]
    fn test_tokenize_yield() {
        let tokens = Lexer::tokenize("@yield('title')").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Directive(DirectiveType::Yield, "'title'".to_string()));
    }

    #[test]
    fn test_tokenize_extends() {
        let tokens = Lexer::tokenize("@extends('layouts.app')").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0],
            Token::Directive(DirectiveType::Extends, "'layouts.app'".to_string())
        );
    }

    #[test]
    fn test_tokenize_mixed_content() {
        let tokens = Lexer::tokenize("<h1>{{ $title }}</h1>").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Text("<h1>".to_string()));
        assert_eq!(tokens[1], Token::Variable("$title".to_string()));
        assert_eq!(tokens[2], Token::Text("</h1>".to_string()));
    }

    #[test]
    fn test_tokenize_csrf() {
        let tokens = Lexer::tokenize("@csrf").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Directive(DirectiveType::Csrf, String::new()));
    }

    #[test]
    fn test_tokenize_method() {
        let tokens = Lexer::tokenize("@method('PUT')").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Directive(DirectiveType::Method, "'PUT'".to_string()));
    }

    #[test]
    fn test_tokenize_else() {
        let tokens = Lexer::tokenize("@if($a) A @else B @endif").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[2], Token::DirectiveStart(DirectiveType::Else, String::new()));
    }

    #[test]
    fn test_tokenize_elseif() {
        let tokens = Lexer::tokenize("@if($a) A @elseif($b) B @endif").unwrap();
        // tokens: @if($a), " A ", @elseif($b), " B ", @endif
        assert_eq!(tokens.len(), 5);
        assert_eq!(
            tokens[2],
            Token::DirectiveStart(DirectiveType::ElseIf, "$b".to_string())
        );
    }
}
