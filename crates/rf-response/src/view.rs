//! Real `view(name, data)` template response.
//!
//! Renders an HTML template from the conventional views directory
//! (`resources/views/<name>.blade.html`, or `.html`) by interpolating
//! `{{ var }}` placeholders from a [`serde::Serialize`] data value, and returns
//! it as a `text/html` HTTP response.
//!
//! This is a deliberately small but *real* renderer: it reads the actual
//! template file off disk and substitutes values from the supplied data. It is
//! NOT a full Blade engine — see [`crate::view::ViewResponse`] docs for what a
//! full engine (directives, layouts, components, loops) would add. For that,
//! the `rf-view` crate ships a Tera-backed engine (`rf_view::View`) that
//! requires a global glob-based init.
//!
//! # Example (handler)
//!
//! ```no_run
//! use rf_response::{view, Response};
//! use axum::response::IntoResponse;
//!
//! async fn home() -> impl IntoResponse {
//!     // renders resources/views/home.blade.html with {{ title }} filled in
//!     view("home", serde_json::json!({ "title": "Welcome" }))
//! }
//!
//! async fn home2() -> impl IntoResponse {
//!     Response::view("home", serde_json::json!({ "title": "Welcome" }))
//! }
//! ```

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

/// Guard against runaway `@include` / `@extends` recursion.
const MAX_DEPTH: usize = 32;

/// A rendered (or to-be-rendered) template response.
///
/// Rendering happens lazily in [`IntoResponse::into_response`] so the helper can
/// be returned directly from a handler. On success it yields a `200 OK` with a
/// `text/html; charset=utf-8` body; a missing template or unreadable file yields
/// a `500` carrying the real error (never fabricated HTML).
pub struct ViewResponse {
    name: String,
    data: Value,
}

impl ViewResponse {
    /// Build a view response for `name`, carrying `data` used to interpolate
    /// `{{ var }}` placeholders.
    pub fn new<T: Serialize>(name: impl Into<String>, data: T) -> Self {
        let data = serde_json::to_value(data).unwrap_or(Value::Null);
        Self {
            name: name.into(),
            data,
        }
    }

    /// Root directory templates are resolved against.
    ///
    /// Defaults to `resources/views` (Laravel-style), overridable via the
    /// `RUSTFORGE_VIEWS_PATH` environment variable.
    fn views_root() -> PathBuf {
        std::env::var_os("RUSTFORGE_VIEWS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("resources/views"))
    }

    /// Resolve a dotted/plain view name to an existing template file.
    ///
    /// `"users.index"` -> `resources/views/users/index.blade.html`
    /// (also tries `.html`, and the name verbatim if it already has an extension).
    fn resolve_path(name: &str) -> Option<PathBuf> {
        let root = Self::views_root();
        let mut candidates: Vec<PathBuf> = Vec::new();

        // If the caller already passed a concrete file (has .html), honour it.
        if name.ends_with(".html") {
            candidates.push(root.join(name));
        } else {
            let slashed = name.replace('.', "/");
            candidates.push(root.join(format!("{slashed}.blade.html")));
            candidates.push(root.join(format!("{slashed}.html")));
        }

        candidates.into_iter().find(|p| p.is_file())
    }

    /// Render the template to an HTML string, or return a human-readable error.
    ///
    /// Supports `{{ var }}` interpolation plus Blade-style control flow and
    /// layout inheritance — see the module-level [`render_document`] for the full
    /// directive set (`@if`/`@foreach`/`@extends`/`@section`/`@yield`/`@include`).
    pub fn render(&self) -> Result<String, String> {
        // Prefer the real rf-view Tera engine when it has been initialized AND
        // it actually knows this template: full expression evaluation, filters,
        // and richer control flow. Tera rendering is synchronous, so this needs
        // no async bridge and cannot deadlock a runtime.
        if let Some(result) = render_via_tera(&self.name, &self.data) {
            return result;
        }
        // Fall back to the built-in file renderer so zero-config (no
        // `ViewEngine::init`) keeps working exactly as before.
        let template = load_template(&self.name)?;
        render_document(&template, &self.data, 0)
    }
}

/// Try to render `name` via the initialized `rf_view` Tera engine.
///
/// Returns:
///   * `None` — the `"view"` feature is disabled, the engine is not
///     initialized, or it holds no template matching `name`; the caller should
///     use the built-in file renderer.
///   * `Some(Ok(html))` — Tera rendered the template.
///   * `Some(Err(msg))` — the engine owns the template but rendering failed;
///     the real Tera error is surfaced (never fabricated), and we do NOT
///     silently fall back to a different file.
///
/// Only compiled when the `"view"` feature is enabled (i.e. the caller
/// depends on `rf-response` with `features = ["view"]`).
#[cfg(feature = "view")]
fn render_via_tera(name: &str, data: &Value) -> Option<Result<String, String>> {
    use rf_view::{Context, ViewEngine};

    // `template_names()` returns an error only when the engine is uninitialized;
    // an initialized-but-empty engine returns `Ok(vec![])`. Use it as the
    // initialization probe so we don't touch engine internals.
    if ViewEngine::template_names().is_err() {
        return None;
    }

    let tpl = tera_template_name(name)?;

    let mut ctx = Context::new();
    if let Value::Object(map) = data {
        for (k, v) in map {
            ctx.insert(k, v);
        }
    }

    Some(ViewEngine::render(&tpl, &ctx).map_err(|e| e.to_string()))
}

/// Stub used when the `"view"` Cargo feature is disabled.
///
/// Always returns `None` so `ViewResponse::render` falls through to the
/// built-in file renderer without touching the (absent) rf-view crate.
#[cfg(not(feature = "view"))]
#[inline(always)]
fn render_via_tera(_name: &str, _data: &Value) -> Option<Result<String, String>> {
    None
}

/// Resolve a dotted/plain view name to a template registered with the Tera
/// engine, mirroring `rf_view`'s naming (dots → slashes, `.tera` extension).
///
/// Only compiled when the `"view"` feature is enabled.
#[cfg(feature = "view")]
fn tera_template_name(name: &str) -> Option<String> {
    use rf_view::ViewEngine;

    let slashed = name.replace('.', "/");
    let candidates = [
        name.to_string(),
        format!("{name}.tera"),
        format!("{slashed}.tera"),
        format!("{slashed}.html"),
    ];
    candidates
        .into_iter()
        .find(|c| ViewEngine::has_template(c).unwrap_or(false))
}

/// Load a template file off disk by view name (dotted or plain), returning its
/// raw contents or a human-readable error (matching `ViewResponse::render`).
fn load_template(name: &str) -> Result<String, String> {
    let path = ViewResponse::resolve_path(name).ok_or_else(|| {
        format!(
            "view '{}' not found under {}",
            name,
            ViewResponse::views_root().display()
        )
    })?;
    std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read view '{}': {e}", path.display()))
}

impl IntoResponse for ViewResponse {
    fn into_response(self) -> AxumResponse {
        match self.render() {
            Ok(html) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                html,
            )
                .into_response(),
            Err(e) => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    e,
                )
                    .into_response()
            }
        }
    }
}

/// Length in bytes of a UTF-8 sequence given its leading byte.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Resolve a dotted path against a JSON value, producing a display string.
fn lookup(data: &Value, path: &str) -> String {
    let mut cur = data;
    for seg in path.split('.') {
        cur = match cur {
            Value::Object(map) => match map.get(seg) {
                Some(v) => v,
                None => return String::new(),
            },
            _ => return String::new(),
        };
    }
    match cur {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Minimal HTML entity escaping (matches Blade's escaped `{{ }}` output).
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Blade-style directive engine
// ---------------------------------------------------------------------------
//
// A small but *real* renderer. Given a template string and `serde_json` data it
// evaluates:
//
//   * `{{ path }}`                      — HTML-escaped interpolation (nested lookup)
//   * `@if(cond) .. @elseif(cond) .. @else .. @endif`
//   * `@foreach(item in list) .. @endforeach`  (iterate a real JSON array)
//   * `@include("partial")`            — render another template with the same data
//   * `@extends("layout")` + `@section("name") .. @endsection` + `@yield("name")`
//                                        — child sections rendered into the parent
//
// Condition support is a documented subset (see [`eval_condition`]): path
// truthiness, `!path` negation, and `==` / `!=` against a string/number/bool
// literal or another path. Arbitrary arithmetic / boolean-operator expressions
// (`&&`, `||`, `<`, `>`, function calls) are intentionally NOT supported.

/// Render a full template document.
///
/// If the template opens with `@extends("layout")`, its `@section` blocks are
/// collected and the *parent* layout is rendered, with `@yield("name")` slots
/// filled by the corresponding rendered child section. Otherwise the template
/// body is rendered directly.
fn render_document(tpl: &str, data: &Value, depth: usize) -> Result<String, String> {
    if depth > MAX_DEPTH {
        return Err("view rendering exceeded max include/extends depth".to_string());
    }
    if let Some(layout) = extract_extends(tpl) {
        let sections = collect_sections(tpl);
        let layout_tpl = load_template(&layout)?;
        render_block(&layout_tpl, data, &sections, depth + 1)
    } else {
        render_block(tpl, data, &HashMap::new(), depth)
    }
}

/// Render a template fragment: interpolation + control-flow directives.
///
/// `sections` carries the (raw, unrendered) child sections for `@yield`
/// resolution; it is empty when not rendering a layout.
fn render_block(
    tpl: &str,
    data: &Value,
    sections: &HashMap<String, String>,
    depth: usize,
) -> Result<String, String> {
    if depth > MAX_DEPTH {
        return Err("view rendering exceeded max include/extends depth".to_string());
    }
    let mut out = String::with_capacity(tpl.len());
    let bytes = tpl.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // {{ path }} interpolation.
        if starts(tpl, i, "{{") {
            if let Some(close) = tpl[i + 2..].find("}}") {
                let expr = tpl[i + 2..i + 2 + close].trim();
                out.push_str(&html_escape(&lookup(data, expr)));
                i = i + 2 + close + 2;
                continue;
            }
        }

        if bytes[i] == b'@' {
            // @if( ... ) ... @endif
            if starts(tpl, i, "@if(") {
                let (cond, after) = read_paren(tpl, i + 3)
                    .ok_or_else(|| "unterminated @if(".to_string())?;
                let end = find_matching(tpl, after, "@if(", "@endif")
                    .ok_or_else(|| "missing @endif".to_string())?;
                let body = &tpl[after..end];
                let branch = choose_branch(cond, body, data);
                if let Some(chosen) = branch {
                    out.push_str(&render_block(chosen, data, sections, depth + 1)?);
                }
                i = end + "@endif".len();
                continue;
            }
            // @foreach(item in list) ... @endforeach
            if starts(tpl, i, "@foreach(") {
                let (spec, after) = read_paren(tpl, i + 8)
                    .ok_or_else(|| "unterminated @foreach(".to_string())?;
                let end = find_matching(tpl, after, "@foreach(", "@endforeach")
                    .ok_or_else(|| "missing @endforeach".to_string())?;
                let body = &tpl[after..end];
                let (var, list_path) = parse_foreach(&spec)
                    .ok_or_else(|| format!("invalid @foreach spec: {spec}"))?;
                if let Some(Value::Array(items)) = resolve(data, &list_path) {
                    for (idx, item) in items.iter().enumerate() {
                        let scope = make_scope(data, &var, item, idx, items.len());
                        out.push_str(&render_block(body, &scope, sections, depth + 1)?);
                    }
                }
                i = end + "@endforeach".len();
                continue;
            }
            // @include("partial") — render another template with the same data.
            if starts(tpl, i, "@include(") {
                let (arg, after) = read_paren(tpl, i + 8)
                    .ok_or_else(|| "unterminated @include(".to_string())?;
                let name = unquote(first_arg(&arg));
                let partial = load_template(&name)?;
                out.push_str(&render_document(&partial, data, depth + 1)?);
                i = after;
                continue;
            }
            // @yield("name") — insert the rendered child section (or nothing).
            if starts(tpl, i, "@yield(") {
                let (arg, after) = read_paren(tpl, i + 6)
                    .ok_or_else(|| "unterminated @yield(".to_string())?;
                let name = unquote(first_arg(&arg));
                if let Some(body) = sections.get(&name) {
                    out.push_str(&render_block(body, data, sections, depth + 1)?);
                }
                i = after;
                continue;
            }
            // @section(...) .. @endsection encountered outside a layout: render
            // the inner body inline (block form); drop inline `@section('a','b')`.
            if starts(tpl, i, "@section(") {
                let (arg, after) = read_paren(tpl, i + 8)
                    .ok_or_else(|| "unterminated @section(".to_string())?;
                if split_top_comma(&arg).is_some() {
                    // inline form `@section('name', 'value')` — no layout to
                    // receive it here; skip.
                    i = after;
                    continue;
                }
                if let Some(end) = find_after(tpl, after, "@endsection") {
                    let body = &tpl[after..end];
                    out.push_str(&render_block(body, data, sections, depth + 1)?);
                    i = end + "@endsection".len();
                    continue;
                }
            }
        }

        // Not a recognised directive/placeholder: copy one UTF-8 char.
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&tpl[i..i + ch_len]);
        i += ch_len;
    }
    Ok(out)
}

/// True if `tpl` has `needle` starting at byte index `i`.
fn starts(tpl: &str, i: usize, needle: &str) -> bool {
    tpl.as_bytes()[i..].starts_with(needle.as_bytes())
}

/// Given `tpl` with `tpl[open]` == '(', return the inner text and the index
/// just past the matching ')'. Respects nested parens and quoted strings.
fn read_paren(tpl: &str, open: usize) -> Option<(String, usize)> {
    let bytes = tpl.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = open;
    let mut in_str: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            }
        } else {
            match c {
                b'"' | b'\'' => in_str = Some(c),
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((tpl[open + 1..i].to_string(), i + 1));
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Find the index where the `close` keyword that matches an already-open `open`
/// directive begins, accounting for nested `open`/`close` pairs.
fn find_matching(tpl: &str, from: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut i = from;
    let bytes = tpl.as_bytes();
    while i < bytes.len() {
        if starts(tpl, i, open) {
            depth += 1;
            i += open.len();
        } else if starts(tpl, i, close) {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += close.len();
        } else {
            i += utf8_len(bytes[i]);
        }
    }
    None
}

/// Find the next occurrence of `kw` at or after `from` (no nesting).
fn find_after(tpl: &str, from: usize, kw: &str) -> Option<usize> {
    tpl[from..].find(kw).map(|p| from + p)
}

/// Locate `@extends("layout")` and return the layout name, if present.
fn extract_extends(tpl: &str) -> Option<String> {
    let p = tpl.find("@extends(")?;
    let (arg, _) = read_paren(tpl, p + 8)?;
    Some(unquote(first_arg(&arg)))
}

/// Collect `@section("name") .. @endsection` block bodies (raw, unrendered).
fn collect_sections(tpl: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut i = 0;
    while let Some(rel) = tpl[i..].find("@section(") {
        let at = i + rel;
        let open = at + 8;
        let (arg, after) = match read_paren(tpl, open) {
            Some(v) => v,
            None => break,
        };
        // Inline form: `@section('name', 'value')`.
        if let Some((name_part, value_part)) = split_top_comma(&arg) {
            map.insert(unquote(name_part.trim()), unquote(value_part.trim()));
            i = after;
            continue;
        }
        let name = unquote(arg.trim());
        match find_after(tpl, after, "@endsection") {
            Some(end) => {
                map.insert(name, tpl[after..end].to_string());
                i = end + "@endsection".len();
            }
            None => break,
        }
    }
    map
}

/// The first comma-separated argument (top level) of a directive arg string.
fn first_arg(arg: &str) -> &str {
    match split_top_comma(arg) {
        Some((a, _)) => a,
        None => arg,
    }
}

/// Split `arg` at the first top-level comma (outside quotes), if any.
fn split_top_comma(arg: &str) -> Option<(&str, &str)> {
    let bytes = arg.as_bytes();
    let mut in_str: Option<u8> = None;
    for (i, &c) in bytes.iter().enumerate() {
        match in_str {
            Some(q) if c == q => in_str = None,
            Some(_) => {}
            None => match c {
                b'"' | b'\'' => in_str = Some(c),
                b',' => return Some((&arg[..i], &arg[i + 1..])),
                _ => {}
            },
        }
    }
    None
}

/// Strip a single pair of matching surrounding quotes, else trim.
fn unquote(s: &str) -> String {
    let t = s.trim();
    let b = t.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Parse a `@foreach(var in list.path)` spec into `(var, list_path)`.
fn parse_foreach(spec: &str) -> Option<(String, String)> {
    let (var, list) = spec.split_once(" in ")?;
    let var = var.trim();
    let list = list.trim();
    if var.is_empty() || list.is_empty() {
        return None;
    }
    Some((var.to_string(), list.to_string()))
}

/// Build a child scope for a `@foreach` iteration: the current data object
/// overlaid with the loop variable (and a small `loop` metadata object).
fn make_scope(data: &Value, var: &str, elem: &Value, index: usize, total: usize) -> Value {
    let mut map = match data {
        Value::Object(m) => m.clone(),
        _ => Map::new(),
    };
    map.insert(var.to_string(), elem.clone());
    let mut loopvar = Map::new();
    loopvar.insert("index".into(), Value::from(index));
    loopvar.insert("iteration".into(), Value::from(index + 1));
    loopvar.insert("first".into(), Value::from(index == 0));
    loopvar.insert("last".into(), Value::from(index + 1 == total));
    loopvar.insert("count".into(), Value::from(total));
    map.insert("loop".to_string(), Value::Object(loopvar));
    Value::Object(map)
}

/// Split an `@if` body into branches and return the first whose condition holds
/// (or the `@else` branch), rendered by the caller. `cond0` is the `@if` head.
fn choose_branch<'a>(cond0: String, body: &'a str, data: &Value) -> Option<&'a str> {
    let branches = split_branches(&cond0, body);
    for (cond, text) in branches {
        match cond {
            None => return Some(text), // @else
            Some(c) if eval_condition(&c, data) => return Some(text),
            _ => {}
        }
    }
    None
}

/// Split an `@if` body into `(Some(cond) | None@else, text)` branches at the
/// top level (ignoring nested `@if .. @endif`).
fn split_branches<'a>(cond0: &str, body: &'a str) -> Vec<(Option<String>, &'a str)> {
    let mut branches: Vec<(Option<String>, &str)> = Vec::new();
    let mut cur_cond: Option<String> = Some(cond0.to_string());
    let mut seg_start = 0usize;
    let mut depth = 0usize;
    let mut i = 0usize;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        if starts(body, i, "@if(") {
            depth += 1;
            i += 4;
        } else if starts(body, i, "@endif") {
            depth = depth.saturating_sub(1);
            i += 6;
        } else if depth == 0 && starts(body, i, "@elseif(") {
            branches.push((cur_cond.take(), &body[seg_start..i]));
            if let Some((cond, after)) = read_paren(body, i + 7) {
                cur_cond = Some(cond);
                seg_start = after;
                i = after;
            } else {
                i += 8;
            }
        } else if depth == 0 && starts(body, i, "@else") && !starts(body, i, "@elseif") {
            branches.push((cur_cond.take(), &body[seg_start..i]));
            cur_cond = None;
            i += 5;
            seg_start = i;
        } else {
            i += utf8_len(bytes[i]);
        }
    }
    branches.push((cur_cond, &body[seg_start..]));
    branches
}

/// Evaluate a (subset) condition against `data`.
///
/// Supported: `!expr` negation; `lhs == rhs` / `lhs != rhs` where each side is a
/// data path, a quoted string, a number, or `true`/`false`/`null`; and bare
/// `path` truthiness. Unsupported operators (`&&`, `||`, `<`, `>`, arithmetic,
/// calls) are NOT parsed — such a condition falls back to path truthiness of the
/// whole string (typically `false`).
fn eval_condition(expr: &str, data: &Value) -> bool {
    let e = expr.trim();
    if let Some(rest) = e.strip_prefix('!') {
        return !eval_condition(rest, data);
    }
    if let Some((lhs, rhs)) = e.split_once("==") {
        return values_eq(&operand(lhs, data), &operand(rhs, data));
    }
    if let Some((lhs, rhs)) = e.split_once("!=") {
        return !values_eq(&operand(lhs, data), &operand(rhs, data));
    }
    match resolve(data, e) {
        Some(v) => truthy(v),
        None => false,
    }
}

/// Resolve a comparison operand: a literal (quoted string / number / bool /
/// null) or a data path.
fn operand(s: &str, data: &Value) -> Value {
    let t = s.trim();
    let b = t.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        return Value::String(t[1..t.len() - 1].to_string());
    }
    match t {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        "null" => return Value::Null,
        _ => {}
    }
    if let Ok(n) = t.parse::<i64>() {
        return Value::from(n);
    }
    if let Ok(n) = t.parse::<f64>() {
        return Value::from(n);
    }
    resolve(data, t).cloned().unwrap_or(Value::Null)
}

/// Equality with light numeric coercion (int vs float).
fn values_eq(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// JSON truthiness (Blade-like): null/false/empty-string/zero/empty
/// array/empty object are falsey.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Resolve a dotted path to a borrowed JSON value (objects only for nesting).
fn resolve<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = data;
    for seg in path.trim().split('.') {
        cur = cur.as_object()?.get(seg)?;
    }
    Some(cur)
}

/// Global `view(name, data)` helper.
///
/// Mirrors [`Response::view`](crate::Response::view) as a bare function so
/// handlers can write `view("home", data)` directly. Returns a [`ViewResponse`]
/// which implements [`axum::response::IntoResponse`].
pub fn view<T: Serialize>(name: impl Into<String>, data: T) -> ViewResponse {
    ViewResponse::new(name, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn render(tpl: &str, data: Value) -> String {
        render_block(tpl, &data, &HashMap::new(), 0).unwrap()
    }

    #[test]
    fn interpolates_simple_and_nested() {
        let tpl = "<h1>{{ title }}</h1><p>{{ user.name }}</p>";
        let out = render(tpl, json!({"title": "Hi", "user": {"name": "Ada"}}));
        assert_eq!(out, "<h1>Hi</h1><p>Ada</p>");
    }

    #[test]
    fn escapes_html_and_handles_missing() {
        let tpl = "{{ danger }}|{{ missing }}|{{ n }}";
        let out = render(tpl, json!({"danger": "<b>&\"", "n": 42}));
        assert_eq!(out, "&lt;b&gt;&amp;&quot;||42");
    }

    #[test]
    fn preserves_non_placeholder_braces() {
        let tpl = "a { b } {c} {{x}}";
        let out = render(tpl, json!({"x": "X"}));
        assert_eq!(out, "a { b } {c} X");
    }

    #[test]
    fn if_truthiness_and_else() {
        let tpl = "@if(admin)YES@elseif(guest)GUEST@elseHIDDEN@endif";
        assert_eq!(render(tpl, json!({"admin": true})), "YES");
        assert_eq!(render(tpl, json!({"admin": false, "guest": true})), "GUEST");
        assert_eq!(render(tpl, json!({"admin": false})), "HIDDEN");
        // empty string / zero / missing are falsey
        assert_eq!(render("@if(x)Y@endif", json!({"x": ""})), "");
        assert_eq!(render("@if(x)Y@endif", json!({"x": 0})), "");
        assert_eq!(render("@if(x)Y@endif", json!({})), "");
    }

    #[test]
    fn if_equality_literals_and_negation() {
        assert_eq!(
            render("@if(status == \"active\")ON@endif", json!({"status": "active"})),
            "ON"
        );
        assert_eq!(
            render("@if(status == \"active\")ON@endif", json!({"status": "off"})),
            ""
        );
        assert_eq!(render("@if(count == 3)T@endif", json!({"count": 3})), "T");
        assert_eq!(render("@if(count != 3)T@endif", json!({"count": 4})), "T");
        assert_eq!(render("@if(!disabled)EN@endif", json!({"disabled": false})), "EN");
    }

    #[test]
    fn foreach_binds_scope_and_loop_meta() {
        let tpl = "<ul>@foreach(post in posts)<li>{{ post.title }}#{{ loop.iteration }}</li>@endforeach</ul>";
        let out = render(
            tpl,
            json!({"posts": [{"title": "A"}, {"title": "B"}, {"title": "C"}]}),
        );
        assert_eq!(out, "<ul><li>A#1</li><li>B#2</li><li>C#3</li></ul>");
        // empty / missing array produces nothing
        assert_eq!(render(tpl, json!({"posts": []})), "<ul></ul>");
        assert_eq!(render(tpl, json!({})), "<ul></ul>");
    }

    #[test]
    fn nested_if_inside_foreach() {
        let tpl = "@foreach(u in users){{ u.name }}@if(u.admin)*@endif;@endforeach";
        let out = render(
            tpl,
            json!({"users": [{"name": "a", "admin": true}, {"name": "b", "admin": false}]}),
        );
        assert_eq!(out, "a*;b;");
    }

    // File-based tests share the process-global `RUSTFORGE_VIEWS_PATH`, so they
    // live in one test to avoid racing under parallel execution.
    #[test]
    fn extends_yield_sections_and_include() {
        let dir = std::env::temp_dir().join(format!("rf_view_unit_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("layout.blade.html"),
            "<html><head><title>@yield(\"title\")</title></head><body>@yield(\"content\")</body></html>",
        )
        .unwrap();
        std::fs::write(
            dir.join("child.blade.html"),
            "@extends(\"layout\")@section(\"title\")Home@endsection@section(\"content\")<h1>{{ heading }}</h1>@foreach(i in items)<p>{{ i }}</p>@endforeach@endsection",
        )
        .unwrap();
        std::fs::write(dir.join("_row.blade.html"), "<td>{{ name }}</td>").unwrap();
        std::fs::write(
            dir.join("page.blade.html"),
            "<table><tr>@include(\"_row\")</tr></table>",
        )
        .unwrap();
        std::env::set_var("RUSTFORGE_VIEWS_PATH", &dir);

        // @extends + @section -> @yield, with control flow inside a section.
        let out = ViewResponse::new("child", json!({"heading": "Hi", "items": ["x", "y"]}))
            .render()
            .unwrap();
        assert_eq!(
            out,
            "<html><head><title>Home</title></head><body><h1>Hi</h1><p>x</p><p>y</p></body></html>"
        );

        // @include renders another template with the same data.
        let out = ViewResponse::new("page", json!({"name": "Ada"}))
            .render()
            .unwrap();
        assert_eq!(out, "<table><tr><td>Ada</td></tr></table>");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
