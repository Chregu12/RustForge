//! Blade template parser - builds AST from tokens

use crate::ast::{AstNode, Expr};
use crate::lexer::{DirectiveType, Lexer, Token};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(String),

    #[error("Expected {expected}, got {got}")]
    ExpectedToken { expected: String, got: String },

    #[error("Unclosed directive: {0:?}")]
    UnclosedDirective(String),

    #[error("Lexer error: {0}")]
    LexerError(String),

    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    #[error("Invalid directive arguments: {0}")]
    InvalidArguments(String),
}

pub type ParseResult<T> = Result<T, ParseError>;

/// Parser for Blade templates
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse template from string
    pub fn parse(input: &str) -> ParseResult<Vec<AstNode>> {
        // Tokenize
        let tokens = Lexer::tokenize(input)
            .map_err(|e| ParseError::LexerError(e.to_string()))?;

        // Parse tokens
        let mut parser = Self::new(tokens);
        parser.parse_nodes()
    }

    /// Parse a sequence of nodes
    fn parse_nodes(&mut self) -> ParseResult<Vec<AstNode>> {
        let mut nodes = Vec::new();

        while !self.is_at_end() {
            // Check if we're at a closing directive
            if let Some(Token::DirectiveEnd(_)) = self.current() {
                break;
            }

            // Check if we're at @else or @elseif
            if let Some(Token::DirectiveStart(DirectiveType::Else, _))
            | Some(Token::DirectiveStart(DirectiveType::ElseIf, _)) = self.current()
            {
                break;
            }

            let node = self.parse_node()?;
            nodes.push(node);

            // Debug: check position
            if cfg!(test) && nodes.len() > 10 {
                eprintln!("DEBUG: Parsed {} nodes, position: {}/{}", nodes.len(), self.position, self.tokens.len());
            }
        }

        Ok(nodes)
    }

    /// Parse a single node
    fn parse_node(&mut self) -> ParseResult<AstNode> {
        match self.current() {
            Some(Token::Text(text)) => {
                let text = text.clone();
                self.advance();
                Ok(AstNode::Text(text))
            }

            Some(Token::Variable(var)) => {
                let var = var.clone();
                self.advance();
                Ok(AstNode::Variable(var))
            }

            Some(Token::RawVariable(var)) => {
                let var = var.clone();
                self.advance();
                Ok(AstNode::RawVariable(var))
            }

            Some(Token::Comment(_)) => {
                // Skip comments
                self.advance();
                self.parse_node()
            }

            Some(Token::DirectiveStart(directive_type, args)) => {
                let directive_type = directive_type.clone();
                let args = args.clone();
                self.advance();
                self.parse_directive_start(directive_type, args)
            }

            Some(Token::Directive(directive_type, args)) => {
                let directive_type = directive_type.clone();
                let args = args.clone();
                self.advance();
                self.parse_standalone_directive(directive_type, args)
            }

            Some(Token::ComponentStart { name, attributes, self_closing }) => {
                let name = name.clone();
                let attributes = attributes.clone();
                let self_closing = *self_closing;
                self.advance();
                self.parse_component(name, attributes, self_closing)
            }

            Some(Token::Eof) => Err(ParseError::UnexpectedToken("EOF".to_string())),

            Some(token) => Err(ParseError::UnexpectedToken(format!("{:?}", token))),

            None => Err(ParseError::UnexpectedToken("None".to_string())),
        }
    }

    /// Parse a directive that opens a block (@if, @foreach, etc.)
    fn parse_directive_start(
        &mut self,
        directive_type: DirectiveType,
        args: String,
    ) -> ParseResult<AstNode> {
        match directive_type {
            DirectiveType::If => self.parse_if(args),
            DirectiveType::ForEach => self.parse_foreach(args),
            DirectiveType::For => self.parse_for(args),
            DirectiveType::While => self.parse_while(args),
            DirectiveType::Section => self.parse_section(args),
            DirectiveType::Auth => self.parse_auth(),
            DirectiveType::Guest => self.parse_guest(),
            DirectiveType::Slot => self.parse_slot(args),
            DirectiveType::Else => {
                // Else is handled in parse_if
                Err(ParseError::UnexpectedToken("@else".to_string()))
            }
            _ => Err(ParseError::UnexpectedToken(format!("{:?}", directive_type))),
        }
    }

    /// Parse standalone directives (@yield, @extends, @csrf, etc.)
    fn parse_standalone_directive(
        &mut self,
        directive_type: DirectiveType,
        args: String,
    ) -> ParseResult<AstNode> {
        match directive_type {
            DirectiveType::Yield => self.parse_yield(args),
            DirectiveType::Extends => self.parse_extends(args),
            DirectiveType::Include => self.parse_include(args),
            DirectiveType::Csrf => Ok(AstNode::Csrf),
            DirectiveType::Method => self.parse_method(args),
            DirectiveType::Json => self.parse_json(args),
            DirectiveType::Dump => self.parse_dump(args),
            DirectiveType::Error => self.parse_error(args),
            DirectiveType::Custom(name) => Ok(AstNode::Custom { name, args }),
            _ => Err(ParseError::UnexpectedToken(format!("{:?}", directive_type))),
        }
    }

    /// Parse @if directive with optional @elseif and @else
    fn parse_if(&mut self, condition_str: String) -> ParseResult<AstNode> {
        let condition = Expr::parse(&condition_str);
        let then_branch = self.parse_nodes()?;

        let mut else_if_branches = Vec::new();
        let mut else_branch = None;

        // Check for @elseif and @else
        loop {
            match self.current() {
                Some(Token::DirectiveStart(DirectiveType::ElseIf, args)) => {
                    let args = args.clone();
                    self.advance();
                    let condition = Expr::parse(&args);
                    let body = self.parse_nodes()?;
                    else_if_branches.push((condition, body));
                }

                Some(Token::DirectiveStart(DirectiveType::Else, _)) => {
                    self.advance();
                    else_branch = Some(self.parse_nodes()?);
                    break;
                }

                Some(Token::DirectiveEnd(DirectiveType::EndIf)) => {
                    self.advance();
                    break;
                }

                _ => {
                    return Err(ParseError::UnclosedDirective("@if".to_string()));
                }
            }
        }

        Ok(AstNode::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        })
    }

    /// Parse @foreach directive
    fn parse_foreach(&mut self, args: String) -> ParseResult<AstNode> {
        // Parse: $items as $item or $items as $key => $value
        let parts: Vec<&str> = args.split(" as ").collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidArguments(args));
        }

        let collection = Expr::parse(parts[0].trim());
        let item_part = parts[1].trim();

        let (key_var, item_var) = if item_part.contains("=>") {
            let kv: Vec<&str> = item_part.split("=>").collect();
            if kv.len() != 2 {
                return Err(ParseError::InvalidArguments(args));
            }
            let key = kv[0].trim().strip_prefix('$').unwrap_or(kv[0].trim()).to_string();
            let item = kv[1].trim().strip_prefix('$').unwrap_or(kv[1].trim()).to_string();
            (Some(key), item)
        } else {
            let item = item_part.strip_prefix('$').unwrap_or(item_part).to_string();
            (None, item)
        };

        let body = self.parse_nodes()?;

        // Expect @endforeach
        if !matches!(self.current(), Some(Token::DirectiveEnd(DirectiveType::EndForEach))) {
            return Err(ParseError::UnclosedDirective("@foreach".to_string()));
        }
        self.advance();

        Ok(AstNode::ForEach {
            collection,
            item_var,
            key_var,
            body,
        })
    }

    /// Parse @for directive
    fn parse_for(&mut self, args: String) -> ParseResult<AstNode> {
        // Parse: $i = 0; $i < 10; $i++
        // For simplicity, store as raw strings for now
        let parts: Vec<&str> = args.split(';').map(|s| s.trim()).collect();
        if parts.len() != 3 {
            return Err(ParseError::InvalidArguments(args));
        }

        let init = parts[0].to_string();
        let condition = parts[1].to_string();
        let increment = parts[2].to_string();

        let body = self.parse_nodes()?;

        // Expect @endfor
        if !matches!(self.current(), Some(Token::DirectiveEnd(DirectiveType::EndFor))) {
            return Err(ParseError::UnclosedDirective("@for".to_string()));
        }
        self.advance();

        Ok(AstNode::For {
            init,
            condition,
            increment,
            body,
        })
    }

    /// Parse @while directive
    fn parse_while(&mut self, condition_str: String) -> ParseResult<AstNode> {
        let condition = Expr::parse(&condition_str);
        let body = self.parse_nodes()?;

        // Expect @endwhile
        if !matches!(self.current(), Some(Token::DirectiveEnd(DirectiveType::EndWhile))) {
            return Err(ParseError::UnclosedDirective("@while".to_string()));
        }
        self.advance();

        Ok(AstNode::While { condition, body })
    }

    /// Parse @section directive
    fn parse_section(&mut self, args: String) -> ParseResult<AstNode> {
        // Parse section name from arguments: 'content' or "content"
        let name = args
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        let content = self.parse_nodes()?;

        // Expect @endsection
        if !matches!(
            self.current(),
            Some(Token::DirectiveEnd(DirectiveType::EndSection))
        ) {
            return Err(ParseError::UnclosedDirective("@section".to_string()));
        }
        self.advance();

        Ok(AstNode::Section { name, content })
    }

    /// Parse @auth directive
    fn parse_auth(&mut self) -> ParseResult<AstNode> {
        let content = self.parse_nodes()?;

        // Expect @endauth
        if !matches!(self.current(), Some(Token::DirectiveEnd(DirectiveType::EndAuth))) {
            return Err(ParseError::UnclosedDirective("@auth".to_string()));
        }
        self.advance();

        Ok(AstNode::Auth { content })
    }

    /// Parse @guest directive
    fn parse_guest(&mut self) -> ParseResult<AstNode> {
        let content = self.parse_nodes()?;

        // Expect @endguest
        if !matches!(self.current(), Some(Token::DirectiveEnd(DirectiveType::EndGuest))) {
            return Err(ParseError::UnclosedDirective("@guest".to_string()));
        }
        self.advance();

        Ok(AstNode::Guest { content })
    }

    /// Parse @yield directive
    fn parse_yield(&mut self, args: String) -> ParseResult<AstNode> {
        // Parse: 'content' or 'content', 'default value'
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

        let name = parts[0]
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        let default = if parts.len() > 1 {
            Some(parts[1].trim_matches(|c| c == '\'' || c == '"').to_string())
        } else {
            None
        };

        Ok(AstNode::Yield { name, default })
    }

    /// Parse @extends directive
    fn parse_extends(&mut self, args: String) -> ParseResult<AstNode> {
        let parent = args
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        Ok(AstNode::Extends { parent })
    }

    /// Parse @include directive
    fn parse_include(&mut self, args: String) -> ParseResult<AstNode> {
        // For now, just parse template name
        let template = args
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        Ok(AstNode::Include {
            template,
            data: None,
        })
    }

    /// Parse @method directive
    fn parse_method(&mut self, args: String) -> ParseResult<AstNode> {
        let method = args
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_uppercase();

        Ok(AstNode::Method { method })
    }

    /// Parse @json directive
    fn parse_json(&mut self, args: String) -> ParseResult<AstNode> {
        let variable = args.trim().to_string();
        Ok(AstNode::Json { variable })
    }

    /// Parse @dump directive
    fn parse_dump(&mut self, args: String) -> ParseResult<AstNode> {
        let variable = args.trim().to_string();
        Ok(AstNode::Dump { variable })
    }

    /// Parse @error directive
    fn parse_error(&mut self, args: String) -> ParseResult<AstNode> {
        let field = args
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        Ok(AstNode::Error { field })
    }

    /// Parse component tag
    fn parse_component(
        &mut self,
        name: String,
        attributes: Vec<(String, String)>,
        self_closing: bool,
    ) -> ParseResult<AstNode> {
        use std::collections::HashMap;

        // If self-closing, return component with no children
        if self_closing {
            return Ok(AstNode::Component {
                name,
                attributes,
                slots: HashMap::new(),
                children: vec![],
            });
        }

        // Parse component children until we hit the closing tag
        let mut children = Vec::new();
        let mut slots = HashMap::new();

        loop {
            // Check for closing component tag
            if let Some(Token::ComponentEnd(close_name)) = self.current() {
                if close_name == &name {
                    self.advance();
                    break;
                }
                return Err(ParseError::UnexpectedToken(format!(
                    "Expected </x-{}>, got </x-{}>",
                    name, close_name
                )));
            }

            // Check for @slot directive
            if let Some(Token::DirectiveStart(DirectiveType::Slot, slot_args)) = self.current() {
                let slot_args = slot_args.clone();
                self.advance();

                // Parse slot name
                let slot_name = slot_args
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();

                // Parse slot content
                let slot_content = self.parse_nodes_until_slot_end()?;

                slots.insert(slot_name, slot_content);
                continue;
            }

            // Check if we're at end
            if self.is_at_end() {
                return Err(ParseError::UnclosedDirective(format!("<x-{}>", name)));
            }

            // Parse regular child node
            let node = self.parse_node()?;
            children.push(node);
        }

        Ok(AstNode::Component {
            name,
            attributes,
            slots,
            children,
        })
    }

    /// Parse nodes until @endslot
    fn parse_nodes_until_slot_end(&mut self) -> ParseResult<Vec<AstNode>> {
        let mut nodes = Vec::new();

        while !self.is_at_end() {
            // Check for @endslot
            if matches!(self.current(), Some(Token::DirectiveEnd(DirectiveType::EndSlot))) {
                self.advance();
                break;
            }

            let node = self.parse_node()?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Parse @slot directive (when defining a slot in a component template)
    fn parse_slot(&mut self, args: String) -> ParseResult<AstNode> {
        // Parse slot name
        let name = args
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        // Parse default content
        let default_content = self.parse_nodes()?;

        // Expect @endslot
        if !matches!(
            self.current(),
            Some(Token::DirectiveEnd(DirectiveType::EndSlot))
        ) {
            return Err(ParseError::UnclosedDirective("@slot".to_string()));
        }
        self.advance();

        Ok(AstNode::SlotDefinition {
            name,
            default_content,
        })
    }

    /// Get current token
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    /// Advance to next token
    fn advance(&mut self) {
        self.position += 1;
    }

    /// Check if at end of tokens
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text() {
        let nodes = Parser::parse("Hello World").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0], AstNode::Text("Hello World".to_string()));
    }

    #[test]
    fn test_parse_variable() {
        let nodes = Parser::parse("{{ $name }}").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0], AstNode::Variable("$name".to_string()));
    }

    #[test]
    fn test_parse_if() {
        let nodes = Parser::parse("@if($show) Content @endif").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                assert_eq!(*condition, Expr::Variable("show".to_string()));
                assert_eq!(then_branch.len(), 1);
                assert_eq!(else_if_branches.len(), 0);
                assert!(else_branch.is_none());
            }
            _ => panic!("Expected If node"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let nodes = Parser::parse("@if($show) A @else B @endif").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::If {
                else_branch: Some(else_b),
                ..
            } => {
                assert_eq!(else_b.len(), 1);
            }
            _ => panic!("Expected If node with else"),
        }
    }

    #[test]
    fn test_parse_foreach() {
        let nodes = Parser::parse("@foreach($items as $item) {{ $item }} @endforeach").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::ForEach {
                collection,
                item_var,
                key_var,
                body,
            } => {
                assert_eq!(*collection, Expr::Variable("items".to_string()));
                assert_eq!(item_var, "item");
                assert!(key_var.is_none());
                assert_eq!(body.len(), 3); // space + variable + space
            }
            _ => panic!("Expected ForEach node"),
        }
    }

    #[test]
    fn test_parse_section() {
        let nodes = Parser::parse("@section('content') Body @endsection").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::Section { name, content } => {
                assert_eq!(name, "content");
                assert_eq!(content.len(), 1);
            }
            _ => panic!("Expected Section node"),
        }
    }

    #[test]
    fn test_parse_yield() {
        let nodes = Parser::parse("@yield('title')").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::Yield { name, default } => {
                assert_eq!(name, "title");
                assert!(default.is_none());
            }
            _ => panic!("Expected Yield node"),
        }
    }

    #[test]
    fn test_parse_extends() {
        let nodes = Parser::parse("@extends('layouts.app')").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::Extends { parent } => {
                assert_eq!(parent, "layouts.app");
            }
            _ => panic!("Expected Extends node"),
        }
    }

    #[test]
    fn test_parse_csrf() {
        let nodes = Parser::parse("@csrf").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0], AstNode::Csrf);
    }

    #[test]
    fn test_parse_method() {
        let nodes = Parser::parse("@method('PUT')").unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::Method { method } => {
                assert_eq!(method, "PUT");
            }
            _ => panic!("Expected Method node"),
        }
    }

    #[test]
    fn test_parse_mixed_content() {
        let nodes = Parser::parse("<h1>{{ $title }}</h1>").unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], AstNode::Text(_)));
        assert!(matches!(nodes[1], AstNode::Variable(_)));
        assert!(matches!(nodes[2], AstNode::Text(_)));
    }

    #[test]
    fn test_parse_nested_if() {
        let template = "@if($a) @if($b) Nested @endif @endif";
        let nodes = Parser::parse(template).unwrap();
        assert_eq!(nodes.len(), 1);

        match &nodes[0] {
            AstNode::If { then_branch, .. } => {
                // then_branch contains: " ", nested @if, " "
                assert_eq!(then_branch.len(), 3);
                assert!(matches!(then_branch[1], AstNode::If { .. }));
            }
            _ => panic!("Expected nested If"),
        }
    }
}
