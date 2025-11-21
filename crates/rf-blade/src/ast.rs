//! Abstract Syntax Tree for Blade templates

use std::collections::HashMap;

/// Represents a slot in a component
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub name: String,
    pub content: Vec<AstNode>,
}

/// AST Node representing different template elements
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    /// Plain text content
    Text(String),

    /// Variable interpolation (escaped): {{ $var }}
    Variable(String),

    /// Raw variable output (unescaped): {!! $var !!}
    RawVariable(String),

    /// Conditional rendering
    If {
        condition: Expr,
        then_branch: Vec<AstNode>,
        else_if_branches: Vec<(Expr, Vec<AstNode>)>,
        else_branch: Option<Vec<AstNode>>,
    },

    /// For-each loop
    ForEach {
        collection: Expr,
        item_var: String,
        key_var: Option<String>,
        body: Vec<AstNode>,
    },

    /// For loop
    For {
        init: String,
        condition: String,
        increment: String,
        body: Vec<AstNode>,
    },

    /// While loop
    While {
        condition: Expr,
        body: Vec<AstNode>,
    },

    /// Section definition
    Section {
        name: String,
        content: Vec<AstNode>,
    },

    /// Yield placeholder
    Yield {
        name: String,
        default: Option<String>,
    },

    /// Template extension
    Extends {
        parent: String,
    },

    /// Template include
    Include {
        template: String,
        data: Option<HashMap<String, String>>,
    },

    /// Auth check
    Auth {
        content: Vec<AstNode>,
    },

    /// Guest check
    Guest {
        content: Vec<AstNode>,
    },

    /// CSRF token
    Csrf,

    /// HTTP method override
    Method {
        method: String,
    },

    /// JSON output
    Json {
        variable: String,
    },

    /// Dump variable
    Dump {
        variable: String,
    },

    /// Validation error display
    Error {
        field: String,
    },

    /// Custom directive
    Custom {
        name: String,
        args: String,
    },

    /// Component usage
    Component {
        name: String,
        attributes: Vec<(String, String)>,
        slots: HashMap<String, Vec<AstNode>>,
        children: Vec<AstNode>,
    },

    /// Named slot definition (inside component template)
    SlotDefinition {
        name: String,
        default_content: Vec<AstNode>,
    },

    /// Props directive (for component prop access)
    Props {
        name: String,
    },
}

/// Expression node for conditions and values
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Variable reference: $var, $user.name, $arr[0]
    Variable(String),

    /// String literal: "hello", 'world'
    String(String),

    /// Number literal: 42, 3.14
    Number(f64),

    /// Boolean literal: true, false
    Bool(bool),

    /// Null literal
    Null,

    /// Binary operation: $a == $b, $x > 10
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },

    /// Unary operation: !$flag, -$num
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },

    /// Function call: count($items), isset($var)
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },

    /// Member access: $user.name
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },

    /// Array access: $arr[0], $map['key']
    ArrayAccess {
        array: Box<Expr>,
        index: Box<Expr>,
    },

    /// Raw expression (fallback for complex PHP expressions)
    Raw(String),
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    // Comparison
    Equal,        // ==
    NotEqual,     // !=
    LessThan,     // <
    LessOrEqual,  // <=
    GreaterThan,  // >
    GreaterOrEqual, // >=

    // Logical
    And, // &&, and
    Or,  // ||, or

    // Arithmetic
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %

    // String
    Concat, // .
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,    // !, not
    Negate, // -
}

impl Expr {
    /// Create a variable expression
    pub fn var(name: impl Into<String>) -> Self {
        Self::Variable(name.into())
    }

    /// Create a string expression
    pub fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// Create a number expression
    pub fn number(n: f64) -> Self {
        Self::Number(n)
    }

    /// Create a boolean expression
    pub fn bool(b: bool) -> Self {
        Self::Bool(b)
    }

    /// Create a raw expression (for complex PHP expressions)
    pub fn raw(s: impl Into<String>) -> Self {
        Self::Raw(s.into())
    }

    /// Parse a simple expression from string
    /// This is a simplified parser for common cases
    pub fn parse(s: &str) -> Self {
        let s = s.trim();

        // Remove $ prefix if present
        let s = s.strip_prefix('$').unwrap_or(s);

        // Check for string literals
        if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            return Self::String(s[1..s.len() - 1].to_string());
        }

        // Check for boolean literals
        if s == "true" {
            return Self::Bool(true);
        }
        if s == "false" {
            return Self::Bool(false);
        }

        // Check for null
        if s == "null" {
            return Self::Null;
        }

        // Check for numbers
        if let Ok(n) = s.parse::<f64>() {
            return Self::Number(n);
        }

        // Check for member access (dot notation)
        if s.contains('.') && !s.contains(' ') {
            let parts: Vec<&str> = s.split('.').collect();
            let mut expr = Self::var(parts[0]);
            for part in parts.iter().skip(1) {
                expr = Self::MemberAccess {
                    object: Box::new(expr),
                    member: part.to_string(),
                };
            }
            return expr;
        }

        // Default to variable or raw expression
        if s.contains(' ') || s.contains('(') || s.contains('[') {
            // Complex expression - store as raw
            Self::Raw(s.to_string())
        } else {
            // Simple variable
            Self::var(s)
        }
    }
}

impl AstNode {
    /// Check if node is a text node
    pub fn is_text(&self) -> bool {
        matches!(self, AstNode::Text(_))
    }

    /// Check if node is a variable
    pub fn is_variable(&self) -> bool {
        matches!(self, AstNode::Variable(_) | AstNode::RawVariable(_))
    }

    /// Check if node is a control structure
    pub fn is_control_structure(&self) -> bool {
        matches!(
            self,
            AstNode::If { .. }
                | AstNode::ForEach { .. }
                | AstNode::For { .. }
                | AstNode::While { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_parse_variable() {
        let expr = Expr::parse("$name");
        assert_eq!(expr, Expr::Variable("name".to_string()));

        let expr = Expr::parse("name");
        assert_eq!(expr, Expr::Variable("name".to_string()));
    }

    #[test]
    fn test_expr_parse_string() {
        let expr = Expr::parse("\"hello\"");
        assert_eq!(expr, Expr::String("hello".to_string()));

        let expr = Expr::parse("'world'");
        assert_eq!(expr, Expr::String("world".to_string()));
    }

    #[test]
    fn test_expr_parse_bool() {
        let expr = Expr::parse("true");
        assert_eq!(expr, Expr::Bool(true));

        let expr = Expr::parse("false");
        assert_eq!(expr, Expr::Bool(false));
    }

    #[test]
    fn test_expr_parse_number() {
        let expr = Expr::parse("42");
        assert_eq!(expr, Expr::Number(42.0));

        let expr = Expr::parse("3.14");
        assert_eq!(expr, Expr::Number(3.14));
    }

    #[test]
    fn test_expr_parse_member_access() {
        let expr = Expr::parse("$user.name");
        match expr {
            Expr::MemberAccess { object, member } => {
                assert_eq!(*object, Expr::Variable("user".to_string()));
                assert_eq!(member, "name");
            }
            _ => panic!("Expected member access"),
        }
    }

    #[test]
    fn test_expr_parse_complex() {
        let expr = Expr::parse("$a > 10");
        assert_eq!(expr, Expr::Raw("a > 10".to_string()));
    }

    #[test]
    fn test_ast_node_is_text() {
        let node = AstNode::Text("Hello".to_string());
        assert!(node.is_text());

        let node = AstNode::Variable("name".to_string());
        assert!(!node.is_text());
    }

    #[test]
    fn test_ast_node_is_variable() {
        let node = AstNode::Variable("name".to_string());
        assert!(node.is_variable());

        let node = AstNode::RawVariable("html".to_string());
        assert!(node.is_variable());

        let node = AstNode::Text("Hello".to_string());
        assert!(!node.is_variable());
    }

    #[test]
    fn test_ast_node_is_control_structure() {
        let node = AstNode::If {
            condition: Expr::Bool(true),
            then_branch: vec![],
            else_if_branches: vec![],
            else_branch: None,
        };
        assert!(node.is_control_structure());

        let node = AstNode::Text("Hello".to_string());
        assert!(!node.is_control_structure());
    }
}
