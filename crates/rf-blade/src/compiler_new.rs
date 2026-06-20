//! Blade template compiler - executes AST to generate HTML

use crate::ast::{AstNode, BinaryOperator, ConditionalEntry, Expr, UnaryOperator};
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

/// Loop-control signal raised by `@break` / `@continue` while compiling a loop
/// body. Propagated up through `compile` so the enclosing loop can react.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopSignal {
    Break,
    Continue,
}

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

    /// Pending loop-control signal (set by `@break` / `@continue`, consumed by
    /// the enclosing loop). Not part of public template data.
    loop_signal: Option<LoopSignal>,
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
            loop_signal: None,
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
            loop_signal: None,
        }
    }

    /// Get variable value from data
    pub fn get_var(&self, name: &str) -> Option<&Value> {
        // Remove $ prefix if present
        let name = name.strip_prefix('$').unwrap_or(name);

        // Normalize PHP-style arrow access (`loop->index`) to dot notation.
        let normalized;
        let name = if name.contains("->") {
            normalized = name.replace("->", ".");
            normalized.as_str()
        } else {
            name
        };

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

            // If a @break / @continue fired inside this node, stop compiling
            // the rest of this node sequence and let the enclosing loop react.
            if context.loop_signal.is_some() {
                break;
            }
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

            AstNode::Unless { condition, body } => {
                // Render body when the condition is falsy (inverse of @if).
                if self.evaluate_expr(condition, context)? {
                    Ok(String::new())
                } else {
                    self.compile(body, context)
                }
            }

            AstNode::Isset { target, body } => {
                // "set" = present AND not null.
                if self.expr_is_set(target, context) {
                    self.compile(body, context)
                } else {
                    Ok(String::new())
                }
            }

            AstNode::Empty { target, body } => {
                // "empty" = null/false/0/""/[]/missing.
                if self.expr_is_empty(target, context) {
                    self.compile(body, context)
                } else {
                    Ok(String::new())
                }
            }

            AstNode::Switch {
                subject,
                cases,
                default,
            } => self.compile_switch(subject, cases, default, context),

            AstNode::Forelse {
                collection,
                item_var,
                key_var,
                body,
                empty,
            } => self.compile_forelse(
                collection,
                item_var,
                key_var.as_deref(),
                body,
                empty,
                context,
            ),

            AstNode::Break { condition } => {
                let act = match condition {
                    Some(cond) => self.evaluate_expr(cond, context)?,
                    None => true,
                };
                if act {
                    context.loop_signal = Some(LoopSignal::Break);
                }
                Ok(String::new())
            }

            AstNode::Continue { condition } => {
                let act = match condition {
                    Some(cond) => self.evaluate_expr(cond, context)?,
                    None => true,
                };
                if act {
                    context.loop_signal = Some(LoopSignal::Continue);
                }
                Ok(String::new())
            }

            AstNode::Once { body } => {
                // Render the body. Full request-scoped @once deduplication
                // (rendering only once across an entire request) needs the
                // higher-level engine and is out of scope for the string
                // compiler; here we simply render the body.
                self.compile(body, context)
            }

            AstNode::Php => {
                // RustForge has no PHP runtime, so @php ... @endphp cannot be
                // executed. Render nothing (and, importantly, no passthrough
                // HTML comment).
                Ok(String::new())
            }

            AstNode::AttributeHelper { word, condition } => {
                // e.g. @checked(cond) -> "checked" when truthy, else "".
                if self.evaluate_expr(condition, context)? {
                    Ok(word.clone())
                } else {
                    Ok(String::new())
                }
            }

            AstNode::ClassList { items } => {
                let classes = self.collect_conditional_entries(items, context, " ")?;
                if classes.is_empty() {
                    Ok(String::new())
                } else {
                    Ok(format!(r#"class="{}""#, classes))
                }
            }

            AstNode::StyleList { items } => {
                let styles = self.collect_conditional_entries(items, context, "; ")?;
                if styles.is_empty() {
                    Ok(String::new())
                } else {
                    Ok(format!(r#"style="{}""#, styles))
                }
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

        // Normalize PHP-style member access (`$loop->index`, `$user->name`) to
        // dot notation so the same resolution path handles both. This is what
        // makes `{{ $loop->index }}` resolve against the injected $loop object.
        let normalized = var_name.replace("->", ".");
        let var_name = normalized.as_str();

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
        self.run_loop(collection, item_var, key_var, body, context)
    }

    /// Shared loop runner for @foreach / @forelse.
    ///
    /// Builds the per-iteration context (item var, optional key var, and the
    /// `$loop` object) and honors `@break` / `@continue` signals. Returns the
    /// number of iterations performed so callers (e.g. @forelse) can detect an
    /// empty collection.
    fn run_loop(
        &self,
        collection: &Expr,
        item_var: &str,
        key_var: Option<&str>,
        body: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        let (output, _count) = self.run_loop_counted(collection, item_var, key_var, body, context)?;
        Ok(output)
    }

    /// Like `run_loop` but also reports how many items were iterated.
    fn run_loop_counted(
        &self,
        collection: &Expr,
        item_var: &str,
        key_var: Option<&str>,
        body: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<(String, usize)> {
        let collection_value = self.resolve_expr(collection, context)?;

        // Normalize the collection into an ordered list of (key, value) pairs.
        let entries: Vec<(Value, Value)> = match collection_value {
            Value::Array(items) => items
                .into_iter()
                .enumerate()
                .map(|(i, v)| (Value::Number(serde_json::Number::from(i as i64)), v))
                .collect(),
            Value::Object(map) => map
                .into_iter()
                .map(|(k, v)| (Value::String(k), v))
                .collect(),
            _ => Vec::new(),
        };

        let count = entries.len();
        let mut output = String::new();

        for (index, (key, value)) in entries.into_iter().enumerate() {
            // Build the per-iteration data object.
            let mut new_map = match &context.data {
                Value::Object(map) => map.clone(),
                _ => serde_json::Map::new(),
            };

            new_map.insert(item_var.to_string(), value);

            if let Some(key_name) = key_var {
                new_map.insert(key_name.to_string(), key);
            }

            // Expose $loop for the current iteration.
            new_map.insert("loop".to_string(), build_loop_object(index, count));

            let mut loop_context = context.clone();
            loop_context.data = Value::Object(new_map);
            loop_context.loop_signal = None;

            let html = self.compile(body, &mut loop_context)?;
            output.push_str(&html);

            // React to loop-control signals raised inside the body.
            match loop_context.loop_signal {
                Some(LoopSignal::Break) => break,
                Some(LoopSignal::Continue) | None => {}
            }
        }

        Ok((output, count))
    }

    /// Compile @forelse directive
    fn compile_forelse(
        &self,
        collection: &Expr,
        item_var: &str,
        key_var: Option<&str>,
        body: &[AstNode],
        empty: &[AstNode],
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        let (output, count) =
            self.run_loop_counted(collection, item_var, key_var, body, context)?;

        if count == 0 {
            self.compile(empty, context)
        } else {
            Ok(output)
        }
    }

    /// Compile @switch directive
    fn compile_switch(
        &self,
        subject: &Expr,
        cases: &[(Expr, Vec<AstNode>)],
        default: &Option<Vec<AstNode>>,
        context: &mut RenderContext,
    ) -> CompileResult<String> {
        let subject_value = self.resolve_expr(subject, context)?;

        for (case_expr, body) in cases {
            let case_value = self.resolve_expr(case_expr, context)?;
            if values_loosely_equal(&subject_value, &case_value) {
                return self.compile(body, context);
            }
        }

        if let Some(default_body) = default {
            self.compile(default_body, context)
        } else {
            Ok(String::new())
        }
    }

    /// Collect the truthy entries of a @class / @style array into a joined
    /// string (joined by `separator`).
    fn collect_conditional_entries(
        &self,
        items: &[ConditionalEntry],
        context: &RenderContext,
        separator: &str,
    ) -> CompileResult<String> {
        let mut parts: Vec<String> = Vec::new();
        for entry in items {
            match entry {
                ConditionalEntry::Always(value) => {
                    if !value.is_empty() {
                        parts.push(value.clone());
                    }
                }
                ConditionalEntry::Conditional { value, condition } => {
                    if self.evaluate_expr(condition, context)? {
                        parts.push(value.clone());
                    }
                }
            }
        }
        Ok(parts.join(separator))
    }

    /// Whether an expression resolves to a "set" value (present and non-null).
    fn expr_is_set(&self, expr: &Expr, context: &RenderContext) -> bool {
        match self.lookup_optional(expr, context) {
            Some(Value::Null) | None => false,
            Some(_) => true,
        }
    }

    /// Whether an expression resolves to an "empty" value
    /// (null/false/0/0.0/""/[]/{}/missing).
    fn expr_is_empty(&self, expr: &Expr, context: &RenderContext) -> bool {
        match self.lookup_optional(expr, context) {
            None => true,
            Some(v) => !self.is_truthy(&v),
        }
    }

    /// Resolve an expression but distinguish "missing" (None) from a present
    /// null value (Some(Null)) for @isset / @empty semantics.
    fn lookup_optional(&self, expr: &Expr, context: &RenderContext) -> Option<Value> {
        match expr {
            Expr::Variable(var) => context.get_var(var).cloned(),
            Expr::MemberAccess { .. } | Expr::ArrayAccess { .. } => {
                // resolve_expr returns Null for missing chained access; treat a
                // resolved Null the same as missing for these forms.
                match self.resolve_expr(expr, context) {
                    Ok(Value::Null) => None,
                    Ok(v) => Some(v),
                    Err(_) => None,
                }
            }
            Expr::Raw(s) => context.get_var(s).cloned(),
            other => self.resolve_expr(other, context).ok(),
        }
    }

    /// Compile @for directive
    fn compile_for(
        &self,
        init: &str,
        condition: &str,
        increment: &str,
        _body: &[AstNode],
        _context: &mut RenderContext,
    ) -> CompileResult<String> {
        // Simplified @for implementation
        // For full implementation, would need to parse and execute PHP-like expressions
        Ok(format!(
            "<!-- for loop: {}; {}; {} -->",
            init, condition, increment
        ))
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

            context.loop_signal = None;
            let html = self.compile(body, context)?;
            output.push_str(&html);
            iterations += 1;

            // Honor @break / @continue raised in the body.
            match context.loop_signal.take() {
                Some(LoopSignal::Break) => break,
                Some(LoopSignal::Continue) | None => {}
            }
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
                // Raw expressions (e.g. "x == 3", "loop->first") are not pre-parsed
                // into BinaryOp nodes by Expr::parse, so evaluate them here. This
                // is shared by @if, @break(cond), @continue(cond), etc.
                self.evaluate_raw(s, context)
            }

            _ => {
                let value = self.resolve_expr(expr, context)?;
                Ok(self.is_truthy(&value))
            }
        }
    }

    /// Evaluate a raw (un-pre-parsed) expression string to a boolean.
    ///
    /// Supports a single top-level logical (`&&`/`and`/`||`/`or`) or comparison
    /// (`==`, `!=`, `<=`, `>=`, `<`, `>`) operator with literal/variable
    /// operands, plus a leading `!` negation and bare truthiness. Operands may
    /// use `->` member access (e.g. `loop->first`).
    fn evaluate_raw(&self, s: &str, context: &RenderContext) -> CompileResult<bool> {
        let s = s.trim();

        // Logical OR (lowest precedence).
        if let Some((l, r)) = split_top_level_op(s, &["||", " or "]) {
            return Ok(self.evaluate_raw(&l, context)? || self.evaluate_raw(&r, context)?);
        }
        // Logical AND.
        if let Some((l, r)) = split_top_level_op(s, &["&&", " and "]) {
            return Ok(self.evaluate_raw(&l, context)? && self.evaluate_raw(&r, context)?);
        }

        // Comparisons (order matters: two-char operators before single-char).
        for op in ["==", "!=", "<=", ">="] {
            if let Some((l, r)) = split_top_level_op(s, &[op]) {
                let lv = self.resolve_raw_operand(&l, context);
                let rv = self.resolve_raw_operand(&r, context);
                return Ok(compare_values(&lv, op, &rv));
            }
        }
        for op in ["<", ">"] {
            if let Some((l, r)) = split_top_level_op(s, &[op]) {
                let lv = self.resolve_raw_operand(&l, context);
                let rv = self.resolve_raw_operand(&r, context);
                return Ok(compare_values(&lv, op, &rv));
            }
        }

        // Leading negation.
        if let Some(rest) = s.strip_prefix('!') {
            return Ok(!self.evaluate_raw(rest, context)?);
        }

        // Bare value: truthiness.
        Ok(self.is_truthy(&self.resolve_raw_operand(s, context)))
    }

    /// Resolve a raw operand (literal or variable/member-access) to a Value.
    fn resolve_raw_operand(&self, s: &str, context: &RenderContext) -> Value {
        let s = s.trim();
        let s = s.strip_prefix('$').unwrap_or(s);

        // String literal.
        if s.len() >= 2
            && ((s.starts_with('\'') && s.ends_with('\''))
                || (s.starts_with('"') && s.ends_with('"')))
        {
            return Value::String(s[1..s.len() - 1].to_string());
        }
        // Boolean / null literals.
        match s {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            "null" => return Value::Null,
            _ => {}
        }
        // Numeric literal.
        if let Ok(n) = s.parse::<f64>() {
            return serde_json::Number::from_f64(n)
                .map(Value::Number)
                .unwrap_or(Value::Null);
        }
        // Variable / member access.
        context.get_var(s).cloned().unwrap_or(Value::Null)
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

/// Split `s` on the first top-level occurrence of any of `ops` (ignoring
/// matches inside quotes or parentheses, and not splitting on `->`).
/// Returns (left, right) trimmed.
fn split_top_level_op(s: &str, ops: &[&str]) -> Option<(String, String)> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = b' ';
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\'' | b'"' => {
                if in_string && b == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = b;
                }
                i += 1;
                continue;
            }
            b'(' | b'[' if !in_string => depth += 1,
            b')' | b']' if !in_string => depth -= 1,
            _ => {}
        }

        if !in_string && depth == 0 {
            // Don't treat the `-` / `>` of a `->` accessor as an operator.
            if b == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                i += 2;
                continue;
            }
            for op in ops {
                if s[i..].starts_with(op) {
                    let left = s[..i].trim().to_string();
                    let right = s[i + op.len()..].trim().to_string();
                    if !left.is_empty() && !right.is_empty() {
                        return Some((left, right));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Compare two values with a comparison operator string (PHP-ish loose rules).
fn compare_values(left: &Value, op: &str, right: &Value) -> bool {
    match op {
        "==" => values_loosely_equal(left, right),
        "!=" => !values_loosely_equal(left, right),
        "<" | "<=" | ">" | ">=" => {
            if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                match op {
                    "<" => l < r,
                    "<=" => l <= r,
                    ">" => l > r,
                    ">=" => l >= r,
                    _ => false,
                }
            } else if let (Value::String(l), Value::String(r)) = (left, right) {
                match op {
                    "<" => l < r,
                    "<=" => l <= r,
                    ">" => l > r,
                    ">=" => l >= r,
                    _ => false,
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Build the `$loop` object exposed inside `@foreach` / `@forelse` for the
/// iteration at `index` (0-based) within a collection of `count` items.
///
/// Mirrors Laravel's loop variable: `index` (0-based), `iteration` (1-based),
/// `first`, `last`, `count`, `remaining`, `index0`, `even`, `odd`.
fn build_loop_object(index: usize, count: usize) -> Value {
    let iteration = index + 1;
    let remaining = count.saturating_sub(iteration);
    let mut map = serde_json::Map::new();
    map.insert("index".to_string(), Value::from(index as i64));
    map.insert("index0".to_string(), Value::from(index as i64));
    map.insert("iteration".to_string(), Value::from(iteration as i64));
    map.insert("count".to_string(), Value::from(count as i64));
    map.insert("remaining".to_string(), Value::from(remaining as i64));
    map.insert("first".to_string(), Value::Bool(index == 0));
    map.insert("last".to_string(), Value::Bool(iteration == count));
    map.insert("even".to_string(), Value::Bool(iteration.is_multiple_of(2)));
    map.insert("odd".to_string(), Value::Bool(iteration % 2 == 1));
    Value::Object(map)
}

/// Loose equality for `@switch` case matching (mirrors PHP's `==` for the
/// common scalar cases): numbers compare numerically, and a number compares
/// equal to a numeric string.
fn values_loosely_equal(a: &Value, b: &Value) -> bool {
    if a == b {
        return true;
    }
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => {
            a.as_f64() == b.as_f64()
        }
        (Value::Number(_), Value::String(s)) | (Value::String(s), Value::Number(_)) => {
            let n = if matches!(a, Value::Number(_)) {
                a.as_f64()
            } else {
                b.as_f64()
            };
            s.trim().parse::<f64>().ok() == n
        }
        _ => false,
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
