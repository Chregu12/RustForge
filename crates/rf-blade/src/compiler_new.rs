//! Blade template compiler - executes AST to generate HTML

use crate::ast::{AstNode, BinaryOperator, Expr, UnaryOperator};
use crate::components::{AttributeBag, ComponentProps, ComponentRegistry};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Section not found: {0}")]
    SectionNotFound(String),
}

pub type CompileResult<T> = Result<T, CompileError>;

/// Execution context for template rendering
#[derive(Debug, Clone)]
pub struct RenderContext {
    /// Template data/variables
    pub data: Value,

    /// Sections defined in the template
    pub sections: HashMap<String, String>,

    /// Parent template name (from @extends)
    pub parent: Option<String>,

    /// Authentication state (for @auth/@guest)
    pub is_authenticated: bool,

    /// Component registry (shared reference)
    pub component_registry: Option<Arc<ComponentRegistry>>,
}

impl RenderContext {
    /// Create a new render context
    pub fn new(data: Value) -> Self {
        Self {
            data,
            sections: HashMap::new(),
            parent: None,
            is_authenticated: false,
            component_registry: None,
        }
    }

    /// Create a new render context with component registry
    pub fn with_components(data: Value, registry: Arc<ComponentRegistry>) -> Self {
        Self {
            data,
            sections: HashMap::new(),
            parent: None,
            is_authenticated: false,
            component_registry: Some(registry),
        }
    }

    /// Get variable value from data
    pub fn get_var(&self, name: &str) -> Option<&Value> {
        // Remove $ prefix if present
        let name = name.strip_prefix('$').unwrap_or(name);

        // Support dot notation: user.name
        if name.contains('.') {
            let parts: Vec<&str> = name.split('.').collect();
            let mut current = &self.data;

            for part in parts {
                match current {
                    Value::Object(map) => {
                        current = map.get(part)?;
                    }
                    _ => return None,
                }
            }

            Some(current)
        } else {
            match &self.data {
                Value::Object(map) => map.get(name),
                _ => None,
            }
        }
    }

    /// Set authenticated state
    pub fn set_authenticated(&mut self, authenticated: bool) {
        self.is_authenticated = authenticated;
    }

    /// Add a section
    pub fn add_section(&mut self, name: String, content: String) {
        self.sections.insert(name, content);
    }

    /// Set parent template
    pub fn set_parent(&mut self, parent: String) {
        self.parent = Some(parent);
    }
}

/// Blade template compiler/executor
pub struct Compiler {
    /// Enable HTML escaping for variables (default: true)
    pub escape_html: bool,
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self { escape_html: true }
    }

    /// Compile and execute AST nodes to HTML
    pub fn compile(&self, nodes: &[AstNode], context: &mut RenderContext) -> CompileResult<String> {
        let mut output = String::new();

        for node in nodes {
            let html = self.compile_node(node, context)?;
            output.push_str(&html);
        }

        Ok(output)
    }

    /// Compile a single AST node
    fn compile_node(&self, node: &AstNode, context: &mut RenderContext) -> CompileResult<String> {
        match node {
            AstNode::Text(text) => Ok(text.clone()),

            AstNode::Variable(var) => self.compile_variable(var, context, true),

            AstNode::RawVariable(var) => self.compile_variable(var, context, false),

            AstNode::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.compile_if(
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                context,
            ),

            AstNode::ForEach {
                collection,
                item_var,
                key_var,
                body,
            } => self.compile_foreach(collection, item_var, key_var.as_deref(), body, context),

            AstNode::For {
                init,
                condition,
                increment,
                body,
            } => self.compile_for(init, condition, increment, body, context),

            AstNode::While { condition, body } => self.compile_while(condition, body, context),

            AstNode::Section { name, content } => {
                // Compile section content and store it
                let html = self.compile(content, context)?;
                context.add_section(name.clone(), html);
                Ok(String::new()) // Sections don't output directly
            }

            AstNode::Yield { name, default } => {
                // Output section content or default
                Ok(context
                    .sections
                    .get(name)
                    .cloned()
                    .or_else(|| default.clone())
                    .unwrap_or_default())
            }

            AstNode::Extends { parent } => {
                // Mark parent template
                context.set_parent(parent.clone());
                Ok(String::new())
            }

            AstNode::Include { template, data: _ } => {
                // For now, just output a placeholder
                // In full implementation, this would load and render the included template
                Ok(format!("<!-- include: {} -->", template))
            }

            AstNode::Auth { content } => {
                if context.is_authenticated {
                    self.compile(content, context)
                } else {
                    Ok(String::new())
                }
            }

            AstNode::Guest { content } => {
                if !context.is_authenticated {
                    self.compile(content, context)
                } else {
                    Ok(String::new())
                }
            }

            AstNode::Csrf => {
                Ok(r#"<input type="hidden" name="_token" value="__CSRF_TOKEN__">"#.to_string())
            }

            AstNode::Method { method } => Ok(format!(
                r#"<input type="hidden" name="_method" value="{}">"#,
                method
            )),

            AstNode::Json { variable } => {
                if let Some(value) = context.get_var(variable) {
                    Ok(serde_json::to_string(value).unwrap_or_default())
                } else {
                    Ok("null".to_string())
                }
            }

            AstNode::Dump { variable } => {
                if let Some(value) = context.get_var(variable) {
                    Ok(format!(
                        "<pre>{}</pre>",
                        serde_json::to_string_pretty(value).unwrap_or_default()
                    ))
                } else {
                    Ok("<pre>null</pre>".to_string())
                }
            }

            AstNode::Error { field } => {
                // Render validation error messages for the specified field
                Ok(format!(
                    r#"{{{{ if let Some(errors) = $errors.get("{}") }}}}
    <div class="validation-error text-red-600 text-sm mt-1">
        {{{{ for error in errors }}}}
            <div>{{{{ error }}}}</div>
        {{{{ endfor }}}}
    </div>
{{{{ endif }}}}"#,
                    field
                ))
            }

            AstNode::Custom { name, args } => {
                // Custom directives - placeholder
                Ok(format!("<!-- custom: {} {} -->", name, args))
            }

            AstNode::Component {
                name,
                attributes,
                slots,
                children,
            } => self.compile_component(name, attributes, slots, children, context),

            AstNode::SlotDefinition {
                name,
                default_content: _,
            } => {
                // Slot definitions are handled when rendering the component template
                // Output the slot placeholder
                Ok(format!("{{{{ $slots.{} }}}}", name))
            }

            AstNode::Props { name } => {
                // Props access
                if let Some(value) = context.get_var(name) {
                    let text = self.value_to_string(value);
                    if self.escape_html {
                        Ok(html_escape(&text))
                    } else {
                        Ok(text)
                    }
                } else {
                    Ok(String::new())
                }
            }
        }
    }

    /// Compile a component
    fn compile_component(
        &self,
        name: &str,
        attributes: &[(String, String)],
        slots: &HashMap<String, Vec<AstNode>>,
        children: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        // Clone the registry Arc to avoid borrow checker issues
        let registry = context
            .component_registry
            .as_ref()
            .ok_or_else(|| {
                CompileError::EvaluationError("Component registry not available".to_string())
            })?
            .clone();

        // Build props from attributes
        let props = ComponentProps::from_attributes(attributes);

        // Build attribute bag
        let attr_bag = AttributeBag::from_pairs(attributes.to_vec());

        // Compile slots
        let mut compiled_slots = HashMap::new();

        // Compile named slots
        for (slot_name, slot_nodes) in slots {
            let slot_html = self.compile(slot_nodes, context)?;
            compiled_slots.insert(slot_name.clone(), slot_html);
        }

        // Compile default slot from children
        if !children.is_empty() {
            let default_slot_html = self.compile(children, context)?;
            compiled_slots.insert("default".to_string(), default_slot_html);
        }

        // Render component
        registry
            .render_component(name, &props, &attr_bag, &compiled_slots)
            .map_err(|e| CompileError::EvaluationError(format!("Component render error: {}", e)))
    }

    /// Compile variable interpolation
    fn compile_variable(
        &self,
        var: &str,
        context: &RenderContext,
        escape: bool,
    ) -> CompileResult<String> {
        // Remove $ and whitespace
        let var_name = var.trim().strip_prefix('$').unwrap_or(var.trim());

        // Check if it's a member access (contains dot)
        let value = if var_name.contains('.') {
            // Handle member access: user.name -> get "user" then access "name"
            let parts: Vec<&str> = var_name.split('.').collect();
            let mut current = context.get_var(parts[0]);

            // Navigate through the member access chain
            for part in parts.iter().skip(1) {
                if let Some(Value::Object(map)) = current {
                    current = map.get(*part);
                } else {
                    current = None;
                    break;
                }
            }

            current
        } else {
            context.get_var(var_name)
        };

        if let Some(val) = value {
            let text = self.value_to_string(val);
            if escape && self.escape_html {
                Ok(html_escape(&text))
            } else {
                Ok(text)
            }
        } else {
            Ok(String::new()) // Missing variables render as empty string
        }
    }

    /// Compile @if directive
    fn compile_if(
        &self,
        condition: &Expr,
        then_branch: &[AstNode],
        else_if_branches: &[(Expr, Vec<AstNode>)],
        else_branch: &Option<Vec<AstNode>>,
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        // Evaluate main condition
        if self.evaluate_expr(condition, context)? {
            return self.compile(then_branch, context);
        }

        // Try else-if branches
        for (elif_cond, elif_body) in else_if_branches {
            if self.evaluate_expr(elif_cond, context)? {
                return self.compile(elif_body, context);
            }
        }

        // Else branch
        if let Some(else_body) = else_branch {
            self.compile(else_body, context)
        } else {
            Ok(String::new())
        }
    }

    /// Compile @foreach directive
    fn compile_foreach(
        &self,
        collection: &Expr,
        item_var: &str,
        key_var: Option<&str>,
        body: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        let collection_value = self.resolve_expr(collection, context)?;

        let mut output = String::new();

        match collection_value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    // Create new context with loop variables
                    let loop_data = match &context.data {
                        Value::Object(map) => {
                            let mut new_map = map.clone();
                            new_map.insert(item_var.to_string(), item.clone());

                            // Set key variable if specified
                            if let Some(key) = key_var {
                                new_map.insert(
                                    key.to_string(),
                                    Value::Number(serde_json::Number::from(index as i64)),
                                );
                            }

                            Value::Object(new_map)
                        }
                        _ => {
                            // If context.data is not an object, create a new one
                            let mut new_map = serde_json::Map::new();
                            new_map.insert(item_var.to_string(), item.clone());

                            if let Some(key) = key_var {
                                new_map.insert(
                                    key.to_string(),
                                    Value::Number(serde_json::Number::from(index as i64)),
                                );
                            }

                            Value::Object(new_map)
                        }
                    };

                    let mut loop_context = context.clone();
                    loop_context.data = loop_data;

                    let html = self.compile(body, &mut loop_context)?;
                    output.push_str(&html);
                }
            }

            Value::Object(map) => {
                for (key, value) in map.iter() {
                    let loop_data = match &context.data {
                        Value::Object(base_map) => {
                            let mut new_map = base_map.clone();
                            new_map.insert(item_var.to_string(), value.clone());

                            if let Some(key_name) = key_var {
                                new_map.insert(key_name.to_string(), Value::String(key.clone()));
                            }

                            Value::Object(new_map)
                        }
                        _ => {
                            let mut new_map = serde_json::Map::new();
                            new_map.insert(item_var.to_string(), value.clone());

                            if let Some(key_name) = key_var {
                                new_map.insert(key_name.to_string(), Value::String(key.clone()));
                            }

                            Value::Object(new_map)
                        }
                    };

                    let mut loop_context = context.clone();
                    loop_context.data = loop_data;

                    let html = self.compile(body, &mut loop_context)?;
                    output.push_str(&html);
                }
            }

            _ => {
                // Not iterable, skip
            }
        }

        Ok(output)
    }

    /// Compile @for directive
    ///
    /// Executes C-style integer loops of the form
    /// `@for($i = 0; $i < 10; $i++)`. The loop variable is exposed to the
    /// body (and can be referenced with `{{ $i }}`), while any variables
    /// already present in the surrounding context remain visible. The
    /// following forms are supported for each clause:
    ///
    /// * init: `$i = <int|var>`
    /// * condition: `$i <op> <int|var>` where op is `<`, `<=`, `>`, `>=`,
    ///   `==` or `!=`
    /// * increment: `$i++`, `$i--`, `$i += <n>`, `$i -= <n>`,
    ///   `$i = $i + <n>`, `$i = $i - <n>`
    fn compile_for(
        &self,
        init: &str,
        condition: &str,
        increment: &str,
        body: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        const MAX_ITERATIONS: usize = 1_000_000;

        let (var_name, start) = Self::parse_for_init(init, context)?;

        let mut output = String::new();
        let mut value = start;
        let mut iterations = 0usize;

        loop {
            // Expose the loop variable to the body/condition via a child context.
            let mut loop_context = context.clone();
            Self::set_int_var(&mut loop_context, &var_name, value);

            if !Self::eval_for_condition(condition, &loop_context)? {
                break;
            }

            if iterations >= MAX_ITERATIONS {
                return Err(CompileError::EvaluationError(
                    "Infinite loop detected in @for".to_string(),
                ));
            }

            let html = self.compile(body, &mut loop_context)?;
            output.push_str(&html);
            iterations += 1;

            value = Self::apply_for_increment(increment, value, &loop_context)?;
        }

        Ok(output)
    }

    /// Parse the init clause of an @for loop into (variable name, start value).
    fn parse_for_init(init: &str, context: &RenderContext) -> CompileResult<(String, i64)> {
        let (lhs, rhs) = init.split_once('=').ok_or_else(|| {
            CompileError::EvaluationError(format!("Invalid @for init clause: {}", init))
        })?;
        let var = lhs.trim().trim_start_matches('$').to_string();
        if var.is_empty() {
            return Err(CompileError::EvaluationError(format!(
                "Invalid @for init variable: {}",
                init
            )));
        }
        let start = Self::eval_int_operand(rhs, context).ok_or_else(|| {
            CompileError::EvaluationError(format!("Invalid @for init value: {}", rhs))
        })?;
        Ok((var, start))
    }

    /// Evaluate an integer operand: either a literal or a variable reference.
    fn eval_int_operand(token: &str, context: &RenderContext) -> Option<i64> {
        let token = token.trim();
        if let Ok(n) = token.parse::<i64>() {
            return Some(n);
        }
        match context.get_var(token) {
            Some(Value::Number(n)) => n.as_i64(),
            _ => None,
        }
    }

    /// Evaluate the condition clause of an @for loop.
    fn eval_for_condition(condition: &str, context: &RenderContext) -> CompileResult<bool> {
        let cond = condition.trim();
        for op in ["<=", ">=", "==", "!=", "<", ">"] {
            if let Some((lhs, rhs)) = cond.split_once(op) {
                let l = Self::eval_int_operand(lhs, context).ok_or_else(|| {
                    CompileError::EvaluationError(format!(
                        "Invalid @for condition operand: {}",
                        lhs
                    ))
                })?;
                let r = Self::eval_int_operand(rhs, context).ok_or_else(|| {
                    CompileError::EvaluationError(format!(
                        "Invalid @for condition operand: {}",
                        rhs
                    ))
                })?;
                let result = match op {
                    "<=" => l <= r,
                    ">=" => l >= r,
                    "==" => l == r,
                    "!=" => l != r,
                    "<" => l < r,
                    ">" => l > r,
                    _ => unreachable!(),
                };
                return Ok(result);
            }
        }
        Err(CompileError::EvaluationError(format!(
            "Unsupported @for condition: {}",
            condition
        )))
    }

    /// Apply the increment clause of an @for loop to the current value.
    ///
    /// `context` must already expose the loop variable at its current value so
    /// that assignment forms like `$i = $i + 1` resolve correctly.
    fn apply_for_increment(
        increment: &str,
        current: i64,
        context: &RenderContext,
    ) -> CompileResult<i64> {
        let inc = increment.trim();

        if inc.ends_with("++") || inc.starts_with("++") {
            return Ok(current + 1);
        }
        if inc.ends_with("--") || inc.starts_with("--") {
            return Ok(current - 1);
        }
        if let Some((_, rhs)) = inc.split_once("+=") {
            let step = Self::eval_int_operand(rhs, context).ok_or_else(|| {
                CompileError::EvaluationError(format!("Invalid @for increment step: {}", rhs))
            })?;
            return Ok(current + step);
        }
        if let Some((_, rhs)) = inc.split_once("-=") {
            let step = Self::eval_int_operand(rhs, context).ok_or_else(|| {
                CompileError::EvaluationError(format!("Invalid @for increment step: {}", rhs))
            })?;
            return Ok(current - step);
        }
        if let Some((_, rhs)) = inc.split_once('=') {
            let rhs = rhs.trim();
            if let Some((a, b)) = rhs.split_once('+') {
                let av = Self::eval_int_operand(a, context).ok_or_else(|| {
                    CompileError::EvaluationError(format!("Invalid @for increment operand: {}", a))
                })?;
                let bv = Self::eval_int_operand(b, context).ok_or_else(|| {
                    CompileError::EvaluationError(format!("Invalid @for increment operand: {}", b))
                })?;
                return Ok(av + bv);
            }
            if let Some((a, b)) = rhs.split_once('-') {
                let av = Self::eval_int_operand(a, context).ok_or_else(|| {
                    CompileError::EvaluationError(format!("Invalid @for increment operand: {}", a))
                })?;
                let bv = Self::eval_int_operand(b, context).ok_or_else(|| {
                    CompileError::EvaluationError(format!("Invalid @for increment operand: {}", b))
                })?;
                return Ok(av - bv);
            }
            if let Some(v) = Self::eval_int_operand(rhs, context) {
                return Ok(v);
            }
        }

        Err(CompileError::EvaluationError(format!(
            "Unsupported @for increment: {}",
            increment
        )))
    }

    /// Set an integer variable on a render context's data object.
    fn set_int_var(context: &mut RenderContext, name: &str, value: i64) {
        let entry = Value::Number(serde_json::Number::from(value));
        match &mut context.data {
            Value::Object(map) => {
                map.insert(name.to_string(), entry);
            }
            other => {
                let mut map = serde_json::Map::new();
                map.insert(name.to_string(), entry);
                *other = Value::Object(map);
            }
        }
    }

    /// Compile @while directive
    fn compile_while(
        &self,
        condition: &Expr,
        body: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        let mut output = String::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000; // Prevent infinite loops

        while self.evaluate_expr(condition, context)? {
            if iterations >= MAX_ITERATIONS {
                return Err(CompileError::EvaluationError(
                    "Infinite loop detected".to_string(),
                ));
            }

            let html = self.compile(body, context)?;
            output.push_str(&html);
            iterations += 1;
        }

        Ok(output)
    }

    /// Evaluate expression to boolean
    fn evaluate_expr(&self, expr: &Expr, context: &RenderContext) -> CompileResult<bool> {
        match expr {
            Expr::Bool(b) => Ok(*b),

            Expr::Variable(var) => {
                if let Some(value) = context.get_var(var) {
                    Ok(self.is_truthy(value))
                } else {
                    Ok(false)
                }
            }

            Expr::BinaryOp { left, op, right } => {
                let left_val = self.resolve_expr(left, context)?;
                let right_val = self.resolve_expr(right, context)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }

            Expr::UnaryOp { op, expr } => {
                let val = self.resolve_expr(expr, context)?;
                match op {
                    UnaryOperator::Not => Ok(!self.is_truthy(&val)),
                    UnaryOperator::Negate => Ok(false), // Doesn't make sense for boolean context
                }
            }

            Expr::Raw(s) => {
                // For raw expressions, try to get as variable
                if let Some(value) = context.get_var(s) {
                    Ok(self.is_truthy(value))
                } else {
                    Ok(false)
                }
            }

            _ => {
                let value = self.resolve_expr(expr, context)?;
                Ok(self.is_truthy(&value))
            }
        }
    }

    /// Resolve expression to a value
    fn resolve_expr(&self, expr: &Expr, context: &RenderContext) -> CompileResult<Value> {
        match expr {
            Expr::Variable(var) => {
                if let Some(value) = context.get_var(var) {
                    Ok(value.clone())
                } else {
                    Ok(Value::Null)
                }
            }

            Expr::String(s) => Ok(Value::String(s.clone())),

            Expr::Number(n) => Ok(serde_json::Number::from_f64(*n)
                .map(Value::Number)
                .unwrap_or(Value::Null)),

            Expr::Bool(b) => Ok(Value::Bool(*b)),

            Expr::Null => Ok(Value::Null),

            Expr::MemberAccess { object, member } => {
                let obj_value = self.resolve_expr(object, context)?;
                match obj_value {
                    Value::Object(map) => Ok(map.get(member).cloned().unwrap_or(Value::Null)),
                    _ => Ok(Value::Null),
                }
            }

            Expr::Raw(s) => {
                // Try as variable
                if let Some(value) = context.get_var(s) {
                    Ok(value.clone())
                } else {
                    Ok(Value::Null)
                }
            }

            _ => Ok(Value::Null),
        }
    }

    /// Evaluate binary operation
    fn evaluate_binary_op(
        &self,
        left: &Value,
        op: &BinaryOperator,
        right: &Value,
    ) -> CompileResult<bool> {
        match op {
            BinaryOperator::Equal => Ok(left == right),
            BinaryOperator::NotEqual => Ok(left != right),

            BinaryOperator::LessThan => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(l < r)
                } else {
                    Ok(false)
                }
            }

            BinaryOperator::LessOrEqual => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(l <= r)
                } else {
                    Ok(false)
                }
            }

            BinaryOperator::GreaterThan => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(l > r)
                } else {
                    Ok(false)
                }
            }

            BinaryOperator::GreaterOrEqual => {
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(l >= r)
                } else {
                    Ok(false)
                }
            }

            BinaryOperator::And => Ok(self.is_truthy(left) && self.is_truthy(right)),

            BinaryOperator::Or => Ok(self.is_truthy(left) || self.is_truthy(right)),

            _ => Ok(false),
        }
    }

    /// Check if value is truthy
    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
        }
    }

    /// Convert JSON value to string
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// HTML escape for safe output
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_new::Parser;
    use serde_json::json;

    #[test]
    fn test_compile_text() {
        let nodes = Parser::parse("Hello World").unwrap();
        let mut context = RenderContext::new(json!({}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "Hello World");
    }

    #[test]
    fn test_compile_variable() {
        let nodes = Parser::parse("{{ $name }}").unwrap();
        let mut context = RenderContext::new(json!({"name": "Alice"}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "Alice");
    }

    #[test]
    fn test_compile_variable_escape() {
        let nodes = Parser::parse("{{ $code }}").unwrap();
        let mut context = RenderContext::new(json!({"code": "<script>alert('xss')</script>"}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
    }

    #[test]
    fn test_compile_raw_variable() {
        let nodes = Parser::parse("{!! $html !!}").unwrap();
        let mut context = RenderContext::new(json!({"html": "<b>Bold</b>"}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "<b>Bold</b>");
    }

    #[test]
    fn test_compile_if_true() {
        let nodes = Parser::parse("@if($show) Visible @endif").unwrap();
        let mut context = RenderContext::new(json!({"show": true}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, " Visible ");
    }

    #[test]
    fn test_compile_if_false() {
        let nodes = Parser::parse("@if($show) Visible @endif").unwrap();
        let mut context = RenderContext::new(json!({"show": false}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "");
    }

    #[test]
    fn test_compile_if_else() {
        let nodes = Parser::parse("@if($show) Yes @else No @endif").unwrap();
        let mut context = RenderContext::new(json!({"show": false}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, " No ");
    }

    #[test]
    fn test_compile_foreach() {
        let nodes = Parser::parse("@foreach($items as $item){{ $item }}@endforeach").unwrap();
        let mut context = RenderContext::new(json!({"items": ["a", "b", "c"]}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "abc");
    }

    #[test]
    fn test_compile_for() {
        let compiler = Compiler::new();

        // Ascending loop with post-increment.
        let nodes = Parser::parse("@for($i = 0; $i < 3; $i++){{ $i }}@endfor").unwrap();
        let mut context = RenderContext::new(json!({}));
        assert_eq!(compiler.compile(&nodes, &mut context).unwrap(), "012");

        // Stepped loop with += and an inclusive bound.
        let nodes = Parser::parse("@for($n = 1; $n <= 10; $n += 3)[{{ $n }}]@endfor").unwrap();
        let mut context = RenderContext::new(json!({}));
        assert_eq!(
            compiler.compile(&nodes, &mut context).unwrap(),
            "[1][4][7][10]"
        );

        // Descending loop whose start comes from context data.
        let nodes = Parser::parse("@for($i = $start; $i > 0; $i--){{ $i }}@endfor").unwrap();
        let mut context = RenderContext::new(json!({ "start": 3 }));
        assert_eq!(compiler.compile(&nodes, &mut context).unwrap(), "321");

        // Assignment-style increment plus access to an outer variable.
        let nodes =
            Parser::parse("@for($i = 0; $i < 2; $i = $i + 1){{ $label }}{{ $i }} @endfor").unwrap();
        let mut context = RenderContext::new(json!({ "label": "x" }));
        assert_eq!(compiler.compile(&nodes, &mut context).unwrap(), "x0 x1 ");
    }

    #[test]
    fn test_compile_section_yield() {
        let nodes = Parser::parse("@section('content')Body@endsection@yield('content')").unwrap();
        let mut context = RenderContext::new(json!({}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "Body");
    }

    #[test]
    fn test_compile_csrf() {
        let nodes = Parser::parse("@csrf").unwrap();
        let mut context = RenderContext::new(json!({}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert!(html.contains("_token"));
    }

    #[test]
    fn test_compile_method() {
        let nodes = Parser::parse("@method('PUT')").unwrap();
        let mut context = RenderContext::new(json!({}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert!(html.contains("PUT"));
    }

    #[test]
    fn test_compile_auth() {
        let nodes = Parser::parse("@auth Logged in @endauth").unwrap();
        let mut context = RenderContext::new(json!({}));
        context.set_authenticated(true);
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, " Logged in ");
    }

    #[test]
    fn test_compile_guest() {
        let nodes = Parser::parse("@guest Not logged in @endguest").unwrap();
        let mut context = RenderContext::new(json!({}));
        context.set_authenticated(false);
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, " Not logged in ");
    }

    #[test]
    fn test_compile_member_access() {
        let nodes = Parser::parse("{{ $user.name }}").unwrap();
        let mut context = RenderContext::new(json!({"user": {"name": "Bob"}}));
        let compiler = Compiler::new();

        let html = compiler.compile(&nodes, &mut context).unwrap();
        assert_eq!(html, "Bob");
    }
}
